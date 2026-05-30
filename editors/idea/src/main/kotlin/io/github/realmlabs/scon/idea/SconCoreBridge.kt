package io.github.realmlabs.scon.idea

import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiFile
import io.github.realmlabs.scon.AstArray
import io.github.realmlabs.scon.AstArraySpread
import io.github.realmlabs.scon.AstArrayValueItem
import io.github.realmlabs.scon.AstBool
import io.github.realmlabs.scon.AstField
import io.github.realmlabs.scon.AstInclude
import io.github.realmlabs.scon.AstNode
import io.github.realmlabs.scon.AstNull
import io.github.realmlabs.scon.AstNumber
import io.github.realmlabs.scon.AstObject
import io.github.realmlabs.scon.AstObjectSpread
import io.github.realmlabs.scon.AstObjectValue
import io.github.realmlabs.scon.AstPath
import io.github.realmlabs.scon.AstString
import io.github.realmlabs.scon.AstStringInterpolationPart
import io.github.realmlabs.scon.AstStringLiteralPart
import io.github.realmlabs.scon.AstSubstitution
import io.github.realmlabs.scon.AstValue
import io.github.realmlabs.scon.ParsedDocument
import io.github.realmlabs.scon.SconParseOptions
import io.github.realmlabs.scon.SconResolveOptions
import io.github.realmlabs.scon.SconValue
import io.github.realmlabs.scon.SourceSpan
import io.github.realmlabs.scon.parseSource
import io.github.realmlabs.scon.resolveDocument
import java.nio.file.Path

internal fun PsiFile.sconSourcePath(): Path? =
    virtualFile?.toNioPath()

internal fun PsiFile.parseSconDocument(): ParsedDocument =
    parseSource(
        text,
        SconParseOptions(
            sourceName = virtualFile?.path ?: name,
            sourcePath = sconSourcePath(),
        ),
    )

internal fun PsiFile.resolveSconDocument(document: ParsedDocument = parseSconDocument()): SconValue.ObjectValue =
    resolveDocument(
        document,
        SconResolveOptions(includeRoot = sconSourcePath()?.parent),
    )

internal fun SourceSpan.toTextRange(file: PsiFile): TextRange =
    TextRange(start.coerceIn(0, file.textLength), end.coerceIn(0, file.textLength))

internal fun ParsedDocument.collectDefinitionPaths(): List<SconDefinition> {
    val out = mutableListOf<SconDefinition>()
    collectObjectDefinitions(root, emptyList(), out)
    return out
}

internal data class SconDefinition(
    val path: List<String>,
    val span: SourceSpan,
) {
    val dotted: String = path.joinToString(".")
}

private fun collectObjectDefinitions(
    obj: AstObject,
    prefix: List<String>,
    out: MutableList<SconDefinition>,
) {
    for (member in obj.members) {
        when (member) {
            is AstField -> {
                val path = prefix + member.path.segments.map { it.value }
                out += SconDefinition(path, member.path.span)
                val objectValue = member.value as? AstObjectValue
                if (objectValue != null) collectObjectDefinitions(objectValue.value, path, out)
                collectValueDefinitions(member.value, path, out)
            }
            is AstInclude, is AstObjectSpread -> Unit
        }
    }
}

private fun collectValueDefinitions(value: AstValue, prefix: List<String>, out: MutableList<SconDefinition>) {
    when (value) {
        is AstObjectValue -> collectObjectDefinitions(value.value, prefix, out)
        is AstArray -> value.items.forEach {
            when (it) {
                is AstArrayValueItem -> collectValueDefinitions(it.value, prefix, out)
                is AstArraySpread -> Unit
            }
        }
        is AstString, is AstSubstitution, is AstNull, is AstBool, is AstNumber -> Unit
    }
}

internal fun ParsedDocument.findDefinition(path: String): SconDefinition? =
    collectDefinitionPaths().firstOrNull { it.dotted == path }

internal fun pathAtOffsetInSubstitution(text: String, offset: Int): String? {
    val start = text.lastIndexOf("\${", (offset - 1).coerceAtLeast(0))
    if (start < 0) return null
    val end = text.indexOf('}', start + 2).let { if (it < 0) text.length else it }
    if (offset !in (start + 2)..end) return null
    return text.substring(start + 2, end).takeIf { it.matches(Regex("[A-Za-z_][A-Za-z0-9_-]*(\\.[A-Za-z_][A-Za-z0-9_-]*)*")) }
}

internal fun pathPrefixAtOffsetInSubstitution(text: String, offset: Int): String? {
    val start = text.lastIndexOf("\${", (offset - 1).coerceAtLeast(0))
    if (start < 0 || offset < start + 2) return null
    val closeBefore = text.lastIndexOf('}', (offset - 1).coerceAtLeast(0))
    if (closeBefore > start) return null
    return text.substring(start + 2, offset).takeIf { it.all { ch -> ch.isLetterOrDigit() || ch == '_' || ch == '-' || ch == '.' } }
}

internal fun SconValue.preview(): String =
    when (this) {
        SconValue.Null -> "null"
        is SconValue.Bool -> value.toString()
        is SconValue.Number -> value.toSconString()
        is SconValue.StringValue -> "\"${value.take(80)}\""
        is SconValue.ArrayValue -> "array[${values.size}]"
        is SconValue.ObjectValue -> "object{${values.size}}"
    }

internal fun SconValue.typeName(): String =
    when (this) {
        SconValue.Null -> "null"
        is SconValue.Bool -> "bool"
        is SconValue.Number -> "number"
        is SconValue.StringValue -> "string"
        is SconValue.ArrayValue -> "array"
        is SconValue.ObjectValue -> "object"
    }

internal fun SconValue.getPath(path: String): SconValue? {
    var current: SconValue = this
    for (segment in path.split('.')) {
        current = (current as? SconValue.ObjectValue)?.values?.get(segment) ?: return null
    }
    return current
}
