use std::path::PathBuf;

use crate::ast::*;
use crate::error::{Error, ErrorCode, Result};

pub(crate) fn parse_str(source: &str, file: Option<PathBuf>) -> Result<Document> {
    Parser::new(source, file).parse_document()
}

struct ParsedPath {
    path: SconPath,
    span: Span,
}

struct Parser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    column: usize,
    file: Option<PathBuf>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, file: Option<PathBuf>) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
            file,
        }
    }

    fn parse_document(mut self) -> Result<Document> {
        let start = self.pos;
        self.skip_ws_comments()?;
        let body = if self.peek() == Some(b'{') {
            self.bump();
            let body = self.parse_object_body(Some(b'}'))?;
            self.expect(b'}')?;
            body
        } else if matches!(self.peek(), Some(b'[' | b'-' | b'0'..=b'9')) {
            return Err(self.err(
                ErrorCode::InvalidRootType,
                "SCON document root must be an object",
            ));
        } else {
            self.parse_object_body(None)?
        };
        self.skip_ws_comments()?;
        if !self.is_eof() {
            return Err(self.err(
                ErrorCode::UnexpectedToken,
                "unexpected token after document",
            ));
        }
        Ok(Document {
            body,
            file: self.file,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_object_body(&mut self, terminator: Option<u8>) -> Result<ObjectBody> {
        let start = self.pos;
        let mut body = ObjectBody::default();
        let mut locals_seen = false;
        loop {
            self.skip_ws_comments()?;
            if self.is_eof() || terminator.is_some_and(|t| self.peek() == Some(t)) {
                break;
            }
            if self.peek() == Some(b'.') && self.starts_with(b"...") {
                if locals_seen {
                    return Err(self.err(
                        ErrorCode::InvalidSpread,
                        "object spread must appear before local members",
                    ));
                }
                let member_start = self.pos;
                let loc = self.loc();
                self.consume(b"...");
                self.skip_inline_ws()?;
                let path = self.parse_substitution()?;
                body.spreads.push(ObjectSpread {
                    path: path.path,
                    path_span: path.span,
                    loc,
                    span: Span::new(member_start, self.pos),
                });
            } else {
                locals_seen = true;
                body.members.push(self.parse_local_member()?);
            }

            let saw_newline_after_member = self.skip_ws_comments()?;
            if self.peek() == Some(b',') {
                self.bump();
                self.skip_ws_comments()?;
                if self.peek() == Some(b',') {
                    return Err(
                        self.err(ErrorCode::UnexpectedToken, "consecutive commas are invalid")
                    );
                }
                continue;
            }
            if self.is_eof() || terminator.is_some_and(|t| self.peek() == Some(t)) {
                break;
            }
            if saw_newline_after_member {
                continue;
            }
            return Err(self.err(
                ErrorCode::UnexpectedToken,
                "object members must be separated by newline or comma",
            ));
        }
        body.span = Span::new(start, self.pos);
        Ok(body)
    }

    fn parse_local_member(&mut self) -> Result<LocalMember> {
        let member_start = self.pos;
        if self.peek() == Some(b'i') && self.starts_with_keyword(b"include") {
            let checkpoint = (self.pos, self.line, self.column);
            let loc = self.loc();
            self.consume(b"include");
            if matches!(self.peek(), Some(b' ' | b'\t')) {
                self.skip_inline_ws()?;
                let path_start = self.pos;
                let path = self.parse_string_no_interpolation()?;
                return Ok(LocalMember::Include {
                    path,
                    path_span: Span::new(path_start, self.pos),
                    loc,
                    span: Span::new(member_start, self.pos),
                });
            }
            self.pos = checkpoint.0;
            self.line = checkpoint.1;
            self.column = checkpoint.2;
        }

        let loc = self.loc();
        let path = self.parse_path()?;
        self.skip_inline_ws()?;
        let value = match self.peek() {
            Some(b'=') => {
                self.bump();
                self.skip_inline_ws()?;
                match self.peek() {
                    None | Some(b'\n') => {
                        return Err(self.err(
                            ErrorCode::UnexpectedToken,
                            "field value must be on the same logical line as =",
                        ));
                    }
                    Some(b'\r') if self.peek_n(1) == Some(b'\n') => {
                        return Err(self.err(
                            ErrorCode::UnexpectedToken,
                            "field value must be on the same logical line as =",
                        ));
                    }
                    Some(b'\r') => {
                        return Err(
                            self.err(ErrorCode::InvalidCharacter, "standalone CR is invalid")
                        );
                    }
                    _ => {}
                }
                self.parse_value()?
            }
            Some(b'{') => {
                self.bump();
                let body = self.parse_object_body(Some(b'}'))?;
                self.expect(b'}')?;
                AstValue::Object {
                    body,
                    span: Span::new(path.span.start_byte, self.pos),
                }
            }
            _ => {
                return Err(self.err(
                    ErrorCode::UnexpectedToken,
                    "expected = or object shorthand after path",
                ));
            }
        };
        Ok(LocalMember::Field(Field {
            path: path.path,
            path_span: path.span,
            value,
            loc,
            span: Span::new(member_start, self.pos),
        }))
    }

    fn parse_value(&mut self) -> Result<AstValue> {
        self.skip_inline_ws()?;
        let start = self.pos;
        match self.peek() {
            Some(b'{') => {
                self.bump();
                let body = self.parse_object_body(Some(b'}'))?;
                self.expect(b'}')?;
                Ok(AstValue::Object {
                    body,
                    span: Span::new(start, self.pos),
                })
            }
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(AstValue::String {
                value: self.parse_string_value(true)?,
                span: Span::new(start, self.pos),
            }),
            Some(b'$') if self.peek_n(1) == Some(b'{') => {
                let path = self.parse_substitution()?;
                Ok(AstValue::Substitution {
                    path: path.path,
                    path_span: path.span,
                    span: Span::new(start, self.pos),
                })
            }
            Some(b'-' | b'0'..=b'9') => Ok(AstValue::Number {
                value: self.parse_number()?,
                span: Span::new(start, self.pos),
            }),
            Some(b't') if self.starts_with_keyword(b"true") => {
                self.consume(b"true");
                Ok(AstValue::Bool {
                    value: true,
                    span: Span::new(start, self.pos),
                })
            }
            Some(b'f') if self.starts_with_keyword(b"false") => {
                self.consume(b"false");
                Ok(AstValue::Bool {
                    value: false,
                    span: Span::new(start, self.pos),
                })
            }
            Some(b'n') if self.starts_with_keyword(b"null") => {
                self.consume(b"null");
                Ok(AstValue::Null {
                    span: Span::new(start, self.pos),
                })
            }
            Some(_) | None => Err(self.err(ErrorCode::UnexpectedToken, "expected value")),
        }
    }

    fn parse_array(&mut self) -> Result<AstValue> {
        let start = self.pos;
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws_comments()?;
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(AstValue::Array {
                items,
                span: Span::new(start, self.pos),
            });
        }
        loop {
            self.skip_ws_comments()?;
            if self.peek() == Some(b'.') && self.starts_with(b"...") {
                let item_start = self.pos;
                let loc = self.loc();
                self.consume(b"...");
                self.skip_inline_ws()?;
                let path = self.parse_substitution()?;
                items.push(ArrayItem::Spread {
                    path: path.path,
                    path_span: path.span,
                    loc,
                    span: Span::new(item_start, self.pos),
                });
            } else {
                items.push(ArrayItem::Value(self.parse_value()?));
            }
            self.skip_ws_comments()?;
            match self.peek() {
                Some(b',') => {
                    self.bump();
                    self.skip_ws_comments()?;
                    if self.peek() == Some(b',') {
                        return Err(
                            self.err(ErrorCode::UnexpectedToken, "consecutive commas are invalid")
                        );
                    }
                    if self.peek() == Some(b']') {
                        self.bump();
                        break;
                    }
                }
                Some(b']') => {
                    self.bump();
                    break;
                }
                _ => {
                    return Err(self.err(
                        ErrorCode::UnexpectedToken,
                        "array elements must be separated by commas",
                    ));
                }
            }
        }
        Ok(AstValue::Array {
            items,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_path(&mut self) -> Result<ParsedPath> {
        let start = self.pos;
        let mut path = SconPath::new();
        path.push(self.parse_path_segment()?);
        while self.peek() == Some(b'.') {
            self.bump();
            path.push(self.parse_path_segment()?);
        }
        Ok(ParsedPath {
            path,
            span: Span::new(start, self.pos),
        })
    }

    fn parse_path_segment(&mut self) -> Result<String> {
        match self.peek() {
            Some(b'"') => self.parse_string_no_interpolation(),
            Some(ch) if is_ident_start(ch) => {
                let start = self.pos;
                self.bump();
                self.skip_ident_continue();
                Ok(self.source[start..self.pos].to_string())
            }
            _ => Err(self.err(ErrorCode::UnexpectedToken, "expected path segment")),
        }
    }

    fn parse_substitution(&mut self) -> Result<ParsedPath> {
        self.expect(b'$')?;
        self.expect(b'{')?;
        let path_start = self.pos;
        let path = self.parse_path()?;
        self.expect(b'}')?;
        Ok(ParsedPath {
            path: path.path,
            span: Span::new(path_start, self.pos - 1),
        })
    }

    fn parse_string_no_interpolation(&mut self) -> Result<String> {
        match self.parse_string_value(false)? {
            StringValue::Literal(text) => Ok(text),
            StringValue::Parts(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        StringPart::Literal(text) => out.push_str(&text),
                        StringPart::Interpolation { .. } => {
                            return Err(self.err(
                                ErrorCode::UnexpectedToken,
                                "interpolation is not allowed here",
                            ));
                        }
                    }
                }
                Ok(out)
            }
        }
    }

    fn parse_string_value(&mut self, allow_interpolation: bool) -> Result<StringValue> {
        self.expect(b'"')?;
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut had_interpolation = false;
        let mut segment_start = self.pos;
        loop {
            self.skip_string_literal_bytes();
            match self.peek() {
                Some(b'"') => {
                    if !had_interpolation && literal.is_empty() {
                        let text = self.source[segment_start..self.pos].to_string();
                        self.bump();
                        return Ok(StringValue::Literal(text));
                    }
                    literal.push_str(&self.source[segment_start..self.pos]);
                    self.bump();
                    if !had_interpolation {
                        return Ok(StringValue::Literal(literal));
                    }
                    break;
                }
                Some(b'\\') => {
                    literal.push_str(&self.source[segment_start..self.pos]);
                    self.bump();
                    match self.bump() {
                        Some(b'"') => literal.push('"'),
                        Some(b'\\') => literal.push('\\'),
                        Some(b'/') => literal.push('/'),
                        Some(b'b') => literal.push('\u{0008}'),
                        Some(b'f') => literal.push('\u{000c}'),
                        Some(b'n') => literal.push('\n'),
                        Some(b'r') => literal.push('\r'),
                        Some(b't') => literal.push('\t'),
                        Some(b'$') => literal.push('$'),
                        Some(b'u') => literal.push(self.parse_unicode_escape()?),
                        _ => {
                            return Err(self.err(ErrorCode::InvalidEscape, "invalid string escape"));
                        }
                    }
                    segment_start = self.pos;
                }
                Some(b'$') if self.peek_n(1) == Some(b'{') => {
                    if !allow_interpolation {
                        return Err(self.err(
                            ErrorCode::UnexpectedToken,
                            "interpolation is not allowed here",
                        ));
                    }
                    literal.push_str(&self.source[segment_start..self.pos]);
                    if !literal.is_empty() {
                        parts.push(StringPart::Literal(std::mem::take(&mut literal)));
                    }
                    had_interpolation = true;
                    let interpolation_start = self.pos;
                    self.bump();
                    self.bump();
                    let path_start = self.pos;
                    let path = self.parse_path()?;
                    self.expect(b'}')?;
                    parts.push(StringPart::Interpolation {
                        path: path.path,
                        path_span: Span::new(path_start, self.pos - 1),
                        span: Span::new(interpolation_start, self.pos),
                    });
                    segment_start = self.pos;
                }
                Some(b'\n') => {
                    return Err(self.err(
                        ErrorCode::UnterminatedString,
                        "multiline strings are not supported",
                    ));
                }
                Some(b'\r') if self.peek_n(1) == Some(b'\n') => {
                    return Err(self.err(
                        ErrorCode::UnterminatedString,
                        "multiline strings are not supported",
                    ));
                }
                Some(b'\r') => {
                    return Err(self.err(ErrorCode::InvalidCharacter, "standalone CR is invalid"));
                }
                Some(ch) if ch < 0x20 => {
                    return Err(
                        self.err(ErrorCode::InvalidCharacter, "control character in string")
                    );
                }
                Some(_) => {
                    self.bump_char().ok_or_else(|| {
                        self.err(ErrorCode::UnterminatedString, "unterminated string")
                    })?;
                }
                None => return Err(self.err(ErrorCode::UnterminatedString, "unterminated string")),
            }
        }
        if !literal.is_empty() {
            parts.push(StringPart::Literal(literal));
        }
        Ok(StringValue::Parts(parts))
    }

    fn parse_unicode_escape(&mut self) -> Result<char> {
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(ch) = self.bump() else {
                return Err(self.err(ErrorCode::InvalidEscape, "incomplete unicode escape"));
            };
            value = value * 16
                + hex_value(ch)
                    .ok_or_else(|| self.err(ErrorCode::InvalidEscape, "invalid unicode escape"))?;
        }
        char::from_u32(value)
            .ok_or_else(|| self.err(ErrorCode::InvalidEscape, "invalid unicode scalar value"))
    }

    fn parse_number(&mut self) -> Result<String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        match self.peek() {
            Some(b'0') => {
                self.bump();
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.err(ErrorCode::InvalidNumber, "leading zeroes are invalid"));
                }
            }
            Some(b'1'..=b'9') => {
                self.bump();
                self.skip_ascii_digits();
            }
            _ => return Err(self.err(ErrorCode::InvalidNumber, "invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.bump();
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err(ErrorCode::InvalidNumber, "fraction requires digits"));
            }
            self.skip_ascii_digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err(ErrorCode::InvalidNumber, "exponent requires digits"));
            }
            self.skip_ascii_digits();
        }
        Ok(self.source[start..self.pos].to_string())
    }

    fn skip_inline_ws(&mut self) -> Result<()> {
        let start = self.pos;
        while let Some(ch) = self.bytes.get(self.pos).copied() {
            match ch {
                b' ' | b'\t' => self.pos += 1,
                byte if byte >= 0x80 => {
                    let loc = self.loc();
                    let ch = self.bump_char().expect("valid UTF-8 boundary");
                    if ch.is_whitespace() {
                        return Err(Error::new(
                            ErrorCode::InvalidWhitespace,
                            format!("invalid whitespace character U+{:04X}", ch as u32),
                        )
                        .at(loc));
                    }
                    self.rewind_char(ch);
                    break;
                }
                _ => break,
            }
        }
        self.column += self.pos - start;
        Ok(())
    }

    fn skip_ws_comments(&mut self) -> Result<bool> {
        let mut saw_newline = false;
        loop {
            match self.peek() {
                Some(b' ' | b'\t') => {
                    self.skip_inline_ws()?;
                }
                Some(b'\n') => {
                    saw_newline = true;
                    self.consume_newline()?;
                }
                Some(b'\r') => {
                    saw_newline = true;
                    self.consume_newline()?;
                }
                Some(b'#') => {
                    self.skip_line_comment();
                }
                Some(b'/') if self.peek_n(1) == Some(b'/') => {
                    self.skip_line_comment();
                }
                Some(byte) if byte >= 0x80 => {
                    let loc = self.loc();
                    let ch = self.bump_char().expect("valid UTF-8 boundary");
                    if ch.is_whitespace() {
                        return Err(Error::new(
                            ErrorCode::InvalidWhitespace,
                            format!("invalid whitespace character U+{:04X}", ch as u32),
                        )
                        .at(loc));
                    }
                    self.rewind_char(ch);
                    break;
                }
                _ => break,
            }
        }
        Ok(saw_newline)
    }

    fn skip_line_comment(&mut self) {
        let start = self.pos;
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|ch| !matches!(ch, b'\n' | b'\r'))
        {
            self.pos += 1;
        }
        self.column += self.pos - start;
    }

    fn starts_with_keyword(&self, s: &[u8]) -> bool {
        self.starts_with(s) && !self.peek_n(s.len()).is_some_and(is_ident_continue)
    }

    fn starts_with(&self, s: &[u8]) -> bool {
        self.bytes[self.pos..].starts_with(s)
    }

    fn consume(&mut self, s: &[u8]) {
        debug_assert!(self.starts_with(s));
        self.pos += s.len();
        self.column += s.len();
    }

    fn expect(&mut self, expected: u8) -> Result<()> {
        match self.bump() {
            Some(ch) if ch == expected => Ok(()),
            _ => Err(self.err(
                ErrorCode::UnexpectedToken,
                format!("expected '{}'", expected as char),
            )),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_n(&self, n: usize) -> Option<u8> {
        self.bytes.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.source[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn skip_ident_continue(&mut self) {
        let start = self.pos;
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|ch| is_ident_continue(*ch))
        {
            self.pos += 1;
        }
        self.column += self.pos - start;
    }

    fn skip_ascii_digits(&mut self) {
        let start = self.pos;
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.pos += 1;
        }
        self.column += self.pos - start;
    }

    fn skip_string_literal_bytes(&mut self) {
        let start = self.pos;
        while let Some(byte) = self.bytes.get(self.pos).copied() {
            match byte {
                b'"' | b'\\' | b'\n' | b'\r' | 0x00..=0x1f => break,
                b'$' if self.peek_n(1) == Some(b'{') => break,
                0x80..=0xff => break,
                _ => self.pos += 1,
            }
        }
        self.column += self.pos - start;
    }

    fn consume_newline(&mut self) -> Result<()> {
        match self.peek() {
            Some(b'\n') => {
                self.pos += 1;
                self.line += 1;
                self.column = 1;
                Ok(())
            }
            Some(b'\r') if self.peek_n(1) == Some(b'\n') => {
                self.pos += 2;
                self.line += 1;
                self.column = 1;
                Ok(())
            }
            Some(b'\r') => Err(self.err(ErrorCode::InvalidCharacter, "standalone CR is invalid")),
            _ => unreachable!("consume_newline called away from a newline"),
        }
    }

    fn rewind_char(&mut self, ch: char) {
        self.pos -= ch.len_utf8();
        if ch == '\n' {
            self.line -= 1;
            self.column = 1;
        } else {
            self.column -= 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn loc(&self) -> Location {
        Location {
            file: self.file.clone(),
            line: self.line,
            column: self.column,
            span: Span::new(self.pos, self.pos),
        }
    }

    fn err(&self, code: ErrorCode, message: impl Into<String>) -> Error {
        Error::new(code, message).at(self.loc())
    }
}

fn is_ident_start(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

fn is_ident_continue(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-'
}

fn hex_value(ch: u8) -> Option<u32> {
    match ch {
        b'0'..=b'9' => Some((ch - b'0') as u32),
        b'a'..=b'f' => Some((ch - b'a' + 10) as u32),
        b'A'..=b'F' => Some((ch - b'A' + 10) as u32),
        _ => None,
    }
}
