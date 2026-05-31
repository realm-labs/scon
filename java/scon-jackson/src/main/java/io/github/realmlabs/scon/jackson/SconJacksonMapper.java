package io.github.realmlabs.scon.jackson;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.github.realmlabs.scon.SconMapper;
import io.github.realmlabs.scon.SconValue;

public final class SconJacksonMapper {
    private final ObjectMapper mapper;

    public SconJacksonMapper() {
        this(new ObjectMapper());
    }

    public SconJacksonMapper(ObjectMapper mapper) {
        this.mapper = mapper;
    }

    public <T> T readValue(String source, Class<T> type) {
        return SconMapper.readValue(source, type);
    }

    public String writeValue(Object value) {
        Object plain = mapper.convertValue(value, Object.class);
        return SconMapper.writeValue(plain);
    }

    public <T> T convertValue(SconValue value, Class<T> type) {
        return mapper.convertValue(SconMapper.decode(value, Object.class), type);
    }
}
