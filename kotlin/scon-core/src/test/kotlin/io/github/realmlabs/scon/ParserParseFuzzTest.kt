package io.github.realmlabs.scon

import com.code_intelligence.jazzer.junit.FuzzTest
import org.junit.jupiter.api.condition.EnabledIfSystemProperty

@EnabledIfSystemProperty(named = "scon.fuzz.enabled", matches = "true")
@EnabledIfSystemProperty(named = "scon.fuzz.target", matches = "parse|all")
class ParserParseFuzzTest {
    @FuzzTest(maxDuration = "30s")
    fun parse(data: ByteArray) {
        val source = data.toString(Charsets.UTF_8)
        runCatching { resolveSource(source) }
    }
}
