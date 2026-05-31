package io.github.realmlabs.scon;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

final class SconMapperTest {
    enum Mode { fast, slow }
    record Nested(boolean enabled) {}
    record Config(String name, int port, double ratio, List<String> tags, Nested nested, Mode mode) {}

    @Test
    void readsAndWritesRecord() {
        Config cfg = SconMapper.readValue("""
            name = "demo"
            port = 8080
            ratio = 1.5
            tags = ["a", "b"]
            nested { enabled = true }
            mode = "fast"
            """, Config.class);
        assertEquals(new Config("demo", 8080, 1.5, List.of("a", "b"), new Nested(true), Mode.fast), cfg);
        assertEquals(cfg, SconMapper.readValue(SconMapper.writeValue(cfg), Config.class));
    }

    @Test
    void rejectsInvalidTypedShapes() {
        assertThrows(SconException.class, () -> SconMapper.writeValue(Map.of(1, "bad")));
        assertThrows(SconException.class, () -> SconMapper.readValue("name = 1", Config.class));
    }
}
