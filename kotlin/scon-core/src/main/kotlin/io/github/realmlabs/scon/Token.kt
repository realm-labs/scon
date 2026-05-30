package io.github.realmlabs.scon

public enum class SconTokenKind {
    Identifier,
    String,
    Number,
    True,
    False,
    Null,
    Include,
    SubstitutionStart,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Equals,
    Dot,
    Comma,
    Spread,
    Comment,
    Newline,
    Whitespace,
    Eof,
}

public data class SconToken(
    val kind: SconTokenKind,
    val text: String,
    val span: SourceSpan,
)
