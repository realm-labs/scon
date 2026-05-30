use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, ErrorCode, Result};
use crate::limits::LoadOptions;
use crate::value::Value;

#[derive(Clone, Debug, Default)]
pub struct ParseOptions {
    pub file: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct FormatOptions {
    pub indent: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { indent: 2 }
    }
}

#[derive(Clone, Debug)]
pub struct SourcePosition {
    pub line: usize,
    pub character: usize,
    pub byte: usize,
}

#[derive(Clone, Debug)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Clone, Debug)]
pub enum CommentKind {
    Hash,
    SlashSlash,
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub kind: CommentKind,
    pub text: String,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub path: Vec<String>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub message: String,
    pub file: Option<PathBuf>,
    pub range: Option<SourceRange>,
}

#[derive(Clone, Debug)]
pub struct ParsedDocument {
    pub file: Option<PathBuf>,
    pub comments: Vec<Comment>,
    pub symbols: Vec<Symbol>,
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
    pub comments: Vec<Comment>,
    pub symbols: Vec<Symbol>,
    pub value: Option<Value>,
}

pub fn parse_source(source: &str, options: ParseOptions) -> Result<ParsedDocument> {
    crate::parser::parse_str(source, options.file.clone())?;
    Ok(ParsedDocument {
        file: options.file,
        comments: collect_comments(source),
        symbols: collect_symbols(source),
    })
}

pub fn analyze_source(source: &str, options: ParseOptions) -> Analysis {
    match parse_source(source, options) {
        Ok(parsed) => Analysis {
            diagnostics: Vec::new(),
            comments: parsed.comments,
            symbols: parsed.symbols,
            value: None,
        },
        Err(err) => Analysis {
            diagnostics: vec![diagnostic_from_error(&err, source)],
            comments: collect_comments(source),
            symbols: collect_symbols(source),
            value: None,
        },
    }
}

pub fn analyze_file(path: impl AsRef<Path>, options: LoadOptions) -> Analysis {
    let path = path.as_ref();
    let source = fs::read_to_string(path).unwrap_or_default();
    let comments = collect_comments(&source);
    let symbols = collect_symbols(&source);
    match crate::parse_file_with_options(path, options) {
        Ok(value) => Analysis {
            diagnostics: Vec::new(),
            comments,
            symbols,
            value: Some(value),
        },
        Err(err) => Analysis {
            diagnostics: vec![diagnostic_from_error(&err, &source)],
            comments,
            symbols,
            value: None,
        },
    }
}

pub fn format_source(source: &str, options: FormatOptions) -> Result<String> {
    crate::parser::parse_str(source, None)?;
    Ok(format_source_unchecked(source, options))
}

pub fn resolve_source(source: &str, options: ParseOptions) -> Result<Value> {
    let doc = crate::parser::parse_str(source, options.file)?;
    crate::eval::eval_document(doc, &mut crate::loader::NoopLoader)
}

pub fn resolve_file(path: impl AsRef<Path>, options: LoadOptions) -> Result<Value> {
    crate::parse_file_with_options(path, options)
}

pub fn get_path<'a>(value: &'a Value, path: &str) -> Result<&'a Value> {
    let path = parse_path_query(path)?;
    let mut current = value;
    for segment in &path {
        let Value::Object(object) = current else {
            return Err(Error::new(
                ErrorCode::TypeMismatch,
                "path segment requires object",
            ));
        };
        current = object.get(segment).ok_or_else(|| {
            Error::new(
                ErrorCode::MissingReference,
                format!("path {:?} is not defined", path),
            )
            .with_path(&path)
        })?;
    }
    Ok(current)
}

pub fn diagnostic_from_error(error: &Error, source: &str) -> Diagnostic {
    let start = SourcePosition {
        line: error.line.saturating_sub(1),
        character: error.column.saturating_sub(1),
        byte: line_column_to_byte(source, error.line, error.column).unwrap_or(0),
    };
    let end = SourcePosition {
        line: start.line,
        character: start.character + 1,
        byte: start.byte.saturating_add(1),
    };
    Diagnostic {
        code: error.code,
        message: error.message.clone(),
        file: error.file.clone(),
        range: Some(SourceRange { start, end }),
    }
}

fn format_source_unchecked(source: &str, options: FormatOptions) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        if starts_with_closer(trimmed) {
            indent = indent.saturating_sub(options.indent);
        }
        write_spaces(&mut out, indent);
        out.push_str(&format_line(trimmed));
        out.push('\n');
        if opens_block(trimmed) {
            indent += options.indent;
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn format_line(line: &str) -> String {
    let Some(eq) = find_unquoted(line, b'=') else {
        return line.to_string();
    };
    let left = line[..eq].trim_end();
    let right = line[eq + 1..].trim_start();
    format!("{left} = {right}")
}

fn starts_with_closer(line: &str) -> bool {
    matches!(line.as_bytes().first(), Some(b'}' | b']'))
}

fn opens_block(line: &str) -> bool {
    let code = strip_inline_comment(line).trim_end();
    code.ends_with('{') || code.ends_with('[')
}

fn strip_inline_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        match byte {
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            b'#' if !in_string => return &line[..i],
            b'/' if !in_string && bytes.get(i + 1) == Some(&b'/') => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

fn find_unquoted(line: &str, target: u8) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in line.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            byte if byte == target && !in_string => return Some(index),
            b'#' if !in_string => return None,
            b'/' if !in_string && line.as_bytes().get(index + 1) == Some(&b'/') => return None,
            _ => {}
        }
    }
    None
}

fn collect_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut byte_offset = 0usize;
    for (line_index, line) in source.lines().enumerate() {
        if let Some((column, kind)) = find_comment_start(line) {
            let text = line[column..].to_string();
            let len = text.len();
            comments.push(Comment {
                kind,
                text,
                range: SourceRange {
                    start: SourcePosition {
                        line: line_index,
                        character: column,
                        byte: byte_offset + column,
                    },
                    end: SourcePosition {
                        line: line_index,
                        character: column + len,
                        byte: byte_offset + column + len,
                    },
                },
            });
        }
        byte_offset += line.len() + 1;
    }
    comments
}

fn find_comment_start(line: &str) -> Option<(usize, CommentKind)> {
    let mut in_string = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            b'#' if !in_string => return Some((index, CommentKind::Hash)),
            b'/' if !in_string && bytes.get(index + 1) == Some(&b'/') => {
                return Some((index, CommentKind::SlashSlash));
            }
            _ => {}
        }
    }
    None
}

fn collect_symbols(source: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut byte_offset = 0usize;
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = strip_inline_comment(line).trim_start();
        let leading = line.len() - line.trim_start().len();
        if trimmed.is_empty() || trimmed.starts_with("include") || trimmed.starts_with("...") {
            byte_offset += line.len() + 1;
            continue;
        }
        let end = trimmed
            .find(|ch: char| ch == '=' || ch == '{' || ch.is_whitespace())
            .unwrap_or(trimmed.len());
        let key = &trimmed[..end];
        if !key.is_empty() {
            let path = key.split('.').map(str::to_string).collect::<Vec<_>>();
            symbols.push(Symbol {
                path,
                range: SourceRange {
                    start: SourcePosition {
                        line: line_index,
                        character: leading,
                        byte: byte_offset + leading,
                    },
                    end: SourcePosition {
                        line: line_index,
                        character: leading + key.len(),
                        byte: byte_offset + leading + key.len(),
                    },
                },
            });
        }
        byte_offset += line.len() + 1;
    }
    symbols
}

fn parse_path_query(path: &str) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '.' if !current.is_empty() => {
                parts.push(std::mem::take(&mut current));
            }
            '"' if current.is_empty() => {
                while let Some(ch) = chars.next() {
                    match ch {
                        '"' => break,
                        '\\' => {
                            let Some(escaped) = chars.next() else {
                                return Err(Error::new(
                                    ErrorCode::InvalidEscape,
                                    "unterminated quoted path escape",
                                ));
                            };
                            current.push(escaped);
                        }
                        ch => current.push(ch),
                    }
                }
            }
            ch => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    if parts.is_empty() {
        return Err(Error::new(ErrorCode::UnexpectedToken, "empty path"));
    }
    Ok(parts)
}

fn line_column_to_byte(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut current_line = 1usize;
    let mut current_column = 1usize;
    for (index, ch) in source.char_indices() {
        if current_line == line && current_column == column {
            return Some(index);
        }
        if ch == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }
    Some(source.len())
}

fn write_spaces(out: &mut String, count: usize) {
    for _ in 0..count {
        out.push(' ');
    }
}
