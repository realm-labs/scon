package io.github.realmlabs.scon

import java.nio.file.Path
import kotlin.io.path.isRegularFile

internal class Resolver(
    private val options: SconResolveOptions,
) {
    private val includeStack = mutableListOf<Path>()
    private val includeSeen = mutableSetOf<Path>()
    private val documentCache = mutableMapOf<Path, ParsedDocument>()

    fun resolve(document: ParsedDocument): SconValue.ObjectValue {
        val evaluator = Evaluator(options, includeStack, includeSeen, documentCache)
        document.sourcePath?.let {
            val canonical = it.toAbsolutePath().normalize()
            includeStack.add(canonical)
            includeSeen.add(canonical)
        }
        try {
            return evaluator.evalDocument(document)
        } finally {
            if (document.sourcePath != null) includeStack.removeLast()
        }
    }
}

private enum class Layer { Base, Local }
private enum class Kind { StructuralObject, OrdinaryValue }

private data class EvalEntry(
    var value: EvalValue,
    var layer: Layer,
    var kind: Kind,
)

private sealed interface EvalValue {
    data object Null : EvalValue
    data class Bool(val value: Boolean) : EvalValue
    data class Number(val value: SconNumber) : EvalValue
    data class StringValue(val value: String) : EvalValue
    data class ArrayValue(val values: List<EvalValue>) : EvalValue
    data class ObjectValue(val values: LinkedHashMap<String, EvalEntry>) : EvalValue
}

private class Evaluator(
    private val options: SconResolveOptions,
    private val includeStack: MutableList<Path>,
    private val includeSeen: MutableSet<Path>,
    private val documentCache: MutableMap<Path, ParsedDocument>,
) {
    private val root = linkedMapOf<String, EvalEntry>()
    private val inProgress = mutableListOf<List<String>>(emptyList())

    fun evalDocument(document: ParsedDocument): SconValue.ObjectValue {
        evalObjectBody(document.root, emptyList(), document.sourcePath)
        return publicObject(root)
    }

    private fun evalObjectBody(obj: AstObject, path: List<String>, sourcePath: Path?) {
        if (path.size > options.limits.maxObjectDepth) {
            throw sconError(SconErrorCode.ResourceLimitExceeded, "maximum object depth exceeded", obj.span)
        }
        var localSeen = false
        for (member in obj.members) {
            when (member) {
                is AstObjectSpread -> {
                    if (localSeen) {
                        throw sconError(SconErrorCode.InvalidSpread, "object spread must appear before local members", member.span)
                    }
                    val target = lookupCompleted(member.substitution.path, member.span)
                    val spreadObject = target.value as? EvalValue.ObjectValue
                        ?: throw sconError(SconErrorCode.TypeMismatch, "object spread target is not an object", member.span)
                    overlayBase(objectAt(path, member.span), cloneObject(spreadObject.values, Layer.Base, Kind.OrdinaryValue))
                }
                is AstInclude -> {
                    val included = loadInclude(sourcePath, member)
                    evalObjectBody(included.root, path, included.sourcePath)
                }
                is AstField -> {
                    localSeen = true
                    evalField(member, path, sourcePath)
                }
            }
        }
    }

    private fun evalField(field: AstField, currentPath: List<String>, sourcePath: Path?) {
        val targetPath = currentPath + field.path.segments.map { it.value }
        when (val value = field.value) {
            is AstObjectValue -> {
                ensureLocalObject(targetPath, field.span)
                inProgress += targetPath
                try {
                    evalObjectBody(value.value, targetPath, sourcePath)
                } finally {
                    inProgress.removeLast()
                }
            }
            else -> {
                insertLocalValue(targetPath, evalValue(value, sourcePath), Kind.OrdinaryValue, field.span)
            }
        }
    }

    private fun evalValue(value: AstValue, sourcePath: Path?): EvalValue =
        when (value) {
            is AstNull -> EvalValue.Null
            is AstBool -> EvalValue.Bool(value.value)
            is AstNumber -> EvalValue.Number(SconNumber.parse(value.raw))
            is AstString -> evalString(value)
            is AstSubstitution -> lookupCompleted(value.path, value.span).value.deepCopy()
            is AstArray -> {
                val out = mutableListOf<EvalValue>()
                for (item in value.items) {
                    if (out.size >= options.limits.maxArrayLength) {
                        throw sconError(SconErrorCode.ResourceLimitExceeded, "maximum array length exceeded", item.span)
                    }
                    when (item) {
                        is AstArrayValueItem -> out += evalValue(item.value, sourcePath)
                        is AstArraySpread -> {
                            val target = lookupCompleted(item.substitution.path, item.span)
                            val values = target.value as? EvalValue.ArrayValue
                                ?: throw sconError(SconErrorCode.TypeMismatch, "array spread target is not an array", item.span)
                            out += values.values.map { it.deepCopy() }
                        }
                    }
                }
                EvalValue.ArrayValue(out)
            }
            is AstObjectValue -> {
                val nested = Evaluator(options, includeStack, includeSeen, documentCache)
                nested.evalObjectBody(value.value, emptyList(), sourcePath)
                EvalValue.ObjectValue(nested.root)
            }
        }

    private fun evalString(value: AstString): EvalValue.StringValue {
        if (value.parts.size == 1 && value.parts.single() is AstStringLiteralPart) {
            return EvalValue.StringValue((value.parts.single() as AstStringLiteralPart).value)
        }
        val out = StringBuilder()
        for (part in value.parts) {
            when (part) {
                is AstStringLiteralPart -> out.append(part.value)
                is AstStringInterpolationPart -> {
                    when (val replacement = lookupCompleted(part.path, part.span).value) {
                        is EvalValue.StringValue -> out.append(replacement.value)
                        is EvalValue.Number -> out.append(replacement.value.toSconString())
                        is EvalValue.Bool -> out.append(replacement.value)
                        else -> throw sconError(
                            SconErrorCode.TypeMismatch,
                            "interpolation requires string, number, or boolean",
                            part.span,
                        )
                    }
                }
            }
        }
        return EvalValue.StringValue(out.toString())
    }

    private fun lookupCompleted(path: AstPath, span: SourceSpan): EvalEntry {
        val names = path.segments.map { it.value }
        if (names in inProgress) {
            throw sconError(SconErrorCode.MissingReference, "reference is not completed yet", span)
        }
        var current = root
        var entry: EvalEntry? = null
        for ((index, name) in names.withIndex()) {
            entry = current[name] ?: throw sconError(SconErrorCode.MissingReference, "missing reference '$name'", span)
            if (index < names.lastIndex) {
                val objectValue = entry.value as? EvalValue.ObjectValue
                    ?: throw sconError(SconErrorCode.TypeMismatch, "reference path crosses non-object value", span)
                current = objectValue.values
            }
        }
        return entry ?: throw sconError(SconErrorCode.MissingReference, "missing reference", span)
    }

    private fun ensureLocalObject(path: List<String>, span: SourceSpan) {
        var current = root
        for ((index, name) in path.withIndex()) {
            val existing = current[name]
            if (existing == null) {
                val child = linkedMapOf<String, EvalEntry>()
                current[name] = EvalEntry(EvalValue.ObjectValue(child), Layer.Local, Kind.StructuralObject)
                current = child
                continue
            }
            val objectValue = existing.value as? EvalValue.ObjectValue
                ?: throw sconError(SconErrorCode.PathConflict, "path conflicts with scalar value", span)
            if (index == path.lastIndex) {
                if (existing.layer == Layer.Local && existing.kind != Kind.StructuralObject) {
                    throw sconError(SconErrorCode.PathConflict, "object field conflicts with ordinary value", span)
                }
                existing.layer = Layer.Local
                existing.kind = Kind.StructuralObject
            }
            current = objectValue.values
        }
    }

    private fun insertLocalValue(path: List<String>, value: EvalValue, kind: Kind, span: SourceSpan) {
        var current = root
        for (name in path.dropLast(1)) {
            val existing = current[name]
            current = if (existing == null) {
                val child = linkedMapOf<String, EvalEntry>()
                current[name] = EvalEntry(EvalValue.ObjectValue(child), Layer.Local, Kind.StructuralObject)
                child
            } else {
                (existing.value as? EvalValue.ObjectValue)?.values
                    ?: throw sconError(SconErrorCode.PathConflict, "path conflicts with scalar value", span)
            }
        }
        val leaf = path.last()
        val existing = current[leaf]
        if (existing == null) {
            current[leaf] = EvalEntry(value, Layer.Local, kind)
            return
        }
        if (existing.layer == Layer.Base) {
            overlayLocal(existing, value, kind, span)
            return
        }
        if (
            existing.kind == Kind.StructuralObject &&
            kind == Kind.StructuralObject &&
                existing.value is EvalValue.ObjectValue &&
                value is EvalValue.ObjectValue
        ) {
            val existingObject = existing.value as EvalValue.ObjectValue
            mergeLocalObjects(existingObject.values, value.values, span)
            return
        }
        throw sconError(SconErrorCode.DuplicateKey, "duplicate key '$leaf'", span)
    }

    private fun overlayLocal(existing: EvalEntry, value: EvalValue, kind: Kind, span: SourceSpan) {
        val existingValue = existing.value
        if (existingValue is EvalValue.ObjectValue && value is EvalValue.ObjectValue) {
            mergeOverride(existingValue.values, value.values)
            existing.layer = Layer.Local
            existing.kind = kind
            return
        }
        existing.value = value
        existing.layer = Layer.Local
        existing.kind = kind
    }

    private fun mergeLocalObjects(
        target: LinkedHashMap<String, EvalEntry>,
        source: LinkedHashMap<String, EvalEntry>,
        span: SourceSpan,
    ) {
        for ((key, entry) in source) {
            val existing = target[key]
            if (existing == null) {
                target[key] = entry.deepCopy()
            } else if (
                existing.kind == Kind.StructuralObject &&
                entry.kind == Kind.StructuralObject &&
                existing.value is EvalValue.ObjectValue &&
                entry.value is EvalValue.ObjectValue
            ) {
                val existingObject = existing.value as EvalValue.ObjectValue
                val entryObject = entry.value as EvalValue.ObjectValue
                mergeLocalObjects(existingObject.values, entryObject.values, span)
            } else {
                throw sconError(SconErrorCode.DuplicateKey, "duplicate key '$key'", span)
            }
        }
    }

    private fun objectAt(path: List<String>, span: SourceSpan): LinkedHashMap<String, EvalEntry> {
        var current = root
        for (name in path) {
            val entry = current[name] ?: throw sconError(SconErrorCode.PathConflict, "target object does not exist", span)
            current = (entry.value as? EvalValue.ObjectValue)?.values
                ?: throw sconError(SconErrorCode.PathConflict, "target path is not an object", span)
        }
        return current
    }

    private fun loadInclude(includingFile: Path?, include: AstInclude): ParsedDocument {
        val includePath = include.path.value
        if (
            includePath.contains("://") ||
            includePath.startsWith("classpath:") ||
            includePath.contains('*') ||
            includePath.startsWith('~') ||
            includePath.startsWith('$') ||
            includePath.startsWith('/') ||
            looksLikeWindowsAbsolute(includePath)
        ) {
            throw sconError(SconErrorCode.InvalidIncludePath, "invalid include path", include.span)
        }
        val includeRoot = options.includeRoot?.toAbsolutePath()?.normalize()
            ?: includingFile?.toAbsolutePath()?.normalize()?.parent
            ?: throw sconError(SconErrorCode.InvalidIncludePath, "includes require a file context", include.span)
        val base = includingFile?.toAbsolutePath()?.normalize()?.parent ?: includeRoot
        val candidate = base.resolve(includePath).normalize()
        val canonical = candidate.toAbsolutePath().normalize()
        if (!canonical.startsWith(includeRoot.toAbsolutePath().normalize())) {
            throw sconError(SconErrorCode.IncludePathDenied, "include path escapes include root", include.span)
        }
        val source = options.sourceStore.readSource(canonical)
            ?: throw sconError(SconErrorCode.IncludeNotFound, "include file not found: $canonical", include.span)
        if (options.sourceStore == FileSconSourceStore && !canonical.isRegularFile()) {
            throw sconError(SconErrorCode.IncludeNotFile, "include path is not a file", include.span)
        }
        if (canonical in includeStack) {
            throw sconError(SconErrorCode.IncludeCycle, "include cycle: $canonical", include.span)
        }
        if (includeStack.size >= options.limits.maxIncludeDepth) {
            throw sconError(SconErrorCode.ResourceLimitExceeded, "maximum include depth exceeded", include.span)
        }
        includeSeen.add(canonical)
        if (includeSeen.size > options.limits.maxIncludeFiles) {
            throw sconError(SconErrorCode.ResourceLimitExceeded, "maximum include file count exceeded", include.span)
        }
        if (source.length > options.limits.maxFileSize) {
            throw sconError(SconErrorCode.ResourceLimitExceeded, "maximum file size exceeded", include.span)
        }
        val cached = documentCache[canonical]
        if (cached != null) return cached
        includeStack.add(canonical)
        try {
            val parsed = try {
                parseSource(source, SconParseOptions(sourceName = canonical.toString(), sourcePath = canonical))
            } catch (err: SconException) {
                val code = if (err.error.code == SconErrorCode.InvalidRootType) {
                    SconErrorCode.IncludeRootTypeError
                } else {
                    SconErrorCode.IncludeParseError
                }
                throw SconException(err.error.copy(code = code))
            }
            documentCache[canonical] = parsed
            return parsed
        } finally {
            includeStack.removeLast()
        }
    }
}

private fun overlayBase(target: LinkedHashMap<String, EvalEntry>, source: LinkedHashMap<String, EvalEntry>) {
    for ((key, entry) in source) {
        val existing = target[key]
        if (existing == null) {
            target[key] = entry.deepCopy()
        } else if (existing.layer == Layer.Base) {
            val existingValue = existing.value
            val entryValue = entry.value
            if (existingValue is EvalValue.ObjectValue && entryValue is EvalValue.ObjectValue) {
                mergeOverride(existingValue.values, entryValue.values)
            } else {
                target[key] = entry.deepCopy()
            }
        }
    }
}

private fun mergeOverride(target: LinkedHashMap<String, EvalEntry>, source: LinkedHashMap<String, EvalEntry>) {
    for ((key, entry) in source) {
        val existing = target[key]
        val existingValue = existing?.value
        val entryValue = entry.value
        if (existingValue is EvalValue.ObjectValue && entryValue is EvalValue.ObjectValue) {
            mergeOverride(existingValue.values, entryValue.values)
        } else {
            target[key] = entry.deepCopy()
        }
    }
}

private fun cloneObject(
    source: LinkedHashMap<String, EvalEntry>,
    layer: Layer,
    kind: Kind,
): LinkedHashMap<String, EvalEntry> =
    linkedMapOf<String, EvalEntry>().also { out ->
        for ((key, entry) in source) {
            val copied = entry.deepCopy()
            copied.layer = layer
            copied.kind = kind
            out[key] = copied
        }
    }

private fun EvalEntry.deepCopy(): EvalEntry =
    EvalEntry(value.deepCopy(), layer, kind)

private fun EvalValue.deepCopy(): EvalValue =
    when (this) {
        EvalValue.Null -> EvalValue.Null
        is EvalValue.Bool -> copy()
        is EvalValue.Number -> copy()
        is EvalValue.StringValue -> copy()
        is EvalValue.ArrayValue -> EvalValue.ArrayValue(values.map { it.deepCopy() })
        is EvalValue.ObjectValue -> EvalValue.ObjectValue(values.mapValuesTo(linkedMapOf()) { it.value.deepCopy() })
    }

private fun publicObject(source: LinkedHashMap<String, EvalEntry>): SconValue.ObjectValue =
    SconValue.ObjectValue(source.mapValuesTo(linkedMapOf()) { publicValue(it.value.value) })

private fun publicValue(value: EvalValue): SconValue =
    when (value) {
        EvalValue.Null -> SconValue.Null
        is EvalValue.Bool -> SconValue.Bool(value.value)
        is EvalValue.Number -> SconValue.Number(value.value)
        is EvalValue.StringValue -> SconValue.StringValue(value.value)
        is EvalValue.ArrayValue -> SconValue.ArrayValue(value.values.map(::publicValue))
        is EvalValue.ObjectValue -> publicObject(value.values)
    }

private fun sconError(code: SconErrorCode, message: String, span: SourceSpan): SconException =
    SconException(SconError(code, message, span))

private fun looksLikeWindowsAbsolute(path: String): Boolean =
    path.length >= 3 && path[1] == ':' && (path[2] == '/' || path[2] == '\\')
