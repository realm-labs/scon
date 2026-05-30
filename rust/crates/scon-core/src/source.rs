use crate::ast::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePosition {
    pub line: usize,
    pub character: usize,
    pub byte: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Utf16Position {
    pub line: usize,
    pub character: usize,
}

#[derive(Clone, Debug)]
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    pub fn source_position(&self, source: &str, byte: usize) -> SourcePosition {
        let byte = byte.min(source.len());
        let line = self.line_for_byte(byte);
        let line_start = self.line_starts[line];
        SourcePosition {
            line,
            character: source[line_start..byte].chars().count(),
            byte,
        }
    }

    pub fn byte_for_line_character(
        &self,
        source: &str,
        line: usize,
        character: usize,
    ) -> Option<usize> {
        let line_start = *self.line_starts.get(line)?;
        let line_end = self.line_end(source, line);
        let mut chars_seen = 0usize;
        for (offset, _) in source[line_start..line_end].char_indices() {
            if chars_seen == character {
                return Some(line_start + offset);
            }
            chars_seen += 1;
        }
        (chars_seen == character).then_some(line_end)
    }

    pub fn utf16_position(&self, source: &str, byte: usize) -> Utf16Position {
        let byte = byte.min(source.len());
        let line = self.line_for_byte(byte);
        let line_start = self.line_starts[line];
        Utf16Position {
            line,
            character: source[line_start..byte].chars().map(char::len_utf16).sum(),
        }
    }

    pub fn byte_for_utf16_position(
        &self,
        source: &str,
        line: usize,
        character: usize,
    ) -> Option<usize> {
        let line_start = *self.line_starts.get(line)?;
        let line_end = self.line_end(source, line);
        let mut units_seen = 0usize;
        for (offset, ch) in source[line_start..line_end].char_indices() {
            if units_seen == character {
                return Some(line_start + offset);
            }
            units_seen += ch.len_utf16();
            if units_seen > character {
                return None;
            }
        }
        (units_seen == character).then_some(line_end)
    }

    pub fn source_range(&self, source: &str, span: Span) -> SourceRange {
        SourceRange {
            start: self.source_position(source, span.start_byte),
            end: self.source_position(source, span.end_byte),
        }
    }

    fn line_for_byte(&self, byte: usize) -> usize {
        match self.line_starts.binary_search(&byte) {
            Ok(line) => line,
            Err(next) => next.saturating_sub(1),
        }
    }

    fn line_end(&self, source: &str, line: usize) -> usize {
        self.line_starts
            .get(line + 1)
            .map(|next_start| next_start.saturating_sub(1))
            .unwrap_or(source.len())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommentKind {
    Hash,
    SlashSlash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Identifier,
    String,
    Number,
    Keyword,
    Punctuation,
    Comment(CommentKind),
    Whitespace,
    Newline,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comment {
    pub kind: CommentKind,
    pub text: String,
    pub range: SourceRange,
}

pub fn collect_tokens(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let start = pos;
        match bytes[pos] {
            b' ' | b'\t' => {
                pos += 1;
                while matches!(bytes.get(pos), Some(b' ' | b'\t')) {
                    pos += 1;
                }
                tokens.push(token(TokenKind::Whitespace, start, pos));
            }
            b'\n' => {
                pos += 1;
                tokens.push(token(TokenKind::Newline, start, pos));
            }
            b'\r' if bytes.get(pos + 1) == Some(&b'\n') => {
                pos += 2;
                tokens.push(token(TokenKind::Newline, start, pos));
            }
            b'#' => {
                pos = consume_comment(bytes, pos);
                tokens.push(token(TokenKind::Comment(CommentKind::Hash), start, pos));
            }
            b'/' if bytes.get(pos + 1) == Some(&b'/') => {
                pos = consume_comment(bytes, pos);
                tokens.push(token(
                    TokenKind::Comment(CommentKind::SlashSlash),
                    start,
                    pos,
                ));
            }
            b'"' => {
                pos = consume_string(bytes, pos);
                tokens.push(token(TokenKind::String, start, pos));
            }
            b'-' | b'0'..=b'9' => {
                pos += 1;
                while matches!(
                    bytes.get(pos),
                    Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                ) {
                    pos += 1;
                }
                tokens.push(token(TokenKind::Number, start, pos));
            }
            ch if is_ident_start(ch) => {
                pos += 1;
                while bytes.get(pos).is_some_and(|ch| is_ident_continue(*ch)) {
                    pos += 1;
                }
                let text = &source[start..pos];
                let kind = if matches!(text, "include" | "true" | "false" | "null") {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                };
                tokens.push(token(kind, start, pos));
            }
            b'{' | b'}' | b'[' | b']' | b'=' | b',' | b'.' | b'$' => {
                pos += 1;
                tokens.push(token(TokenKind::Punctuation, start, pos));
            }
            _ => {
                pos += source[pos..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
                tokens.push(token(TokenKind::Unknown, start, pos));
            }
        }
    }
    tokens
}

pub fn comments_from_tokens(
    source: &str,
    line_index: &LineIndex,
    tokens: &[Token],
) -> Vec<Comment> {
    tokens
        .iter()
        .filter_map(|token| {
            let TokenKind::Comment(kind) = token.kind else {
                return None;
            };
            Some(Comment {
                kind,
                text: source[token.span.start_byte..token.span.end_byte].to_string(),
                range: line_index.source_range(source, token.span),
            })
        })
        .collect()
}

fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span::new(start, end),
    }
}

fn consume_comment(bytes: &[u8], mut pos: usize) -> usize {
    while bytes
        .get(pos)
        .is_some_and(|ch| !matches!(ch, b'\n' | b'\r'))
    {
        pos += 1;
    }
    pos
}

fn consume_string(bytes: &[u8], mut pos: usize) -> usize {
    pos += 1;
    let mut escaped = false;
    while let Some(byte) = bytes.get(pos).copied() {
        pos += 1;
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'"' => break,
            b'\n' | b'\r' => break,
            _ => {}
        }
    }
    pos
}

fn is_ident_start(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

fn is_ident_continue(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-'
}
