package io.github.realmlabs.scon.jackson;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

final class SconJacksonMapperTest {
    record Config(String name, int port) {}

    @Test
    void readsAndWritesViaJacksonAdapter() {
        var mapper = new SconJacksonMapper();
        var cfg = mapper.readValue("name = \"demo\"\nport = 8080", Config.class);
        assertEquals(new Config("demo", 8080), cfg);
        assertEquals(cfg, mapper.readValue(mapper.writeValue(cfg), Config.class));
    }
}
