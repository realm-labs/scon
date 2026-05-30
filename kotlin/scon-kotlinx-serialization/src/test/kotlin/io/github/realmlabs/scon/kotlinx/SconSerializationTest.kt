package io.github.realmlabs.scon.kotlinx

import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.MapSerializer
import kotlinx.serialization.builtins.serializer
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class SconSerializationTest {
    @Serializable
    data class Server(
        val host: String,
        val port: Int,
    )

    @Serializable
    data class Metric(
        val value: Double,
    )

    @Serializable
    enum class Mode {
        Dev,
        Prod,
    }

    @Serializable
    data class AppConfig(
        val name: String,
        val enabled: Boolean,
        val retries: UInt,
        val timeout: Double,
        val tags: List<String>,
        val metadata: Map<String, String>,
        val server: Server,
        val mode: Mode = Mode.Dev,
        val optional: String? = null,
    )

    @Test
    fun decodesDataClassesFromScon() {
        val config = Scon.decodeFromString<AppConfig>(
            """
            name = "demo"
            enabled = true
            retries = 3
            timeout = 1.5
            tags = ["api", "prod"]
            metadata = { region = "us" }
            server {
              host = "127.0.0.1"
              port = 8080
            }
            mode = "Prod"
            optional = null
            """.trimIndent(),
        )

        assertEquals("demo", config.name)
        assertEquals(3u, config.retries)
        assertEquals(Server("127.0.0.1", 8080), config.server)
        assertEquals(Mode.Prod, config.mode)
    }

    @Test
    fun encodesDataClassesAsCanonicalSconAndRoundTrips() {
        val config = AppConfig(
            name = "demo",
            enabled = true,
            retries = 3u,
            timeout = 1.5,
            tags = listOf("api", "prod"),
            metadata = linkedMapOf("region" to "us"),
            server = Server("127.0.0.1", 8080),
            mode = Mode.Prod,
        )

        val encoded = Scon.encodeToString(config)

        assertEquals(
            "name = \"demo\"\n" +
                "enabled = true\n" +
                "retries = 3\n" +
                "timeout = 1.5\n" +
                "tags = [\n" +
                "  \"api\",\n" +
                "  \"prod\",\n" +
                "]\n" +
                "metadata = {\n" +
                "  region = \"us\"\n" +
                "}\n" +
                "server = {\n" +
                "  host = \"127.0.0.1\"\n" +
                "  port = 8080\n" +
                "}\n" +
                "mode = \"Prod\"\n" +
                "optional = null\n" +
                "\n",
            encoded,
        )
        assertEquals(config, Scon.decodeFromString<AppConfig>(encoded))
    }

    @Test
    fun rejectsNonObjectRootsWhenEncoding() {
        val error = assertFailsWith<SconSerializationException> {
            Scon.encodeToString(listOf("a", "b"))
        }

        assertEquals(true, error.message?.contains("root must be an object") == true)
    }

    @Test
    fun reportsDecodeTypeMismatch() {
        val error = assertFailsWith<SconSerializationException> {
            Scon.decodeFromString<Server>(
                """
                host = "127.0.0.1"
                port = "wrong"
                """.trimIndent(),
            )
        }

        assertEquals(true, error.message?.contains("port") == true || error.message?.contains("Int") == true)
    }

    @Test
    fun rejectsNonStringMapKeys() {
        val serializer = MapSerializer(Int.serializer(), String.serializer())

        assertFailsWith<SconSerializationException> {
            Scon.encodeToString(serializer, mapOf(1 to "one"))
        }
    }

    @Test
    fun rejectsNonFiniteFloats() {
        assertFailsWith<SconSerializationException> {
            Scon.encodeToString(Metric(Double.NaN))
        }
    }

    @Test
    fun reportsNumericOverflow() {
        val error = assertFailsWith<SconSerializationException> {
            Scon.decodeFromString<Server>(
                """
                host = "127.0.0.1"
                port = 9223372036854775807
                """.trimIndent(),
            )
        }

        assertEquals(true, error.message?.contains("port") == true || error.message?.contains("Int") == true)
    }
}
