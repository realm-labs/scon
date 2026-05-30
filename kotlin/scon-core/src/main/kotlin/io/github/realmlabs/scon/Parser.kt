package io.github.realmlabs.scon

internal class Parser(
    private val tokens: List<SconToken>,
    private val sourceName: String,
    private val sourcePath: java.nio.file.Path?,
) {
    private var index = 0

    fun parseDocument(): ParsedDocument {
        skipTrivia()
        val root = if (matches(SconTokenKind.LeftBrace)) {
            parseObject(explicitBraces = true, opening = previous())
        } else if (check(SconTokenKind.LeftBracket)) {
            throw error(SconErrorCode.InvalidRootType, "SCON document root must be an object", peek())
        } else {
            parseObjectBody(explicitBraces = false, start = peek().span.start)
        }
        skipTrivia()
        expect(SconTokenKind.Eof, "expected end of file")
        return ParsedDocument(sourceName = sourceName, sourcePath = sourcePath, root = root, tokens = tokens)
    }

    private fun parseObject(explicitBraces: Boolean, opening: SconToken): AstObject {
        val members = mutableListOf<AstObjectMember>()
        skipTrivia()
        while (!check(SconTokenKind.RightBrace) && !check(SconTokenKind.Eof)) {
            members += parseObjectMember()
            skipTrivia()
            if (matches(SconTokenKind.Comma)) {
                skipTrivia()
                if (check(SconTokenKind.Comma)) throw error(SconErrorCode.UnexpectedToken, "consecutive commas are invalid", peek())
            }
        }
        val closing = expect(SconTokenKind.RightBrace, "expected '}'")
        return AstObject(members, SourceSpan(opening.span.start, closing.span.end), explicitBraces)
    }

    private fun parseObjectBody(explicitBraces: Boolean, start: Int): AstObject {
        val members = mutableListOf<AstObjectMember>()
        skipTrivia()
        while (!check(SconTokenKind.Eof) && !check(SconTokenKind.RightBrace)) {
            members += parseObjectMember()
            skipTrivia()
            if (matches(SconTokenKind.Comma)) {
                skipTrivia()
                if (check(SconTokenKind.Comma)) throw error(SconErrorCode.UnexpectedToken, "consecutive commas are invalid", peek())
            }
        }
        val end = members.lastOrNull()?.span?.end ?: start
        return AstObject(members, SourceSpan(start, end), explicitBraces)
    }

    private fun parseObjectMember(): AstObjectMember {
        skipTrivia()
        if (matches(SconTokenKind.Include)) {
            val include = previous()
            skipInlineTrivia()
            val path = parseString()
            if (path.parts.any { it is AstStringInterpolationPart }) {
                throw SconException(
                    SconError(
                        SconErrorCode.UnexpectedToken,
                        "$sourceName: include path must be a literal string",
                        path.span,
                    ),
                )
            }
            return AstInclude(path, SourceSpan(include.span.start, path.span.end))
        }
        if (matches(SconTokenKind.Spread)) {
            val spread = previous()
            val substitution = parseSubstitution()
            return AstObjectSpread(substitution, SourceSpan(spread.span.start, substitution.span.end))
        }
        val path = parsePath()
        skipInlineTrivia()
        val value = if (matches(SconTokenKind.Equals)) {
            skipInlineTrivia()
            if (check(SconTokenKind.Newline)) throw error(SconErrorCode.UnexpectedToken, "field value cannot start on the next line", peek())
            parseValue()
        } else if (matches(SconTokenKind.LeftBrace)) {
            AstObjectValue(parseObject(explicitBraces = true, opening = previous()))
        } else {
            throw error(SconErrorCode.UnexpectedToken, "expected '=' or object shorthand", peek())
        }
        return AstField(path, value, SourceSpan(path.span.start, value.span.end))
    }

    private fun parseValue(): AstValue {
        skipTrivia()
        return when {
            matches(SconTokenKind.Null) -> AstNull(previous().span)
            matches(SconTokenKind.True) -> AstBool(true, previous().span)
            matches(SconTokenKind.False) -> AstBool(false, previous().span)
            matches(SconTokenKind.Number) -> AstNumber(previous().text, previous().span)
            check(SconTokenKind.String) -> parseString()
            matches(SconTokenKind.LeftBrace) -> AstObjectValue(parseObject(explicitBraces = true, opening = previous()))
            matches(SconTokenKind.LeftBracket) -> parseArray(previous())
            check(SconTokenKind.SubstitutionStart) -> parseSubstitution()
            else -> throw error(SconErrorCode.UnexpectedToken, "expected value", peek())
        }
    }

    private fun parseArray(opening: SconToken): AstArray {
        val items = mutableListOf<AstArrayItem>()
        skipTrivia()
        while (!check(SconTokenKind.RightBracket) && !check(SconTokenKind.Eof)) {
            val itemStart = peek().span.start
            items += if (matches(SconTokenKind.Spread)) {
                val substitution = parseSubstitution()
                AstArraySpread(substitution, SourceSpan(itemStart, substitution.span.end))
            } else {
                val value = parseValue()
                AstArrayValueItem(value, value.span)
            }
            skipTrivia()
            if (!matches(SconTokenKind.Comma)) break
            skipTrivia()
            if (check(SconTokenKind.Comma)) throw error(SconErrorCode.UnexpectedToken, "consecutive commas are invalid", peek())
        }
        val closing = expect(SconTokenKind.RightBracket, "expected ']'")
        return AstArray(items, SourceSpan(opening.span.start, closing.span.end))
    }

    private fun parseSubstitution(): AstSubstitution {
        val start = expect(SconTokenKind.SubstitutionStart, "expected '\${'")
        val path = parsePath()
        val end = expect(SconTokenKind.RightBrace, "expected '}'")
        return AstSubstitution(path, SourceSpan(start.span.start, end.span.end))
    }

    private fun parsePath(): AstPath {
        val first = parsePathSegment()
        val segments = mutableListOf(first)
        while (matches(SconTokenKind.Dot)) {
            segments += parsePathSegment()
        }
        return AstPath(segments, SourceSpan(first.span.start, segments.last().span.end))
    }

    private fun parsePathSegment(): AstPathSegment {
        if (matches(SconTokenKind.Identifier)) {
            val token = previous()
            return AstPathSegment(token.text, quoted = false, token.span)
        }
        if (check(SconTokenKind.String)) {
            val string = parseString()
            return AstPathSegment(string.value, quoted = true, string.span)
        }
        throw error(SconErrorCode.UnexpectedToken, "expected path segment", peek())
    }

    private fun parseString(): AstString {
        val token = expect(SconTokenKind.String, "expected string")
        val parts = parseStringParts(token)
        val value = parts.joinToString(separator = "") {
            when (it) {
                is AstStringLiteralPart -> it.value
                is AstStringInterpolationPart -> ""
            }
        }
        return AstString(value, token.text, parts, token.span)
    }

    private fun parseStringParts(token: SconToken): List<AstStringPart> {
        val raw = token.text
        val parts = mutableListOf<AstStringPart>()
        val out = StringBuilder(raw.length)
        var i = 1
        while (i < raw.length - 1) {
            val ch = raw[i++]
            if (ch == '$' && i < raw.length - 1 && raw[i] == '{') {
                if (out.isNotEmpty()) {
                    parts += AstStringLiteralPart(out.toString())
                    out.clear()
                }
                val pathStart = i + 1
                val close = raw.indexOf('}', pathStart)
                if (close < 0) throw error(SconErrorCode.UnterminatedString, "unterminated interpolation", token)
                val path = parseInterpolationPath(raw.substring(pathStart, close), token.span.start + pathStart)
                parts += AstStringInterpolationPart(path, SourceSpan(token.span.start + i - 1, token.span.start + close + 1))
                i = close + 1
                continue
            }
            if (ch != '\\') {
                out.append(ch)
                continue
            }
            when (val escaped = raw[i++]) {
                '"' -> out.append('"')
                '\\' -> out.append('\\')
                '/' -> out.append('/')
                'b' -> out.append('\b')
                'f' -> out.append('\u000C')
                'n' -> out.append('\n')
                'r' -> out.append('\r')
                't' -> out.append('\t')
                '$' -> out.append('$')
                'u' -> {
                    val hex = raw.substring(i, i + 4)
                    out.append(hex.toInt(16).toChar())
                    i += 4
                }
                else -> throw error(SconErrorCode.InvalidEscape, "invalid string escape '$escaped'", token)
            }
        }
        if (out.isNotEmpty() || parts.isEmpty()) {
            parts += AstStringLiteralPart(out.toString())
        }
        return parts
    }

    private fun parseInterpolationPath(text: String, baseOffset: Int): AstPath {
        if (text.startsWith(".") || text.startsWith("?") || text.contains(":-")) {
            throw SconException(
                SconError(
                    SconErrorCode.UnexpectedToken,
                    "$sourceName: invalid substitution path",
                    SourceSpan(baseOffset, (baseOffset + text.length).coerceAtLeast(baseOffset + 1)),
                ),
            )
        }
        val parsed = Parser(Lexer(text, sourceName).lex(), sourceName, sourcePath).parsePathOnly()
        fun shift(segment: AstPathSegment): AstPathSegment =
            segment.copy(span = SourceSpan(segment.span.start + baseOffset, segment.span.end + baseOffset))
        return parsed.copy(
            segments = parsed.segments.map(::shift),
            span = SourceSpan(parsed.span.start + baseOffset, parsed.span.end + baseOffset),
        )
    }

    private fun parsePathOnly(): AstPath {
        skipTrivia()
        val path = parsePath()
        skipTrivia()
        expect(SconTokenKind.Eof, "expected end of substitution path")
        return path
    }

    private fun skipTrivia() {
        while (check(SconTokenKind.Whitespace) || check(SconTokenKind.Newline) || check(SconTokenKind.Comment)) index++
    }

    private fun skipInlineTrivia() {
        while (check(SconTokenKind.Whitespace) || check(SconTokenKind.Comment)) index++
    }

    private fun expect(kind: SconTokenKind, message: String): SconToken {
        if (!matches(kind)) throw error(SconErrorCode.UnexpectedToken, message, peek())
        return previous()
    }

    private fun matches(kind: SconTokenKind): Boolean {
        if (!check(kind)) return false
        index++
        return true
    }

    private fun check(kind: SconTokenKind): Boolean = peek().kind == kind
    private fun peek(): SconToken = tokens[index]
    private fun previous(): SconToken = tokens[index - 1]

    private fun error(code: SconErrorCode, message: String, token: SconToken): SconException =
        SconException(SconError(code, "$sourceName: $message", token.span))
}
