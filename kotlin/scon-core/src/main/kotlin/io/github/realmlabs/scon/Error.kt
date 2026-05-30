package io.github.realmlabs.scon

public enum class SconErrorCode {
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

public data class SconError(
    val code: SconErrorCode,
    val message: String,
    val span: SourceSpan? = null,
)

public class SconException(
    public val error: SconError,
    cause: Throwable? = null,
) : RuntimeException(error.message, cause)
