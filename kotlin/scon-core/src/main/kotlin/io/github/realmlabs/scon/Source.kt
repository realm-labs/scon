package io.github.realmlabs.scon

public data class SourcePosition(
    val line: Int,
    val character: Int,
    val offset: Int,
)

public data class SourceRange(
    val start: SourcePosition,
    val end: SourcePosition,
)

public enum class SconCommentKind {
    Line,
}

public data class SconComment(
    val kind: SconCommentKind,
    val text: String,
    val range: SourceRange,
)

public class LineIndex(source: String) {
    private val lineStarts: IntArray = buildLineStarts(source)

    public fun sourcePosition(offset: Int): SourcePosition {
        val bounded = offset.coerceAtLeast(0)
        var low = 0
        var high = lineStarts.lastIndex
        while (low <= high) {
            val mid = (low + high) ushr 1
            if (lineStarts[mid] <= bounded) {
                low = mid + 1
            } else {
                high = mid - 1
            }
        }
        val line = high.coerceAtLeast(0)
        return SourcePosition(line, bounded - lineStarts[line], bounded)
    }

    public fun sourceRange(span: SourceSpan): SourceRange =
        SourceRange(sourcePosition(span.start), sourcePosition(span.end))
}

public fun commentsFromTokens(
    source: String,
    lineIndex: LineIndex,
    tokens: List<SconToken>,
): List<SconComment> =
    tokens
        .filter { it.kind == SconTokenKind.Comment }
        .map {
            SconComment(
                kind = SconCommentKind.Line,
                text = source.substring(it.span.start, it.span.end),
                range = lineIndex.sourceRange(it.span),
            )
        }

private fun buildLineStarts(source: String): IntArray {
    val starts = mutableListOf(0)
    var index = 0
    while (index < source.length) {
        when (source[index]) {
            '\n' -> starts += index + 1
            '\r' -> {
                if (index + 1 < source.length && source[index + 1] == '\n') {
                    starts += index + 2
                    index += 1
                } else {
                    starts += index + 1
                }
            }
        }
        index += 1
    }
    return starts.toIntArray()
}
