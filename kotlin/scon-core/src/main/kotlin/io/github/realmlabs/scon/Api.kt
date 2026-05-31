package io.github.realmlabs.scon

import java.nio.file.Files
import java.nio.file.Path

public interface SconSourceStore {
    public fun readSource(path: Path): String?
}

public object FileSconSourceStore : SconSourceStore {
    override fun readSource(path: Path): String? =
        if (Files.exists(path)) Files.readString(path) else null
}

public data class SconParseOptions(
    val sourceName: String = "<source>",
    val preserveTrivia: Boolean = true,
    val sourcePath: Path? = null,
)

public data class SconResolveOptions(
    val includeRoot: Path? = null,
    val limits: SconLimits = SconLimits(),
    val sourceStore: SconSourceStore = FileSconSourceStore,
)

public data class SconLimits(
    val maxFileSize: Int = 16 * 1024 * 1024,
    val maxIncludeDepth: Int = 64,
    val maxIncludeFiles: Int = 1024,
    val maxArrayLength: Int = 1_000_000,
    val maxObjectDepth: Int = 512,
)

public data class SconFormatOptions(
    val indent: Int = 2,
)

public fun parseSource(
    source: String,
    options: SconParseOptions = SconParseOptions(),
): ParsedDocument {
    val lexer = Lexer(source, options.sourceName)
    val tokens = lexer.lex()
    return Parser(tokens, options.sourceName, options.sourcePath).parseDocument()
}

public fun resolveSource(
    source: String,
    options: SconResolveOptions = SconResolveOptions(),
): SconValue {
    val document = parseSource(source)
    return Resolver(options).resolve(document)
}

public fun parseValue(
    source: String,
    options: SconResolveOptions = SconResolveOptions(),
): SconValue =
    resolveSource(source, options)

public fun resolveDocument(
    document: ParsedDocument,
    options: SconResolveOptions = SconResolveOptions(),
): SconValue.ObjectValue =
    Resolver(options).resolve(document)

public fun resolveFile(
    path: Path,
    options: SconResolveOptions = SconResolveOptions(),
): SconValue {
    val source = options.sourceStore.readSource(path)
        ?: throw SconException(
            SconError(
                code = SconErrorCode.IncludeNotFound,
                message = "SCON file not found: $path",
                span = null,
            ),
        )
    if (source.length > options.limits.maxFileSize) {
        throw SconException(
            SconError(
                code = SconErrorCode.ResourceLimitExceeded,
                message = "SCON file exceeds maxFileSize",
                span = null,
            ),
        )
    }
    val includeRoot = options.includeRoot ?: path.toAbsolutePath().parent
    val document = parseSource(
        source,
        SconParseOptions(sourceName = path.toString(), sourcePath = path.toAbsolutePath().normalize()),
    )
    return Resolver(options.copy(includeRoot = includeRoot)).resolve(document)
}

public fun parseValueFile(
    path: Path,
    options: SconResolveOptions = SconResolveOptions(),
): SconValue =
    resolveFile(path, options)

public fun formatSource(
    source: String,
    options: SconFormatOptions = SconFormatOptions(),
): String {
    val document = parseSource(source)
    return SourceFormatter(source, options).format(document)
}
