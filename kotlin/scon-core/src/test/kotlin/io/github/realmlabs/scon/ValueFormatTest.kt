package io.github.realmlabs.scon

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class ValueFormatTest {
    @Test
    fun formatsResolvedValuesAsCanonicalScon() {
        val value = SconValue.ObjectValue(
            linkedMapOf(
                "name" to SconValue.StringValue("demo"),
                "needs quote" to SconValue.StringValue("literal \${path}"),
                "items" to SconValue.ArrayValue(
                    listOf(
                        SconValue.Number(SconNumber.I64(-1)),
                        SconValue.Bool(true),
                        SconValue.ObjectValue(linkedMapOf("nested" to SconValue.Null)),
                    ),
                ),
            ),
        )

        assertEquals(
            "name = \"demo\"\n" +
                "\"needs quote\" = \"literal \\${'$'}{path}\"\n" +
                "items = [\n" +
                "  -1,\n" +
                "  true,\n" +
                "  {\n" +
                "    nested = null\n" +
                "  },\n" +
                "]\n" +
                "\n",
            value.toSconString(),
        )
    }

    @Test
    fun formattedResolvedValuesRoundTrip() {
        val source = """
            name = "demo"
            nested {
              value = 42
            }
        """.trimIndent()
        val value = parseValue(source)

        assertEquals(value, parseValue(value.toSconString()))
    }

    @Test
    fun rejectsNonObjectRootForCanonicalSconDocuments() {
        val error = assertFailsWith<SconException> {
            SconValue.ArrayValue(emptyList()).toSconString()
        }.error

        assertEquals(SconErrorCode.InvalidRootType, error.code)
    }
}
