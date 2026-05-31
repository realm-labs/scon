package io.github.realmlabs.scon

public fun SconValue.toSconString(): String {
    val root = this as? SconValue.ObjectValue
        ?: throw SconException(
            SconError(
                code = SconErrorCode.InvalidRootType,
                message = "SCON document root must be an object",
            ),
        )
    return buildString {
        appendObjectBody(root.values, 0)
        append('\n')
    }
}

private fun StringBuilder.appendObjectBody(
    objectValue: LinkedHashMap<String, SconValue>,
    indent: Int,
) {
    for ((key, value) in objectValue) {
        appendIndent(indent)
        append(formatKey(key))
        append(" = ")
        appendSconValue(value, indent)
        append('\n')
    }
}

private fun StringBuilder.appendSconValue(value: SconValue, indent: Int) {
    when (value) {
        SconValue.Null -> append("null")
        is SconValue.Bool -> append(value.value)
        is SconValue.Number -> append(value.value.toSconString())
        is SconValue.StringValue -> appendSconString(value.value)
        is SconValue.ArrayValue -> {
            if (value.values.isEmpty()) {
                append("[]")
                return
            }
            append("[\n")
            for (item in value.values) {
                appendIndent(indent + 2)
                appendSconValue(item, indent + 2)
                append(",\n")
            }
            appendIndent(indent)
            append(']')
        }
        is SconValue.ObjectValue -> {
            if (value.values.isEmpty()) {
                append("{}")
                return
            }
            append("{\n")
            appendObjectBody(value.values, indent + 2)
            appendIndent(indent)
            append('}')
        }
    }
}

private fun StringBuilder.appendIndent(indent: Int) {
    repeat(indent) {
        append(' ')
    }
}

private fun StringBuilder.appendSconString(value: String) {
    append('"')
    var index = 0
    while (index < value.length) {
        val ch = value[index]
        when (ch) {
            '"' -> append("\\\"")
            '\\' -> append("\\\\")
            '\n' -> append("\\n")
            '\r' -> append("\\r")
            '\t' -> append("\\t")
            '\b' -> append("\\b")
            '\u000C' -> append("\\f")
            '$' -> {
                if (index + 1 < value.length && value[index + 1] == '{') {
                    append("\\$")
                } else {
                    append(ch)
                }
            }
            else -> {
                if (ch.isISOControl()) {
                    append("\\u")
                    append(ch.code.toString(16).uppercase().padStart(4, '0'))
                } else {
                    append(ch)
                }
            }
        }
        index += 1
    }
    append('"')
}

private fun formatKey(key: String): String =
    if (key.isSconIdentifier()) {
        key
    } else {
        buildString { appendSconString(key) }
    }
