use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Document, Location};
use crate::error::{Error, ErrorCode, Result};
use crate::limits::LoadOptions;
use crate::parser;

pub(crate) trait IncludeLoader {
    fn load_include(
        &mut self,
        including_file: Option<&Path>,
        path: &str,
        loc: Location,
    ) -> Result<Document>;
}

pub(crate) struct NoopLoader;

impl IncludeLoader for NoopLoader {
    fn load_include(
        &mut self,
        _including_file: Option<&Path>,
        _path: &str,
        loc: Location,
    ) -> Result<Document> {
        Err(Error::new(
            ErrorCode::InvalidIncludePath,
            "includes are not available when parsing from a string",
        )
        .at(loc))
    }
}

pub(crate) struct FileLoader {
    entry: PathBuf,
    include_root: PathBuf,
    options: LoadOptions,
    cache: HashMap<PathBuf, Document>,
    stack: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
}

impl FileLoader {
    pub(crate) fn new(entry: &Path, mut options: LoadOptions) -> Result<Self> {
        let entry = entry.canonicalize().map_err(|err| {
            Error::new(
                ErrorCode::IncludeNotFound,
                format!("failed to open entry file: {err}"),
            )
        })?;
        if !entry.is_file() {
            return Err(Error::new(
                ErrorCode::IncludeNotFile,
                "entry path is not a file",
            ));
        }
        let default_root = entry
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let include_root = match options.include_root.take() {
            Some(root) => root.canonicalize().map_err(|err| {
                Error::new(
                    ErrorCode::InvalidIncludePath,
                    format!("failed to canonicalize include root: {err}"),
                )
            })?,
            None => default_root,
        };
        Ok(Self {
            entry,
            include_root,
            options,
            cache: HashMap::new(),
            stack: Vec::new(),
            seen: HashSet::new(),
        })
    }

    pub(crate) fn load_entry(&mut self) -> Result<Document> {
        let entry = self.entry.clone();
        self.load_canonical(&entry, false)
    }

    fn load_canonical(&mut self, path: &Path, is_include: bool) -> Result<Document> {
        if let Some(doc) = self.cache.get(path) {
            return Ok(doc.clone());
        }
        if self.stack.contains(&path.to_path_buf()) {
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
        self.stack.push(path.to_path_buf());
        self.seen.insert(path.to_path_buf());
        if self.seen.len() > self.options.limits.max_include_files {
            return Err(Error::new(
                ErrorCode::ResourceLimitExceeded,
                "maximum include file count exceeded",
            ));
        }
        let metadata = fs::metadata(path).map_err(|err| {
            Error::new(
                ErrorCode::IncludeNotFound,
                format!("failed to read include metadata: {err}"),
            )
        })?;
        if !metadata.is_file() {
            return Err(Error::new(
                ErrorCode::IncludeNotFile,
                "include path is not a file",
            ));
        }
        if metadata.len() as usize > self.options.limits.max_file_size {
            return Err(Error::new(
                ErrorCode::ResourceLimitExceeded,
                "maximum file size exceeded",
            ));
        }
        let source = fs::read_to_string(path).map_err(|err| {
            Error::new(
                ErrorCode::IncludeNotFound,
                format!("failed to read include file: {err}"),
            )
        })?;
        let doc = parser::parse_str(&source, Some(path.to_path_buf())).map_err(|err| Error {
            code: if is_include {
                if err.code == ErrorCode::InvalidRootType {
                    ErrorCode::IncludeRootTypeError
                } else {
                    ErrorCode::IncludeParseError
                }
            } else {
                err.code
            },
            ..err
        })?;
        self.stack.pop();
        self.cache.insert(path.to_path_buf(), doc.clone());
        Ok(doc)
    }

    fn resolve_include(
        &self,
        including_file: Option<&Path>,
        include_path: &str,
        loc: Location,
    ) -> Result<PathBuf> {
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
        let base = including_file
            .and_then(Path::parent)
            .unwrap_or(&self.include_root);
        let candidate = base.join(include_path);
        let canonical = candidate.canonicalize().map_err(|err| {
            Error::new(
                ErrorCode::IncludeNotFound,
                format!("include file not found: {err}"),
            )
            .at(loc.clone())
        })?;
        if !canonical.starts_with(&self.include_root) {
            return Err(Error::new(
                ErrorCode::IncludePathDenied,
                "include path escapes include root",
            )
            .at(loc));
        }
        Ok(canonical)
    }
}

impl IncludeLoader for FileLoader {
    fn load_include(
        &mut self,
        including_file: Option<&Path>,
        path: &str,
        loc: Location,
    ) -> Result<Document> {
        let resolved = self.resolve_include(including_file, path, loc)?;
        self.load_canonical(&resolved, true)
    }
}

fn looks_like_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/'
}
