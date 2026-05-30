use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    ArrayItem, AstValue, Document, LocalMember, ObjectBody, SconPath, Span, StringPart,
};
use crate::error::{Error, ErrorCode, Result};
use crate::limits::LoadOptions;
use crate::loader::IncludeLoader;
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
    pub file: Option<PathBuf>,
    pub range: SourceRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug)]
pub struct DiagnosticRelatedInformation {
    pub file: Option<PathBuf>,
    pub range: SourceRange,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub file: Option<PathBuf>,
    pub range: Option<SourceRange>,
    pub related_information: Vec<DiagnosticRelatedInformation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionKind {
    Field,
}

#[derive(Clone, Debug)]
pub struct Definition {
    pub path: Vec<String>,
    pub kind: DefinitionKind,
    pub file: Option<PathBuf>,
    pub range: SourceRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceKind {
    Substitution,
    Interpolation,
    ObjectSpread,
    ArraySpread,
}

#[derive(Clone, Debug)]
pub struct Reference {
    pub path: Vec<String>,
    pub kind: ReferenceKind,
    pub file: Option<PathBuf>,
    pub range: SourceRange,
    pub target: Option<Definition>,
}

#[derive(Clone, Debug)]
pub struct IncludeReference {
    pub path: String,
    pub file: Option<PathBuf>,
    pub range: SourceRange,
    pub resolved_path: Option<PathBuf>,
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
pub struct AnalyzedDocument {
    pub file: Option<PathBuf>,
    pub parsed: Option<ParsedDocument>,
    pub diagnostics: Vec<Diagnostic>,
    pub comments: Vec<Comment>,
    pub symbols: Vec<Symbol>,
    pub definitions: Vec<Definition>,
    pub references: Vec<Reference>,
    pub includes: Vec<IncludeReference>,
    pub value: Option<Value>,
}

pub trait SourceStore {
    fn read_source(&self, path: &Path) -> std::io::Result<Option<String>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FileSourceStore;

impl SourceStore for FileSourceStore {
    fn read_source(&self, path: &Path) -> std::io::Result<Option<String>> {
        match fs::read_to_string(path) {
            Ok(source) => Ok(Some(source)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}

pub fn parse_source(source: &str, options: ParseOptions) -> Result<ParsedDocument> {
    let document = crate::parser::parse_str(source, options.file.clone())?;
    let line_index = LineIndex::new(source);
    let tokens = source::collect_tokens(source);
    let comments = source::comments_from_tokens(source, &line_index, &tokens);
    let symbols = collect_symbols(
        &document.body,
        source,
        &line_index,
        options.file.as_ref(),
        &[],
    );
    Ok(ParsedDocument {
        file: options.file,
        line_index,
        tokens,
        comments,
        symbols,
    })
}

pub fn analyze_source(source: &str, options: ParseOptions) -> AnalyzedDocument {
    let file = options.file.clone();
    let document = match crate::parser::parse_str(source, file.clone()) {
        Ok(document) => document,
        Err(err) => {
            let line_index = LineIndex::new(source);
            let tokens = source::collect_tokens(source);
            let comments = source::comments_from_tokens(source, &line_index, &tokens);
            return AnalyzedDocument {
                file,
                parsed: None,
                diagnostics: vec![diagnostic_from_error(&err, source)],
                comments,
                symbols: Vec::new(),
                definitions: Vec::new(),
                references: Vec::new(),
                includes: Vec::new(),
                value: None,
            };
        }
    };
    analyze_parsed_document(
        source,
        document,
        file,
        &mut crate::loader::NoopLoader,
        |_| None,
    )
}

pub fn analyze_file(path: impl AsRef<Path>, options: LoadOptions) -> AnalyzedDocument {
    analyze_file_with_store(path, options, &FileSourceStore)
}

pub fn analyze_file_with_store(
    path: impl AsRef<Path>,
    options: LoadOptions,
    store: &dyn SourceStore,
) -> AnalyzedDocument {
    let path = path.as_ref();
    let source = store.read_source(path).ok().flatten().unwrap_or_default();
    let file = Some(path.to_path_buf());
    let document = match crate::parser::parse_str(&source, file.clone()) {
        Ok(document) => document,
        Err(err) => {
            let line_index = LineIndex::new(&source);
            let tokens = source::collect_tokens(&source);
            let comments = source::comments_from_tokens(&source, &line_index, &tokens);
            return AnalyzedDocument {
                file,
                parsed: None,
                diagnostics: vec![diagnostic_from_error(&err, &source)],
                comments,
                symbols: Vec::new(),
                definitions: Vec::new(),
                references: Vec::new(),
                includes: Vec::new(),
                value: None,
            };
        }
    };
    let mut loader = AnalysisLoader::new(path, options, store);
    analyze_parsed_document(&source, document, file, &mut loader, |error| {
        error
            .file
            .as_deref()
            .and_then(|path| store.read_source(path).ok().flatten())
    })
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
    let range = if let Some(span) = error.span {
        line_index.source_range(source, span)
    } else {
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
        SourceRange { start, end }
    };
    Diagnostic {
        code: error.code,
        message: error.message.clone(),
        severity: DiagnosticSeverity::Error,
        file: error.file.clone(),
        range: Some(range),
        related_information: related_information_from_error(error, source),
    }
}

fn analyze_parsed_document(
    source: &str,
    document: Document,
    file: Option<PathBuf>,
    loader: &mut dyn IncludeLoader,
    diagnostic_source: impl Fn(&Error) -> Option<String>,
) -> AnalyzedDocument {
    let line_index = LineIndex::new(source);
    let tokens = source::collect_tokens(source);
    let comments = source::comments_from_tokens(source, &line_index, &tokens);
    let mut semantic = SemanticCollector::new(source, &line_index, file.as_ref());
    semantic.collect_body(&document.body, &[]);
    let symbols = collect_symbols(&document.body, source, &line_index, file.as_ref(), &[]);
    let definitions = semantic.definitions;
    let references = semantic.references;
    let includes = semantic.includes;
    let parsed = ParsedDocument {
        file: file.clone(),
        line_index,
        tokens,
        comments: comments.clone(),
        symbols: symbols.clone(),
    };
    let (diagnostics, value) = match crate::eval::eval_document(document, loader) {
        Ok(value) => (Vec::new(), Some(value)),
        Err(err) => {
            let source = diagnostic_source(&err).unwrap_or_else(|| source.to_string());
            (vec![diagnostic_from_error(&err, &source)], None)
        }
    };
    AnalyzedDocument {
        file,
        parsed: Some(parsed),
        diagnostics,
        comments,
        symbols,
        definitions,
        references,
        includes,
        value,
    }
}

fn related_information_from_error(
    error: &Error,
    source: &str,
) -> Vec<DiagnosticRelatedInformation> {
    error
        .include_stack
        .iter()
        .map(|path| DiagnosticRelatedInformation {
            file: Some(path.clone()),
            range: LineIndex::new(source).source_range(source, Span::default()),
            message: "included from here".to_string(),
        })
        .collect()
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
    file: Option<&PathBuf>,
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
            file: file.cloned(),
            range: line_index.source_range(source, field.path_span),
        });
        if let AstValue::Object { body, .. } = &field.value {
            symbols.extend(collect_symbols(body, source, line_index, file, &path));
        }
    }
    symbols
}

struct SemanticCollector<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
    file: Option<&'a PathBuf>,
    completed_definitions: HashMap<Vec<String>, Definition>,
    definitions: Vec<Definition>,
    references: Vec<Reference>,
    includes: Vec<IncludeReference>,
}

impl<'a> SemanticCollector<'a> {
    fn new(source: &'a str, line_index: &'a LineIndex, file: Option<&'a PathBuf>) -> Self {
        Self {
            source,
            line_index,
            file,
            completed_definitions: HashMap::new(),
            definitions: Vec::new(),
            references: Vec::new(),
            includes: Vec::new(),
        }
    }

    fn collect_body(&mut self, body: &ObjectBody, parent_path: &[String]) {
        for spread in &body.spreads {
            self.push_reference(&spread.path, ReferenceKind::ObjectSpread, spread.path_span);
        }

        for member in &body.members {
            match member {
                LocalMember::Include {
                    path,
                    path_span,
                    loc,
                    ..
                } => {
                    self.includes.push(IncludeReference {
                        path: path.clone(),
                        file: self.file.cloned(),
                        range: self.source_range(*path_span),
                        resolved_path: resolve_include_path(self.file, path, loc).ok(),
                    });
                }
                LocalMember::Field(field) => {
                    let full_path = join_paths(parent_path, &field.path);
                    match &field.value {
                        AstValue::Object { body, .. } => {
                            self.push_definition(full_path.clone(), field.path_span);
                            self.collect_body(body, &full_path);
                        }
                        value => {
                            self.collect_value(value);
                            self.push_definition(full_path, field.path_span);
                        }
                    }
                }
            }
        }
    }

    fn collect_value(&mut self, value: &AstValue) {
        match value {
            AstValue::Object { body, .. } => self.collect_body(body, &[]),
            AstValue::Array { items, .. } => {
                for item in items {
                    match item {
                        ArrayItem::Value(value) => self.collect_value(value),
                        ArrayItem::Spread {
                            path, path_span, ..
                        } => self.push_reference(path, ReferenceKind::ArraySpread, *path_span),
                    }
                }
            }
            AstValue::String { value, .. } => {
                if let crate::ast::StringValue::Parts(parts) = value {
                    for part in parts {
                        if let StringPart::Interpolation {
                            path, path_span, ..
                        } = part
                        {
                            self.push_reference(path, ReferenceKind::Interpolation, *path_span);
                        }
                    }
                }
            }
            AstValue::Substitution {
                path, path_span, ..
            } => self.push_reference(path, ReferenceKind::Substitution, *path_span),
            AstValue::Number { .. } | AstValue::Bool { .. } | AstValue::Null { .. } => {}
        }
    }

    fn push_definition(&mut self, path: Vec<String>, span: Span) {
        let definition = Definition {
            path: path.clone(),
            kind: DefinitionKind::Field,
            file: self.file.cloned(),
            range: self.source_range(span),
        };
        self.completed_definitions.insert(path, definition.clone());
        self.definitions.push(definition);
    }

    fn push_reference(&mut self, path: &SconPath, kind: ReferenceKind, span: Span) {
        let path = path.iter().cloned().collect::<Vec<_>>();
        let target = self.completed_definitions.get(&path).cloned();
        self.references.push(Reference {
            path,
            kind,
            file: self.file.cloned(),
            range: self.source_range(span),
            target,
        });
    }

    fn source_range(&self, span: Span) -> SourceRange {
        self.line_index.source_range(self.source, span)
    }
}

fn join_paths(parent: &[String], child: &SconPath) -> Vec<String> {
    let mut path = Vec::with_capacity(parent.len() + child.len());
    path.extend(parent.iter().cloned());
    path.extend(child.iter().cloned());
    path
}

fn resolve_include_path(
    file: Option<&PathBuf>,
    include_path: &str,
    loc: &crate::ast::Location,
) -> Result<PathBuf> {
    validate_include_path(include_path, loc.clone())?;
    let base = file
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new("."));
    Ok(base.join(include_path))
}

struct AnalysisLoader<'a> {
    include_root: PathBuf,
    options: LoadOptions,
    store: &'a dyn SourceStore,
    cache: HashMap<PathBuf, Document>,
    stack: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
}

impl<'a> AnalysisLoader<'a> {
    fn new(entry: &Path, mut options: LoadOptions, store: &'a dyn SourceStore) -> Self {
        let entry = normalize_path(entry);
        let default_root = entry
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let include_root = options.include_root.take().unwrap_or(default_root);
        Self {
            include_root: normalize_path(&include_root),
            options,
            store,
            cache: HashMap::new(),
            stack: Vec::new(),
            seen: HashSet::new(),
        }
    }

    fn load_canonical(&mut self, path: PathBuf) -> Result<Document> {
        if let Some(doc) = self.cache.get(&path) {
            return Ok(doc.clone());
        }
        if self.stack.contains(&path) {
            return Err(Error::new(
                ErrorCode::IncludeCycle,
                format!("include cycle: {}", path.display()),
            ));
        }
        if self.stack.len() >= self.options.limits.max_include_depth {
            return Err(Error::new(
                ErrorCode::ResourceLimitExceeded,
                "maximum include depth exceeded",
            ));
        }
        self.stack.push(path.clone());
        self.seen.insert(path.clone());
        if self.seen.len() > self.options.limits.max_include_files {
            return Err(Error::new(
                ErrorCode::ResourceLimitExceeded,
                "maximum include file count exceeded",
            ));
        }
        let source = self
            .store
            .read_source(&path)
            .map_err(|err| {
                Error::new(
                    ErrorCode::IncludeNotFound,
                    format!("failed to read include file: {err}"),
                )
            })?
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::IncludeNotFound,
                    format!("include file not found: {}", path.display()),
                )
            })?;
        if source.len() > self.options.limits.max_file_size {
            return Err(Error::new(
                ErrorCode::ResourceLimitExceeded,
                "maximum file size exceeded",
            ));
        }
        let doc = crate::parser::parse_str(&source, Some(path.clone())).map_err(|err| Error {
            code: ErrorCode::IncludeParseError,
            ..err
        })?;
        self.stack.pop();
        self.cache.insert(path, doc.clone());
        Ok(doc)
    }

    fn resolve_include(
        &self,
        including_file: Option<&Path>,
        include_path: &str,
        loc: crate::ast::Location,
    ) -> Result<PathBuf> {
        validate_include_path(include_path, loc.clone())?;
        let base = including_file
            .and_then(Path::parent)
            .unwrap_or(&self.include_root);
        let candidate = normalize_path(&base.join(include_path));
        if !candidate.starts_with(&self.include_root) {
            return Err(Error::new(
                ErrorCode::IncludePathDenied,
                "include path escapes include root",
            )
            .at(loc));
        }
        Ok(candidate)
    }
}

impl IncludeLoader for AnalysisLoader<'_> {
    fn load_include(
        &mut self,
        including_file: Option<&Path>,
        path: &str,
        loc: crate::ast::Location,
    ) -> Result<Document> {
        let resolved = self.resolve_include(including_file, path, loc)?;
        self.load_canonical(resolved)
    }
}

fn validate_include_path(include_path: &str, loc: crate::ast::Location) -> Result<()> {
    if include_path.contains("://")
        || include_path.starts_with("classpath:")
        || include_path.contains('*')
        || include_path.starts_with('~')
        || include_path.starts_with('$')
        || include_path.starts_with('/')
        || looks_like_windows_absolute(include_path)
    {
        return Err(Error::new(ErrorCode::InvalidIncludePath, "invalid include path").at(loc));
    }
    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn looks_like_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/'
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
