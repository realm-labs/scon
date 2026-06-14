package io.github.realmlabs.scon

import com.code_intelligence.jazzer.junit.FuzzTest
import org.junit.jupiter.api.condition.EnabledIfSystemProperty
import kotlin.test.assertEquals

@EnabledIfSystemProperty(named = "scon.fuzz.enabled", matches = "true")
@EnabledIfSystemProperty(named = "scon.fuzz.target", matches = "format_source|all")
class ParserFormatSourceFuzzTest {
    @FuzzTest(maxDuration = "30s")
    fun formatSource(data: ByteArray) {
        val source = data.toString(Charsets.UTF_8)
        val formatted = runCatching { formatSource(source) }.getOrNull() ?: return

        parseSource(formatted)

        val original = runCatching { resolveSource(source) }.getOrNull()
        val roundTrip = runCatching { resolveSource(formatted) }.getOrNull()
        if (original != null && roundTrip != null) {
            assertEquals(original, roundTrip)
        }
    }
}
