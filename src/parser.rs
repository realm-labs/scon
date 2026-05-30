use std::path::PathBuf;

use crate::ast::*;
use crate::error::{Error, ErrorCode, Result};
use crate::lexer;

pub(crate) fn parse_str(source: &str, file: Option<PathBuf>) -> Result<Document> {
    let normalized = lexer::normalize_source(source, file.clone())?;
    Parser::new(&normalized, file).parse_document()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    file: Option<PathBuf>,
}

impl Parser {
    fn new(source: &str, file: Option<PathBuf>) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            file,
        }
    }

    fn parse_document(mut self) -> Result<Document> {
        self.skip_ws_comments()?;
        let body = if self.peek() == Some('{') {
            self.bump();
            let body = self.parse_object_body(Some('}'))?;
            self.expect('}')?;
            body
        } else if matches!(self.peek(), Some('[') | Some('-') | Some('0'..='9')) {
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
        })
    }

    fn parse_object_body(&mut self, terminator: Option<char>) -> Result<ObjectBody> {
        let mut body = ObjectBody::default();
        let mut locals_seen = false;
        loop {
            let saw_newline = self.skip_ws_comments()?;
            if self.is_eof() || terminator.is_some_and(|t| self.peek() == Some(t)) {
                break;
            }
            if saw_newline {
                // A newline may separate members; parsing continues below.
            }
            if self.starts_with("...") {
                if locals_seen {
                    return Err(self.err(
                        ErrorCode::InvalidSpread,
                        "object spread must appear before local members",
                    ));
                }
                let loc = self.loc();
                self.consume("...");
                self.skip_inline_ws();
                let path = self.parse_substitution()?;
                body.spreads.push(ObjectSpread { path, loc });
            } else {
                locals_seen = true;
                body.members.push(self.parse_local_member()?);
            }

            let saw_newline = self.skip_ws_comments()?;
            if self.peek() == Some(',') {
                self.bump();
                self.skip_ws_comments()?;
                if self.peek() == Some(',') {
                    return Err(
                        self.err(ErrorCode::UnexpectedToken, "consecutive commas are invalid")
                    );
                }
                continue;
            }
            if self.is_eof() || terminator.is_some_and(|t| self.peek() == Some(t)) {
                break;
            }
            if saw_newline {
                continue;
            }
            return Err(self.err(
                ErrorCode::UnexpectedToken,
                "object members must be separated by newline or comma",
            ));
        }
        Ok(body)
    }

    fn parse_local_member(&mut self) -> Result<LocalMember> {
        if self.starts_with_keyword("include") {
            let checkpoint = (self.pos, self.line, self.column);
            let loc = self.loc();
            self.consume("include");
            if matches!(self.peek(), Some(' ' | '\t')) {
                self.skip_inline_ws();
                let path = self.parse_string_no_interpolation()?;
                return Ok(LocalMember::Include { path, loc });
            }
            self.pos = checkpoint.0;
            self.line = checkpoint.1;
            self.column = checkpoint.2;
        }

        let loc = self.loc();
        let path = self.parse_path()?;
        self.skip_inline_ws();
        let value = match self.peek() {
            Some('=') => {
                self.bump();
                self.skip_inline_ws();
                if self.peek() == Some('\n') || self.is_eof() {
                    return Err(self.err(
                        ErrorCode::UnexpectedToken,
                        "field value must be on the same logical line as =",
                    ));
                }
                self.parse_value()?
            }
            Some('{') => {
                self.bump();
                AstValue::Object(self.parse_object_body(Some('}')).and_then(|b| {
                    self.expect('}')?;
                    Ok(b)
                })?)
            }
            _ => {
                return Err(self.err(
                    ErrorCode::UnexpectedToken,
                    "expected = or object shorthand after path",
                ));
            }
        };
        Ok(LocalMember::Field(Field { path, value, loc }))
    }

    fn parse_value(&mut self) -> Result<AstValue> {
        self.skip_inline_ws();
        match self.peek() {
            Some('{') => {
                self.bump();
                let body = self.parse_object_body(Some('}'))?;
                self.expect('}')?;
                Ok(AstValue::Object(body))
            }
            Some('[') => self.parse_array(),
            Some('"') => Ok(AstValue::String(self.parse_string_parts(true)?)),
            Some('$') if self.peek_n(1) == Some('{') => {
                Ok(AstValue::Substitution(self.parse_substitution()?))
            }
            Some('-' | '0'..='9') => Ok(AstValue::Number(self.parse_number()?)),
            Some('t') if self.starts_with_keyword("true") => {
                self.consume("true");
                Ok(AstValue::Bool(true))
            }
            Some('f') if self.starts_with_keyword("false") => {
                self.consume("false");
                Ok(AstValue::Bool(false))
            }
            Some('n') if self.starts_with_keyword("null") => {
                self.consume("null");
                Ok(AstValue::Null)
            }
            Some(_) => Err(self.err(ErrorCode::UnexpectedToken, "expected value")),
            None => Err(self.err(ErrorCode::UnexpectedToken, "expected value")),
        }
    }

    fn parse_array(&mut self) -> Result<AstValue> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws_comments()?;
        if self.peek() == Some(']') {
            self.bump();
            return Ok(AstValue::Array(items));
        }
        loop {
            self.skip_ws_comments()?;
            if self.starts_with("...") {
                let loc = self.loc();
                self.consume("...");
                self.skip_inline_ws();
                let path = self.parse_substitution()?;
                items.push(ArrayItem::Spread { path, loc });
            } else {
                items.push(ArrayItem::Value(self.parse_value()?));
            }
            self.skip_ws_comments()?;
            match self.peek() {
                Some(',') => {
                    self.bump();
                    self.skip_ws_comments()?;
                    if self.peek() == Some(',') {
                        return Err(
                            self.err(ErrorCode::UnexpectedToken, "consecutive commas are invalid")
                        );
                    }
                    if self.peek() == Some(']') {
                        self.bump();
                        break;
                    }
                }
                Some(']') => {
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
        Ok(AstValue::Array(items))
    }

    fn parse_path(&mut self) -> Result<Vec<String>> {
        let mut path = vec![self.parse_path_segment()?];
        while self.peek() == Some('.') {
            self.bump();
            path.push(self.parse_path_segment()?);
        }
        Ok(path)
    }

    fn parse_path_segment(&mut self) -> Result<String> {
        match self.peek() {
            Some('"') => self.parse_string_no_interpolation(),
            Some(ch) if is_ident_start(ch) => {
                let mut out = String::new();
                out.push(self.bump().unwrap());
                while let Some(ch) = self.peek() {
                    if is_ident_continue(ch) {
                        out.push(self.bump().unwrap());
                    } else {
                        break;
                    }
                }
                Ok(out)
            }
            _ => Err(self.err(ErrorCode::UnexpectedToken, "expected path segment")),
        }
    }

    fn parse_substitution(&mut self) -> Result<Vec<String>> {
        self.expect('$')?;
        self.expect('{')?;
        let path = self.parse_path()?;
        self.expect('}')?;
        Ok(path)
    }

    fn parse_string_no_interpolation(&mut self) -> Result<String> {
        let parts = self.parse_string_parts(false)?;
        let mut out = String::new();
        for part in parts {
            match part {
                StringPart::Literal(text) => out.push_str(&text),
                StringPart::Interpolation(_) => {
                    return Err(self.err(
                        ErrorCode::UnexpectedToken,
                        "interpolation is not allowed here",
                    ));
                }
            }
        }
        Ok(out)
    }

    fn parse_string_parts(&mut self, allow_interpolation: bool) -> Result<Vec<StringPart>> {
        self.expect('"')?;
        let mut parts = Vec::new();
        let mut literal = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => literal.push('"'),
                    Some('\\') => literal.push('\\'),
                    Some('/') => literal.push('/'),
                    Some('b') => literal.push('\u{0008}'),
                    Some('f') => literal.push('\u{000c}'),
                    Some('n') => literal.push('\n'),
                    Some('r') => literal.push('\r'),
                    Some('t') => literal.push('\t'),
                    Some('$') => literal.push('$'),
                    Some('u') => literal.push(self.parse_unicode_escape()?),
                    _ => return Err(self.err(ErrorCode::InvalidEscape, "invalid string escape")),
                },
                Some('$') if self.peek() == Some('{') => {
                    if !allow_interpolation {
                        return Err(self.err(
                            ErrorCode::UnexpectedToken,
                            "interpolation is not allowed here",
                        ));
                    }
                    if !literal.is_empty() {
                        parts.push(StringPart::Literal(std::mem::take(&mut literal)));
                    }
                    self.bump();
                    let path = self.parse_path()?;
                    self.expect('}')?;
                    parts.push(StringPart::Interpolation(path));
                }
                Some('\n') => {
                    return Err(self.err(
                        ErrorCode::UnterminatedString,
                        "multiline strings are not supported",
                    ));
                }
                Some(ch) if ch.is_control() => {
                    return Err(
                        self.err(ErrorCode::InvalidCharacter, "control character in string")
                    );
                }
                Some(ch) => literal.push(ch),
                None => return Err(self.err(ErrorCode::UnterminatedString, "unterminated string")),
            }
        }
        if !literal.is_empty() || parts.is_empty() {
            parts.push(StringPart::Literal(literal));
        }
        Ok(parts)
    }

    fn parse_unicode_escape(&mut self) -> Result<char> {
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(ch) = self.bump() else {
                return Err(self.err(ErrorCode::InvalidEscape, "incomplete unicode escape"));
            };
            value = value * 16
                + ch.to_digit(16)
                    .ok_or_else(|| self.err(ErrorCode::InvalidEscape, "invalid unicode escape"))?;
        }
        char::from_u32(value)
            .ok_or_else(|| self.err(ErrorCode::InvalidEscape, "invalid unicode scalar value"))
    }

    fn parse_number(&mut self) -> Result<String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        match self.peek() {
            Some('0') => {
                self.bump();
                if matches!(self.peek(), Some('0'..='9')) {
                    return Err(self.err(ErrorCode::InvalidNumber, "leading zeroes are invalid"));
                }
            }
            Some('1'..='9') => {
                self.bump();
                while matches!(self.peek(), Some('0'..='9')) {
                    self.bump();
                }
            }
            _ => return Err(self.err(ErrorCode::InvalidNumber, "invalid number")),
        }
        if self.peek() == Some('.') {
            self.bump();
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.err(ErrorCode::InvalidNumber, "fraction requires digits"));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.err(ErrorCode::InvalidNumber, "exponent requires digits"));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.bump();
            }
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    fn skip_ws_comments(&mut self) -> Result<bool> {
        let mut saw_newline = false;
        loop {
            match self.peek() {
                Some(' ' | '\t') => {
                    self.bump();
                }
                Some('\n') => {
                    saw_newline = true;
                    self.bump();
                }
                Some('#') => {
                    self.skip_line_comment();
                }
                Some('/') if self.peek_n(1) == Some('/') => {
                    self.skip_line_comment();
                }
                Some(ch) if ch.is_whitespace() => {
                    // Parser input has already rejected CR. Other non-SCON whitespace is invalid.
                    let loc = self.loc();
                    self.bump();
                    return Err(Error::new(
                        ErrorCode::InvalidWhitespace,
                        format!("invalid whitespace character U+{:04X}", ch as u32),
                    )
                    .at(loc));
                }
                _ => break,
            }
        }
        Ok(saw_newline)
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.bump();
        }
    }

    fn starts_with_keyword(&self, s: &str) -> bool {
        self.starts_with(s)
            && !self
                .peek_n(s.chars().count())
                .is_some_and(is_ident_continue)
    }

    fn starts_with(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, ch)| self.peek_n(i) == Some(ch))
    }

    fn consume(&mut self, s: &str) {
        for _ in s.chars() {
            self.bump();
        }
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        match self.bump() {
            Some(ch) if ch == expected => Ok(()),
            _ => Err(self.err(
                ErrorCode::UnexpectedToken,
                format!("expected '{}'", expected),
            )),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn loc(&self) -> Location {
        Location {
            file: self.file.clone(),
            line: self.line,
            column: self.column,
        }
    }

    fn err(&self, code: ErrorCode, message: impl Into<String>) -> Error {
        Error::new(code, message).at(self.loc())
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
