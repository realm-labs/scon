package io.github.realmlabs.scon;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import com.code_intelligence.jazzer.junit.FuzzTest;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;

@EnabledIfSystemProperty(named = "scon.fuzz.enabled", matches = "true")
final class ParserFormatSourceFuzzTest {
    @FuzzTest(maxDuration = "30s")
    void formatSource(byte[] data) {
        String source = new String(data, StandardCharsets.UTF_8);
        String formatted;
        try {
            formatted = Scon.formatSource(source);
        } catch (SconException ignored) {
            return;
        }

        assertNotNull(Scon.analyzeSource(formatted).parsed());

        try {
            SconValue original = Scon.parseString(source);
            SconValue roundTrip = Scon.parseString(formatted);
            assertEquals(Scon.formatValue(original), Scon.formatValue(roundTrip));
        } catch (SconException ignored) {
            // If either source fails semantic resolution, formatting parseability
            // remains the invariant under test.
        }
    }
}
