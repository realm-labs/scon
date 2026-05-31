namespace RealmLabs.Scon;

public enum ErrorCode
{
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

public readonly record struct Span(int Start, int End);

public sealed class SconException : Exception
{
    public SconException(ErrorCode code, string message, Span? span = null) : base($"{code}: {message}")
    {
        Code = code;
        Span = span;
    }

    public ErrorCode Code { get; }
    public Span? Span { get; }
}
