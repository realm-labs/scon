package io.github.realmlabs.scon

import java.nio.file.Files
import java.nio.file.Path

public data class SconParseOptions(
    val sourceName: String = "<source>",
    val preserveTrivia: Boolean = true,
    val sourcePath: Path? = null,
)

public data class SconResolveOptions(
    val includeRoot: Path? = null,
    val maxFileSize: Int? = 1024 * 1024,
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
    if (options.maxFileSize != null && Files.size(path) > options.maxFileSize) {
        throw SconException(
            SconError(
                code = SconErrorCode.ResourceLimitExceeded,
                message = "SCON file exceeds maxFileSize",
                span = null,
            ),
        )
    }
    val source = Files.readString(path)
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
    options: SconParseOptions = SconParseOptions(),
): String {
    parseSource(source, options)
    return if (source.endsWith("\n")) source.replace("\r\n", "\n") else source.replace("\r\n", "\n") + "\n"
}
