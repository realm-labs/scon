package io.github.realmlabs.scon;

import com.code_intelligence.jazzer.junit.FuzzTest;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;

@EnabledIfSystemProperty(named = "scon.fuzz.enabled", matches = "true")
final class ParserParseFuzzTest {
    @FuzzTest(maxDuration = "30s")
    void parse(byte[] data) {
        String source = new String(data, StandardCharsets.UTF_8);
        try {
            Scon.parseString(source);
        } catch (SconException ignored) {
            // Expected syntax and semantic errors are not fuzz failures.
        }
    }
}
