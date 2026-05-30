use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    InvalidCharacter,
    InvalidWhitespace,
    InvalidEscape,
    UnexpectedToken,
    UnterminatedString,
    InvalidNumber,
    InvalidRootType,
    DuplicateKey,
    PathConflict,
    MissingReference,
    TypeMismatch,
    InvalidSpread,
    InvalidIncludePath,
    IncludeNotFound,
    IncludeNotFile,
    IncludePathDenied,
    IncludeCycle,
    IncludeParseError,
    IncludeRootTypeError,
    ResourceLimitExceeded,
    Serde,
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("{code:?} at {line}:{column}: {message}")]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: usize,
    pub column: usize,
    pub path: Option<Vec<String>>,
    pub include_stack: Vec<PathBuf>,
    pub hint: Option<String>,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            file: None,
            line: 1,
            column: 1,
            path: None,
            include_stack: Vec::new(),
            hint: None,
        }
    }

    pub(crate) fn at(mut self, loc: crate::ast::Location) -> Self {
        self.file = loc.file;
        self.line = loc.line;
        self.column = loc.column;
        self
    }

    pub(crate) fn with_path(mut self, path: &[String]) -> Self {
        self.path = Some(path.to_vec());
        self
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::new(ErrorCode::Serde, msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::new(ErrorCode::Serde, msg.to_string())
    }
}
