package io.github.realmlabs.scon

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class CoreApiTest {
    @Test
    fun parsesAndResolvesBasicDocument() {
        val value = resolveSource(
            """
            name = "demo"
            enabled = true
            port = 8080
            tags = ["api", "prod"]
            server.host = "127.0.0.1"
            nested {
              answer = 42
            }
            """.trimIndent(),
        )

        assertEquals(
            """{"name":"demo","enabled":true,"port":8080,"tags":["api","prod"],"server":{"host":"127.0.0.1"},"nested":{"answer":42}}""",
            value.toJsonString(),
        )
    }

    @Test
    fun rejectsRootArray() {
        val error = assertFailsWith<SconException> {
            parseSource("[1, 2, 3]")
        }.error

        assertEquals(SconErrorCode.InvalidRootType, error.code)
    }

    @Test
    fun conformanceFixturesMatchKotlinImplementation() {
        val root = conformanceRoot()
        val manifest = Json.parseToJsonElement(Files.readString(root.resolve("manifest.json"))).jsonObject
        for (caseElement in manifest.getValue("cases").jsonArray) {
            val case = caseElement.jsonObject
            val id = case.getValue("id").jsonPrimitive.content
            val description = case.getValue("description").jsonPrimitive.content
            val entry = root.resolve(case.getValue("entry").jsonPrimitive.content)
            val expected = root.resolve(case.getValue("expected").jsonPrimitive.content)
            when (case.getValue("kind").jsonPrimitive.content) {
                "valid" -> {
                    val actual = Json.parseToJsonElement(resolveFile(entry).toJsonString())
                    val expectedJson = Json.parseToJsonElement(Files.readString(expected))
                    assertEquals(expectedJson, actual, "valid conformance case `$id` resolved differently\n$description")
                }
                "invalid" -> {
                    val error = assertFailsWith<SconException>("invalid conformance case `$id` unexpectedly succeeded\n$description") {
                        resolveFile(entry)
                    }.error
                    val expectedCode = Json.parseToJsonElement(Files.readString(expected))
                        .jsonObject
                        .getValue("code")
                        .jsonPrimitive
                        .content
                    assertEquals(expectedCode, error.code.name, "invalid conformance case `$id` produced wrong error\n$description")
                }
                else -> error("unknown conformance case kind for $id")
            }
        }
    }
}

private fun conformanceRoot(): Path =
    Path.of("..", "..", "tests", "conformance")
        .normalize()
        .toAbsolutePath()
