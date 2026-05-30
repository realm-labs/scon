use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{AstValue, LocalMember, ObjectBody};
use crate::error::{Error, ErrorCode, Result};
use crate::limits::LoadOptions;
use crate::source::{self, LineIndex, Token};
pub use crate::source::{Comment, CommentKind, SourcePosition, SourceRange, Utf16Position};
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
    pub line_index: LineIndex,
    pub tokens: Vec<Token>,
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
    let document = crate::parser::parse_str(source, options.file.clone())?;
    let line_index = LineIndex::new(source);
    let tokens = source::collect_tokens(source);
    let comments = source::comments_from_tokens(source, &line_index, &tokens);
    let symbols = collect_symbols(&document.body, source, &line_index, &[]);
    Ok(ParsedDocument {
        file: options.file,
        line_index,
        tokens,
        comments,
        symbols,
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
            comments: {
                let line_index = LineIndex::new(source);
                let tokens = source::collect_tokens(source);
                source::comments_from_tokens(source, &line_index, &tokens)
            },
            symbols: Vec::new(),
            value: None,
        },
    }
}

pub fn analyze_file(path: impl AsRef<Path>, options: LoadOptions) -> Analysis {
    let path = path.as_ref();
    let source = fs::read_to_string(path).unwrap_or_default();
    let parsed = parse_source(
        &source,
        ParseOptions {
            file: Some(path.to_path_buf()),
        },
    );
    let (comments, symbols) = match parsed {
        Ok(parsed) => (parsed.comments, parsed.symbols),
        Err(_) => {
            let line_index = LineIndex::new(&source);
            let tokens = source::collect_tokens(&source);
            (
                source::comments_from_tokens(&source, &line_index, &tokens),
                Vec::new(),
            )
        }
    };
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
    let line_index = LineIndex::new(source);
    let byte = line_index
        .byte_for_line_character(
            source,
            error.line.saturating_sub(1),
            error.column.saturating_sub(1),
        )
        .unwrap_or(0);
    let start = SourcePosition {
        line: error.line.saturating_sub(1),
        character: error.column.saturating_sub(1),
        byte,
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

fn collect_symbols(
    body: &ObjectBody,
    source: &str,
    line_index: &LineIndex,
    parent_path: &[String],
) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for member in &body.members {
        let LocalMember::Field(field) = member else {
            continue;
        };
        let mut path = parent_path.to_vec();
        path.extend(field.path.iter().cloned());
        symbols.push(Symbol {
            path: path.clone(),
            range: line_index.source_range(source, field.path_span),
        });
        if let AstValue::Object { body, .. } = &field.value {
            symbols.extend(collect_symbols(body, source, line_index, &path));
        }
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

fn write_spaces(out: &mut String, count: usize) {
    for _ in 0..count {
        out.push(' ');
    }
}
