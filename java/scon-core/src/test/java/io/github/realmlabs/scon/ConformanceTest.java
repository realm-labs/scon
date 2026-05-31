package io.github.realmlabs.scon;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

final class ConformanceTest {
    private static final Path ROOT = Path.of("../..").toAbsolutePath().normalize();
    private static final Path CONFORMANCE = ROOT.resolve("tests/conformance");
    private static final ObjectMapper JSON = new ObjectMapper();

    @TestFactory
    Iterable<DynamicTest> cases() throws Exception {
        JsonNode manifest = JSON.readTree(CONFORMANCE.resolve("manifest.json").toFile());
        List<DynamicTest> tests = new ArrayList<>();
        for (JsonNode node : manifest.get("cases")) {
            tests.add(DynamicTest.dynamicTest(node.get("id").asText(), () -> runCase(node)));
        }
        return tests;
    }

    private static void runCase(JsonNode node) throws Exception {
        Path entry = CONFORMANCE.resolve(node.get("entry").asText());
        JsonNode expected = JSON.readTree(CONFORMANCE.resolve(node.get("expected").asText()).toFile());
        if (node.get("kind").asText().equals("valid")) {
            SconValue value = Scon.parseFile(entry);
            assertEquals(expected, toJsonTree(SconMapper.decode(value, Object.class)));
            assertEquals(expected, toJsonTree(SconMapper.decode(Scon.parseString(Scon.formatValue(value)), Object.class)));
        } else {
            SconException ex = assertThrows(SconException.class, () -> Scon.parseFile(entry));
            assertEquals(ErrorCode.valueOf(expected.get("code").asText()), ex.code());
        }
    }

    private static JsonNode toJsonTree(Object value) throws Exception {
        return JSON.readTree(JSON.writeValueAsString(value));
    }
}
