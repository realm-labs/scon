package io.github.realmlabs.scon;

import java.lang.reflect.*;
import java.nio.file.Path;
import java.util.*;

public final class SconMapper {
    private SconMapper() {}

    public static <T> T readValue(String source, Class<T> type) {
        return decode(Scon.parseString(source), type);
    }

    public static <T> T readFile(Path path, Class<T> type) {
        return decode(Scon.parseFile(path), type);
    }

    public static String writeValue(Object value) {
        return Scon.formatValue(encode(value));
    }

    @SuppressWarnings("unchecked")
    public static <T> T decode(SconValue value, Type type) {
        if (type == Object.class) return (T) plain(value);
        if (type == String.class) {
            if (value instanceof SconString s) return (T) s.value();
            throw serde("expected string");
        }
        if (type == boolean.class || type == Boolean.class) {
            if (value instanceof SconBool b) return (T) Boolean.valueOf(b.value());
            throw serde("expected bool");
        }
        if (type instanceof Class<?> cls && Number.class.isAssignableFrom(box(cls)) || isPrimitiveNumber(type)) {
            if (!(value instanceof SconNumber number)) throw serde("expected number");
            return (T) decodeNumber(number, (Class<?>) type);
        }
        if (type instanceof Class<?> cls && cls.isEnum()) {
            if (!(value instanceof SconString s)) throw serde("expected enum string");
            return (T) Enum.valueOf((Class<? extends Enum>) cls.asSubclass(Enum.class), s.value());
        }
        if (type instanceof Class<?> cls && cls.isRecord()) return decodeRecord(value, cls);
        if (type instanceof ParameterizedType pt && pt.getRawType() instanceof Class<?> raw) {
            if (List.class.isAssignableFrom(raw)) {
                if (!(value instanceof SconArray array)) throw serde("expected array");
                var out = new ArrayList<>();
                Type itemType = pt.getActualTypeArguments()[0];
                for (SconValue item : array) out.add(decode(item, itemType));
                return (T) out;
            }
            if (Map.class.isAssignableFrom(raw)) {
                if (!(value instanceof SconObject object)) throw serde("expected object");
                if (pt.getActualTypeArguments()[0] != String.class) throw serde("SCON map keys must be strings");
                var out = new LinkedHashMap<String, Object>();
                Type itemType = pt.getActualTypeArguments()[1];
                for (var entry : object.entrySet()) out.put(entry.getKey(), decode(entry.getValue(), itemType));
                return (T) out;
            }
        }
        if (type instanceof Class<?> cls && cls.isArray()) {
            if (!(value instanceof SconArray array)) throw serde("expected array");
            Class<?> itemType = cls.getComponentType();
            Object out = Array.newInstance(itemType, array.size());
            for (int i = 0; i < array.size(); i++) Array.set(out, i, decode(array.get(i), itemType));
            return (T) out;
        }
        if (type instanceof Class<?> cls) return decodeBean(value, cls);
        throw serde("unsupported target type: " + type);
    }

    public static SconValue encode(Object value) {
        if (value == null) return SconNull.INSTANCE;
        if (value instanceof SconValue sconValue) return sconValue;
        if (value instanceof Boolean bool) return new SconBool(bool);
        if (value instanceof String string) return new SconString(string);
        if (value instanceof Byte || value instanceof Short || value instanceof Integer || value instanceof Long) {
            long n = ((Number) value).longValue();
            return n < 0 ? SconNumber.ofI64(n) : SconNumber.ofU64(n);
        }
        if (value instanceof Float || value instanceof Double) {
            double n = ((Number) value).doubleValue();
            if (!Double.isFinite(n)) throw serde("non-finite floats cannot be serialized");
            return SconNumber.ofF64(n);
        }
        if (value instanceof Enum<?> e) return new SconString(e.name());
        if (value instanceof Iterable<?> iterable) {
            var out = new SconArray();
            for (Object item : iterable) out.add(encode(item));
            return out;
        }
        if (value.getClass().isArray()) {
            var out = new SconArray();
            int len = Array.getLength(value);
            for (int i = 0; i < len; i++) out.add(encode(Array.get(value, i)));
            return out;
        }
        if (value instanceof Map<?, ?> map) {
            var out = new SconObject();
            for (var entry : map.entrySet()) {
                if (!(entry.getKey() instanceof String key)) throw serde("SCON map keys must be strings");
                out.put(key, encode(entry.getValue()));
            }
            return out;
        }
        if (value.getClass().isRecord()) return encodeRecord(value);
        return encodeBean(value);
    }

    private static Object decodeNumber(SconNumber number, Class<?> type) {
        if (type == byte.class || type == Byte.class) {
            long n = number.asI64();
            if (n < Byte.MIN_VALUE || n > Byte.MAX_VALUE) throw serde("integer overflow");
            return (byte) n;
        }
        if (type == short.class || type == Short.class) {
            long n = number.asI64();
            if (n < Short.MIN_VALUE || n > Short.MAX_VALUE) throw serde("integer overflow");
            return (short) n;
        }
        if (type == int.class || type == Integer.class) {
            long n = number.asI64();
            if (n < Integer.MIN_VALUE || n > Integer.MAX_VALUE) throw serde("integer overflow");
            return (int) n;
        }
        if (type == long.class || type == Long.class) return number.asI64();
        if (type == float.class || type == Float.class) return (float) number.asF64();
        if (type == double.class || type == Double.class) return number.asF64();
        throw serde("unsupported numeric type");
    }

    private static <T> T decodeRecord(SconValue value, Class<?> cls) {
        if (!(value instanceof SconObject object)) throw serde("expected object");
        try {
            var components = cls.getRecordComponents();
            var args = new Object[components.length];
            var types = new Class<?>[components.length];
            for (int i = 0; i < components.length; i++) {
                var component = components[i];
                SconValue item = object.get(component.getName());
                if (item == null) throw serde("missing field " + component.getName());
                args[i] = decode(item, component.getGenericType());
                types[i] = component.getType();
            }
            var constructor = cls.getDeclaredConstructor(types);
            constructor.setAccessible(true);
            @SuppressWarnings("unchecked")
            T out = (T) constructor.newInstance(args);
            return out;
        } catch (ReflectiveOperationException ex) {
            throw serde("record decode failed: " + ex.getMessage());
        }
    }

    private static <T> T decodeBean(SconValue value, Class<?> cls) {
        if (!(value instanceof SconObject object)) throw serde("expected object");
        try {
            Object out = cls.getDeclaredConstructor().newInstance();
            for (Field field : cls.getDeclaredFields()) {
                if (Modifier.isStatic(field.getModifiers())) continue;
                SconValue item = object.get(field.getName());
                if (item == null) continue;
                field.setAccessible(true);
                field.set(out, decode(item, field.getGenericType()));
            }
            @SuppressWarnings("unchecked")
            T typed = (T) out;
            return typed;
        } catch (ReflectiveOperationException ex) {
            throw serde("bean decode failed: " + ex.getMessage());
        }
    }

    private static SconValue encodeRecord(Object value) {
        var out = new SconObject();
        try {
            for (RecordComponent component : value.getClass().getRecordComponents()) {
                var accessor = component.getAccessor();
                accessor.setAccessible(true);
                out.put(component.getName(), encode(accessor.invoke(value)));
            }
            return out;
        } catch (ReflectiveOperationException ex) {
            throw serde("record encode failed: " + ex.getMessage());
        }
    }

    private static SconValue encodeBean(Object value) {
        var out = new SconObject();
        try {
            for (Field field : value.getClass().getDeclaredFields()) {
                if (Modifier.isStatic(field.getModifiers())) continue;
                field.setAccessible(true);
                out.put(field.getName(), encode(field.get(value)));
            }
            return out;
        } catch (ReflectiveOperationException ex) {
            throw serde("bean encode failed: " + ex.getMessage());
        }
    }

    private static Object plain(SconValue value) {
        if (value == SconNull.INSTANCE) return null;
        if (value instanceof SconBool bool) return bool.value();
        if (value instanceof SconString string) return string.value();
        if (value instanceof SconNumber number) return number.plainValue();
        if (value instanceof SconArray array) {
            var out = new ArrayList<>();
            for (SconValue item : array) out.add(plain(item));
            return out;
        }
        if (value instanceof SconObject object) {
            var out = new LinkedHashMap<String, Object>();
            for (var entry : object.entrySet()) out.put(entry.getKey(), plain(entry.getValue()));
            return out;
        }
        throw serde("unsupported SCON value");
    }

    private static boolean isPrimitiveNumber(Type type) {
        return type == byte.class || type == short.class || type == int.class || type == long.class || type == float.class || type == double.class;
    }

    private static Class<?> box(Class<?> cls) {
        if (!cls.isPrimitive()) return cls;
        if (cls == int.class) return Integer.class;
        if (cls == long.class) return Long.class;
        if (cls == short.class) return Short.class;
        if (cls == byte.class) return Byte.class;
        if (cls == float.class) return Float.class;
        if (cls == double.class) return Double.class;
        if (cls == boolean.class) return Boolean.class;
        return cls;
    }

    private static SconException serde(String message) {
        return new SconException(ErrorCode.Serde, message);
    }
}
