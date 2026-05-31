package io.github.realmlabs.scon;

public enum ErrorCode {
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
    Serde
}
