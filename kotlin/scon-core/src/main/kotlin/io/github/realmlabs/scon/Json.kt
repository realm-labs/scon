package io.github.realmlabs.scon

public fun SconValue.toJsonString(): String =
    buildString { appendJsonValue(this@toJsonString) }

private fun StringBuilder.appendJsonValue(value: SconValue) {
    when (value) {
        SconValue.Null -> append("null")
        is SconValue.Bool -> append(value.value)
        is SconValue.Number -> append(value.value.toSconString())
        is SconValue.StringValue -> appendJsonString(value.value)
        is SconValue.ArrayValue -> {
            append('[')
            value.values.forEachIndexed { index, item ->
                if (index > 0) append(',')
                appendJsonValue(item)
            }
            append(']')
        }
        is SconValue.ObjectValue -> {
            append('{')
            value.values.entries.forEachIndexed { index, entry ->
                if (index > 0) append(',')
                appendJsonString(entry.key)
                append(':')
                appendJsonValue(entry.value)
            }
            append('}')
        }
    }
}

private fun StringBuilder.appendJsonString(value: String) {
    append('"')
    for (ch in value) {
        when (ch) {
            '"' -> append("\\\"")
            '\\' -> append("\\\\")
            '\b' -> append("\\b")
            '\u000C' -> append("\\f")
            '\n' -> append("\\n")
            '\r' -> append("\\r")
            '\t' -> append("\\t")
            else -> {
                if (ch < ' ') {
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
