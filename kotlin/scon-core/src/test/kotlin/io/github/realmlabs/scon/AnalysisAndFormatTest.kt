package io.github.realmlabs.scon

import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class AnalysisAndFormatTest {
    @Test
    fun analyzesSymbolsDefinitionsReferencesIncludesAndResolvedValue() {
        val source = """
            defaults {
              port = 8080
            }
            server {
              ...${'$'}{defaults}
              host = "127.0.0.1"
            }
            message = "port=${'$'}{defaults.port}"
            include "./extra.scon"
        """.trimIndent()

        val analysis = analyzeSource(
            source,
            SconParseOptions(sourcePath = Path.of("/workspace/app.scon")),
            SconResolveOptions(
                includeRoot = Path.of("/workspace"),
                sourceStore = object : SconSourceStore {
                    override fun readSource(path: Path): String? =
                        if (path.toAbsolutePath().normalize() == Path.of("/workspace/extra.scon")) {
                            "extra = true"
                        } else {
                            null
                        }
                },
            ),
        )

        assertEquals(emptyList(), analysis.diagnostics)
        assertNotNull(analysis.value)
        assertTrue(analysis.symbols.any { it.path == listOf("defaults") })
        assertTrue(analysis.definitions.any { it.path == listOf("defaults", "port") })
        assertTrue(analysis.references.any { it.path == listOf("defaults") && it.kind == SconReferenceKind.ObjectSpread })
        assertTrue(analysis.references.any { it.path == listOf("defaults", "port") && it.kind == SconReferenceKind.Interpolation })
        assertEquals(Path.of("/workspace/extra.scon"), analysis.includes.single().resolvedPath)
    }

    @Test
    fun formatsSourceWhilePreservingCommentsAndCompositionSyntax() {
        val source = """
            # root
            defaults{ # inline
            port=8080
            }
            server={host="127.0.0.1",ports=[ # ports
            ...${'$'}{base_ports},9090]}
            include "./extra.scon"
        """.trimIndent()

        val formatted = formatSource(source)

        assertEquals(
            "# root\n" +
                "defaults { # inline\n" +
                "  port = 8080\n" +
                "}\n" +
                "server = {\n" +
                "  host = \"127.0.0.1\"\n" +
                "  ports = [\n" +
                "    # ports\n" +
                "    ...${'$'}{base_ports},\n" +
                "    9090\n" +
                "  ]\n" +
                "}\n" +
                "include \"./extra.scon\"\n",
            formatted,
        )
        parseSource(formatted)
    }

    @Test
    fun supportsSourceStoreForUnsavedIncludesAndLimitsIncludeCount() {
        val files = mapOf(
            Path.of("/workspace/app.scon") to """
                include "./base.scon"
                name = ${'$'}{base.name}
            """.trimIndent(),
            Path.of("/workspace/base.scon") to """
                base.name = "demo"
            """.trimIndent(),
        )
        val store = object : SconSourceStore {
            override fun readSource(path: Path): String? =
                files[path.toAbsolutePath().normalize()]
        }
        val options = SconResolveOptions(
            includeRoot = Path.of("/workspace"),
            sourceStore = store,
        )

        val analysis = analyzeFile(Path.of("/workspace/app.scon"), options)

        assertEquals(emptyList(), analysis.diagnostics)
        assertEquals(SconValue.StringValue("demo"), getPath(analysis.value!!, "name"))

        val limited = analyzeFile(
            Path.of("/workspace/app.scon"),
            options.copy(limits = SconLimits(maxIncludeFiles = 1)),
        )
        assertEquals(SconErrorCode.ResourceLimitExceeded, limited.diagnostics.single().code)
    }
}
