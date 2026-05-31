package io.github.realmlabs.scon

import java.nio.file.Path

public enum class SconDiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

public data class SconDiagnostic(
    val code: SconErrorCode,
    val message: String,
    val severity: SconDiagnosticSeverity,
    val file: Path?,
    val range: SourceRange?,
)

public data class SconSymbol(
    val path: List<String>,
    val file: Path?,
    val range: SourceRange,
)

public enum class SconDefinitionKind {
    Field,
}

public data class SconDefinition(
    val path: List<String>,
    val kind: SconDefinitionKind,
    val file: Path?,
    val range: SourceRange,
)

public enum class SconReferenceKind {
    Substitution,
    Interpolation,
    ObjectSpread,
    ArraySpread,
}

public data class SconReference(
    val path: List<String>,
    val kind: SconReferenceKind,
    val file: Path?,
    val range: SourceRange,
    val target: SconDefinition?,
)

public data class SconIncludeReference(
    val path: String,
    val file: Path?,
    val range: SourceRange,
    val resolvedPath: Path?,
)

public data class SconParsedSource(
    val file: Path?,
    val lineIndex: LineIndex,
    val tokens: List<SconToken>,
    val comments: List<SconComment>,
    val symbols: List<SconSymbol>,
)

public data class SconAnalysis(
    val file: Path?,
    val parsed: SconParsedSource?,
    val diagnostics: List<SconDiagnostic>,
    val comments: List<SconComment>,
    val symbols: List<SconSymbol>,
    val definitions: List<SconDefinition>,
    val references: List<SconReference>,
    val includes: List<SconIncludeReference>,
    val value: SconValue?,
)

public fun parseSourceForAnalysis(
    source: String,
    options: SconParseOptions = SconParseOptions(),
): SconParsedSource {
    val document = parseSource(source, options)
    val lineIndex = LineIndex(source)
    val comments = commentsFromTokens(source, lineIndex, document.tokens)
    val symbols = collectSymbols(document.root, lineIndex, options.sourcePath, emptyList())
    return SconParsedSource(
        file = options.sourcePath,
        lineIndex = lineIndex,
        tokens = document.tokens,
        comments = comments,
        symbols = symbols,
    )
}

public fun analyzeSource(
    source: String,
    parseOptions: SconParseOptions = SconParseOptions(),
    resolveOptions: SconResolveOptions = SconResolveOptions(),
): SconAnalysis {
    val document = try {
        parseSource(source, parseOptions)
    } catch (err: SconException) {
        return parseFailureAnalysis(source, parseOptions.sourcePath, err)
    }
    return analyzeParsedDocument(source, document, parseOptions.sourcePath, resolveOptions)
}

public fun analyzeFile(
    path: Path,
    options: SconResolveOptions = SconResolveOptions(),
): SconAnalysis {
    val source = options.sourceStore.readSource(path)
        ?: return SconAnalysis(
            file = path,
            parsed = null,
            diagnostics = listOf(
                SconDiagnostic(
                    code = SconErrorCode.IncludeNotFound,
                    message = "SCON file not found: $path",
                    severity = SconDiagnosticSeverity.Error,
                    file = path,
                    range = null,
                ),
            ),
            comments = emptyList(),
            symbols = emptyList(),
            definitions = emptyList(),
            references = emptyList(),
            includes = emptyList(),
            value = null,
        )
    return analyzeSource(
        source,
        SconParseOptions(sourceName = path.toString(), sourcePath = path.toAbsolutePath().normalize()),
        options.copy(includeRoot = options.includeRoot ?: path.toAbsolutePath().parent),
    )
}

public fun getPath(value: SconValue, path: String): SconValue {
    var current = value
    val segments = parsePathQuery(path)
    for (segment in segments) {
        current = (current as? SconValue.ObjectValue)?.values?.get(segment)
            ?: throw SconException(
                SconError(
                    code = SconErrorCode.MissingReference,
                    message = "path ${segments.joinToString(".")} is not defined",
                ),
            )
    }
    return current
}

public fun diagnosticFromError(error: SconError, source: String, file: Path? = null): SconDiagnostic {
    val lineIndex = LineIndex(source)
    return SconDiagnostic(
        code = error.code,
        message = error.message,
        severity = SconDiagnosticSeverity.Error,
        file = file,
        range = error.span?.let(lineIndex::sourceRange),
    )
}

private fun parseFailureAnalysis(source: String, file: Path?, err: SconException): SconAnalysis {
    val lineIndex = LineIndex(source)
    val comments = runCatching {
        commentsFromTokens(source, lineIndex, Lexer(source, file?.toString() ?: "<source>").lex())
    }.getOrElse { emptyList() }
    return SconAnalysis(
        file = file,
        parsed = null,
        diagnostics = listOf(diagnosticFromError(err.error, source, file)),
        comments = comments,
        symbols = emptyList(),
        definitions = emptyList(),
        references = emptyList(),
        includes = emptyList(),
        value = null,
    )
}

private fun analyzeParsedDocument(
    source: String,
    document: ParsedDocument,
    file: Path?,
    resolveOptions: SconResolveOptions,
): SconAnalysis {
    val lineIndex = LineIndex(source)
    val comments = commentsFromTokens(source, lineIndex, document.tokens)
    val semantic = SemanticCollector(lineIndex, file)
    semantic.collectObject(document.root, emptyList())
    val symbols = collectSymbols(document.root, lineIndex, file, emptyList())
    val parsed = SconParsedSource(
        file = file,
        lineIndex = lineIndex,
        tokens = document.tokens,
        comments = comments,
        symbols = symbols,
    )
    val (diagnostics, value) = try {
        emptyList<SconDiagnostic>() to resolveDocument(document, resolveOptions)
    } catch (err: SconException) {
        listOf(diagnosticFromError(err.error, source, file)) to null
    }
    return SconAnalysis(
        file = file,
        parsed = parsed,
        diagnostics = diagnostics,
        comments = comments,
        symbols = symbols,
        definitions = semantic.definitions,
        references = semantic.references,
        includes = semantic.includes,
        value = value,
    )
}

private fun collectSymbols(
    obj: AstObject,
    lineIndex: LineIndex,
    file: Path?,
    prefix: List<String>,
): List<SconSymbol> {
    val out = mutableListOf<SconSymbol>()
    for (member in obj.members) {
        val field = member as? AstField ?: continue
        val path = prefix + field.path.segments.map { it.value }
        out += SconSymbol(path, file, lineIndex.sourceRange(field.path.span))
        val objectValue = field.value as? AstObjectValue
        if (objectValue != null) {
            out += collectSymbols(objectValue.value, lineIndex, file, path)
        }
    }
    return out
}

private class SemanticCollector(
    private val lineIndex: LineIndex,
    private val file: Path?,
) {
    private val completed = mutableMapOf<List<String>, SconDefinition>()
    val definitions = mutableListOf<SconDefinition>()
    val references = mutableListOf<SconReference>()
    val includes = mutableListOf<SconIncludeReference>()

    fun collectObject(obj: AstObject, prefix: List<String>) {
        for (member in obj.members) {
            when (member) {
                is AstObjectSpread -> pushReference(
                    member.substitution.path,
                    SconReferenceKind.ObjectSpread,
                    member.substitution.path.span,
                )
                is AstInclude -> includes += SconIncludeReference(
                    path = member.path.value,
                    file = file,
                    range = lineIndex.sourceRange(member.path.span),
                    resolvedPath = resolveIncludePath(file, member.path.value),
                )
                is AstField -> {
                    val path = prefix + member.path.segments.map { it.value }
                    val objectValue = member.value as? AstObjectValue
                    if (objectValue != null) {
                        pushDefinition(path, member.path.span)
                        collectObject(objectValue.value, path)
                    } else {
                        collectValue(member.value)
                        pushDefinition(path, member.path.span)
                    }
                }
            }
        }
    }

    private fun collectValue(value: AstValue) {
        when (value) {
            is AstObjectValue -> collectObject(value.value, emptyList())
            is AstArray -> value.items.forEach {
                when (it) {
                    is AstArrayValueItem -> collectValue(it.value)
                    is AstArraySpread -> pushReference(
                        it.substitution.path,
                        SconReferenceKind.ArraySpread,
                        it.substitution.path.span,
                    )
                }
            }
            is AstString -> value.parts.forEach {
                if (it is AstStringInterpolationPart) {
                    pushReference(it.path, SconReferenceKind.Interpolation, it.path.span)
                }
            }
            is AstSubstitution -> pushReference(value.path, SconReferenceKind.Substitution, value.path.span)
            is AstBool, is AstNull, is AstNumber -> Unit
        }
    }

    private fun pushDefinition(path: List<String>, span: SourceSpan) {
        val definition = SconDefinition(path, SconDefinitionKind.Field, file, lineIndex.sourceRange(span))
        completed[path] = definition
        definitions += definition
    }

    private fun pushReference(path: AstPath, kind: SconReferenceKind, span: SourceSpan) {
        val names = path.segments.map { it.value }
        references += SconReference(names, kind, file, lineIndex.sourceRange(span), completed[names])
    }
}

private fun resolveIncludePath(file: Path?, includePath: String): Path? =
    runCatching {
        validateIncludePath(includePath)
        (file?.parent ?: Path.of(".")).resolve(includePath).normalize()
    }.getOrNull()

private fun validateIncludePath(includePath: String) {
    if (
        includePath.contains("://") ||
        includePath.startsWith("classpath:") ||
        includePath.contains('*') ||
        includePath.startsWith('~') ||
        includePath.startsWith('$') ||
        includePath.startsWith('/') ||
        looksLikeWindowsAbsolute(includePath)
    ) {
        throw IllegalArgumentException("invalid include path")
    }
}

private fun looksLikeWindowsAbsolute(path: String): Boolean =
    path.length >= 3 && path[1] == ':' && (path[2] == '/' || path[2] == '\\')

private fun parsePathQuery(path: String): List<String> {
    val parts = mutableListOf<String>()
    val current = StringBuilder()
    var index = 0
    while (index < path.length) {
        when (val ch = path[index]) {
            '.' -> {
                if (current.isEmpty()) {
                    throw SconException(SconError(SconErrorCode.UnexpectedToken, "empty path segment"))
                }
                parts += current.toString()
                current.clear()
            }
            '"' -> {
                index += 1
                while (index < path.length && path[index] != '"') {
                    if (path[index] == '\\' && index + 1 < path.length) index += 1
                    current.append(path[index])
                    index += 1
                }
            }
            else -> current.append(ch)
        }
        index += 1
    }
    if (current.isNotEmpty()) parts += current.toString()
    if (parts.isEmpty()) throw SconException(SconError(SconErrorCode.UnexpectedToken, "empty path"))
    return parts
}
