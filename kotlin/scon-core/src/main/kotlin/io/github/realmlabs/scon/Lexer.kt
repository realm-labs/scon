package io.github.realmlabs.scon

internal class Lexer(
    private val source: String,
    private val sourceName: String,
) {
    private val tokens = mutableListOf<SconToken>()
    private var index = 0

    fun lex(): List<SconToken> {
        while (!isAtEnd()) {
            val start = index
            when (val ch = source[index]) {
                ' ', '\t' -> lexHorizontalWhitespace()
                '\n' -> add(SconTokenKind.Newline, start, ++index)
                '\r' -> lexCarriageReturn(start)
                '#' -> lexLineComment()
                '/' -> lexSlashOrError()
                '"' -> lexString()
                '$' -> lexDollarOrError()
                '{' -> add(SconTokenKind.LeftBrace, start, ++index)
                '}' -> add(SconTokenKind.RightBrace, start, ++index)
                '[' -> add(SconTokenKind.LeftBracket, start, ++index)
                ']' -> add(SconTokenKind.RightBracket, start, ++index)
                '=' -> add(SconTokenKind.Equals, start, ++index)
                '.' -> lexDotOrSpread()
                ',' -> add(SconTokenKind.Comma, start, ++index)
                '-' -> lexNumberOrError()
                '?', ':' -> throw lexError(SconErrorCode.UnexpectedToken, "unexpected character '$ch'", start)
                else -> when {
                    ch.isAsciiDigit() -> lexNumber()
                    ch.isIdentifierStart() -> lexIdentifier()
                    ch.isWhitespace() -> throw lexError(SconErrorCode.InvalidWhitespace, "invalid whitespace outside strings", start)
                    else -> throw lexError(SconErrorCode.InvalidCharacter, "unexpected character '$ch'", start)
                }
            }
        }
        tokens += SconToken(SconTokenKind.Eof, "", SourceSpan(source.length, source.length))
        return tokens
    }

    private fun lexHorizontalWhitespace() {
        val start = index
        while (!isAtEnd() && (source[index] == ' ' || source[index] == '\t')) index++
        add(SconTokenKind.Whitespace, start, index)
    }

    private fun lexCarriageReturn(start: Int) {
        if (peek(1) == '\n') {
            index += 2
            add(SconTokenKind.Newline, start, index)
        } else {
            throw lexError(SconErrorCode.InvalidCharacter, "standalone CR is invalid", start)
        }
    }

    private fun lexLineComment() {
        val start = index
        index++
        while (!isAtEnd() && source[index] != '\n' && source[index] != '\r') index++
        add(SconTokenKind.Comment, start, index)
    }

    private fun lexSlashOrError() {
        val start = index
        if (peek(1) != '/') throw lexError(SconErrorCode.InvalidCharacter, "unexpected character '/'", start)
        index += 2
        while (!isAtEnd() && source[index] != '\n' && source[index] != '\r') index++
        add(SconTokenKind.Comment, start, index)
    }

    private fun lexDollarOrError() {
        val start = index
        if (peek(1) != '{') throw lexError(SconErrorCode.InvalidCharacter, "unexpected character '$'", start)
        index += 2
        add(SconTokenKind.SubstitutionStart, start, index)
    }

    private fun lexDotOrSpread() {
        val start = index
        if (peek(1) == '.' && peek(2) == '.') {
            index += 3
            add(SconTokenKind.Spread, start, index)
        } else {
            index++
            add(SconTokenKind.Dot, start, index)
        }
    }

    private fun lexIdentifier() {
        val start = index
        index++
        while (!isAtEnd() && source[index].isIdentifierPart()) index++
        val text = source.substring(start, index)
        val kind = when (text) {
            "true" -> SconTokenKind.True
            "false" -> SconTokenKind.False
            "null" -> SconTokenKind.Null
            "include" -> SconTokenKind.Include
            else -> SconTokenKind.Identifier
        }
        add(kind, start, index)
    }

    private fun lexNumberOrError() {
        val start = index
        if (!peek(1).isAsciiDigit()) throw lexError(SconErrorCode.UnexpectedToken, "expected digit after '-'", start)
        lexNumber()
    }

    private fun lexNumber() {
        val start = index
        if (source[index] == '-') index++
        if (peek() == '0') {
            index++
            if (peek().isAsciiDigit()) throw lexError(SconErrorCode.InvalidNumber, "leading zeroes are invalid", start)
        } else {
            if (!peek().isAsciiDigitNonZero()) throw lexError(SconErrorCode.InvalidNumber, "invalid number", start)
            while (peek().isAsciiDigit()) index++
        }
        if (peek() == '.') {
            index++
            if (!peek().isAsciiDigit()) throw lexError(SconErrorCode.InvalidNumber, "expected digit after decimal point", index)
            while (peek().isAsciiDigit()) index++
        }
        if (peek() == 'e' || peek() == 'E') {
            index++
            if (peek() == '+' || peek() == '-') index++
            if (!peek().isAsciiDigit()) throw lexError(SconErrorCode.InvalidNumber, "expected exponent digit", index)
            while (peek().isAsciiDigit()) index++
        }
        add(SconTokenKind.Number, start, index)
    }

    private fun lexString() {
        val start = index
        index++
        while (!isAtEnd()) {
            when (source[index]) {
                '"' -> {
                    index++
                    add(SconTokenKind.String, start, index)
                    return
                }
                '\n', '\r' -> throw lexError(SconErrorCode.UnterminatedString, "raw multiline strings are invalid", index)
                '\\' -> {
                    index++
                    if (isAtEnd()) throw lexError(SconErrorCode.UnterminatedString, "unterminated string escape", index)
                    when (source[index]) {
                        '"', '\\', '/', 'b', 'f', 'n', 'r', 't', '$' -> index++
                        'u' -> {
                            index++
                            repeat(4) {
                                if (!peek().isHexDigit()) throw lexError(SconErrorCode.InvalidEscape, "invalid unicode escape", index)
                                index++
                            }
                        }
                        else -> throw lexError(SconErrorCode.InvalidEscape, "invalid string escape", index - 1)
                    }
                }
                else -> index++
            }
        }
        throw lexError(SconErrorCode.UnterminatedString, "unterminated string", start)
    }

    private fun add(kind: SconTokenKind, start: Int, end: Int) {
        tokens += SconToken(kind, source.substring(start, end), SourceSpan(start, end))
    }

    private fun lexError(code: SconErrorCode, message: String, start: Int): SconException =
        SconException(
            SconError(
                code = code,
                message = "$sourceName: $message",
                span = SourceSpan(start, (start + 1).coerceAtMost(source.length)),
            ),
        )

    private fun peek(offset: Int = 0): Char? =
        source.getOrNull(index + offset)

    private fun isAtEnd(): Boolean = index >= source.length
}

private fun Char?.isAsciiDigit(): Boolean = this != null && this in '0'..'9'
private fun Char?.isAsciiDigitNonZero(): Boolean = this != null && this in '1'..'9'
private fun Char?.isHexDigit(): Boolean = this != null && (this in '0'..'9' || this in 'a'..'f' || this in 'A'..'F')
private fun Char.isIdentifierStart(): Boolean = this in 'A'..'Z' || this in 'a'..'z' || this == '_'
private fun Char.isIdentifierPart(): Boolean = isIdentifierStart() || this in '0'..'9' || this == '-'
