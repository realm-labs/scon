package io.github.realmlabs.scon

internal class SourceFormatter(
    private val source: String,
    private val options: SconFormatOptions,
) {
    private data class FormatComment(
        val line: Int,
        val text: String,
        val span: SourceSpan,
        var emitted: Boolean = false,
    )

    private val lineIndex = LineIndex(source)
    private val comments = Lexer(source, "<format>").lex()
        .filter { it.kind == SconTokenKind.Comment }
        .map {
            FormatComment(
                line = lineIndex.sourcePosition(it.span.start).line,
                text = source.substring(it.span.start, it.span.end),
                span = it.span,
            )
        }

    fun format(document: ParsedDocument): String =
        buildString {
            emitObjectBody(document.root, 0, this)
            emitCommentsBefore(document.root.span.end, 0, this)
            if (!endsWith('\n')) append('\n')
        }

    private fun emitObjectBody(obj: AstObject, indent: Int, out: StringBuilder) {
        val members = obj.members.sortedBy { it.span.start }
        for ((index, member) in members.withIndex()) {
            val nextMemberStart = members.getOrNull(index + 1)?.span?.start ?: obj.span.end
            emitCommentsBefore(member.span.start, indent, out)
            when (member) {
                is AstField -> emitField(member, indent, out, nextMemberStart)
                is AstInclude -> {
                    writeIndent(out, indent)
                    out.append("include ")
                    out.append(formatStringLiteral(member.path.value))
                    emitInlineComment(member.span, out)
                    out.append('\n')
                }
                is AstObjectSpread -> {
                    writeIndent(out, indent)
                    out.append("...")
                    out.append(formatSubstitution(member.substitution.path))
                    emitInlineComment(member.span, out)
                    out.append('\n')
                }
            }
        }
        emitCommentsBefore(obj.span.end, indent, out)
    }

    private fun emitField(field: AstField, indent: Int, out: StringBuilder, nextMemberStart: Int) {
        writeIndent(out, indent)
        out.append(formatPath(field.path))
        val objectValue = field.value as? AstObjectValue
        if (objectValue != null) {
            if (fieldUsesEquals(field)) {
                out.append(" = {")
            } else {
                out.append(" {")
            }
            emitInlineComment(
                SourceSpan(field.span.start, objectValue.span.start + 1),
                out,
                beforeOffset = objectValue.value.members.minOfOrNull { it.span.start } ?: objectValue.span.end,
            )
            out.append('\n')
            emitObjectBody(objectValue.value, indent + options.indent, out)
            writeIndent(out, indent)
            out.append('}')
            emitInlineComment(field.span, out)
            out.append('\n')
            return
        }
        out.append(" = ")
        emitValue(field.value, indent, out)
        emitInlineComment(field.span, out, beforeOffset = nextMemberStart)
        out.append('\n')
    }

    private fun emitValue(value: AstValue, indent: Int, out: StringBuilder) {
        when (value) {
            is AstObjectValue -> {
                out.append("{\n")
                emitObjectBody(value.value, indent + options.indent, out)
                writeIndent(out, indent)
                out.append('}')
            }
            is AstArray -> emitArray(value, indent, out)
            is AstSubstitution -> out.append(formatSubstitution(value.path))
            is AstString, is AstNumber, is AstBool, is AstNull -> out.append(spanText(value.span).trim())
        }
    }

    private fun emitArray(array: AstArray, indent: Int, out: StringBuilder) {
        if (array.items.isEmpty()) {
            out.append("[]")
            return
        }
        out.append("[\n")
        for ((index, item) in array.items.withIndex()) {
            emitCommentsBefore(item.span.start, indent + options.indent, out)
            writeIndent(out, indent + options.indent)
            when (item) {
                is AstArrayValueItem -> emitValue(item.value, indent + options.indent, out)
                is AstArraySpread -> {
                    out.append("...")
                    out.append(formatSubstitution(item.substitution.path))
                }
            }
            if (index + 1 != array.items.size) out.append(',')
            emitInlineComment(item.span, out)
            out.append('\n')
        }
        emitCommentsBefore(array.span.end, indent + options.indent, out)
        writeIndent(out, indent)
        out.append(']')
    }

    private fun emitCommentsBefore(offset: Int, indent: Int, out: StringBuilder) {
        for (comment in comments) {
            if (comment.emitted || comment.span.start >= offset) continue
            writeIndent(out, indent)
            out.append(comment.text.trim())
            out.append('\n')
            comment.emitted = true
        }
    }

    private fun emitInlineComment(
        span: SourceSpan,
        out: StringBuilder,
        beforeOffset: Int = Int.MAX_VALUE,
    ) {
        val line = lineIndex.sourcePosition(span.start).line
        val comment = comments.firstOrNull {
            !it.emitted &&
                it.line == line &&
                it.span.start >= span.start &&
                it.span.start >= span.end &&
                it.span.start < beforeOffset
        } ?: return
        out.append(' ')
        out.append(comment.text.trim())
        comment.emitted = true
    }

    private fun fieldUsesEquals(field: AstField): Boolean =
        source.substring(field.path.span.end, field.value.span.start).contains('=')

    private fun spanText(span: SourceSpan): String =
        source.substring(span.start.coerceIn(0, source.length), span.end.coerceIn(0, source.length))

    private fun writeIndent(out: StringBuilder, indent: Int) {
        repeat(indent) { out.append(' ') }
    }
}

internal fun formatSubstitution(path: AstPath): String =
    "\${${formatPath(path)}}"

internal fun formatPath(path: AstPath): String =
    path.segments.joinToString(".") { segment ->
        if (segment.value.isSconIdentifier()) segment.value else formatStringLiteral(segment.value)
    }

internal fun formatStringLiteral(text: String): String =
    buildString {
        append('"')
        for (ch in text) {
            when (ch) {
                '"' -> append("\\\"")
                '\\' -> append("\\\\")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                '\b' -> append("\\b")
                '\u000C' -> append("\\f")
                else -> {
                    if (ch.isISOControl()) {
                        append("\\u")
                        append(ch.code.toString(16).padStart(4, '0'))
                    } else {
                        append(ch)
                    }
                }
            }
        }
        append('"')
    }

internal fun String.isSconIdentifier(): Boolean {
    if (isEmpty()) return false
    val first = first()
    if (!first.isAsciiLetter() && first != '_') return false
    return drop(1).all { it.isAsciiLetter() || it.isDigit() || it == '_' || it == '-' }
}

internal fun Char.isAsciiLetter(): Boolean =
    this in 'a'..'z' || this in 'A'..'Z'
