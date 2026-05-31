package io.github.realmlabs.scon;

import java.util.Map;

final class Formatter {
    private Formatter() {}

    static String formatValue(SconValue value) {
        if (!(value instanceof SconObject object)) {
            throw new SconException(ErrorCode.InvalidRootType, "SCON document root must be an object");
        }
        return formatObjectBody(object, 0) + "\n";
    }

    private static String formatObjectBody(SconObject object, int indent) {
        var out = new StringBuilder();
        for (Map.Entry<String, SconValue> entry : object.entrySet()) {
            out.append(" ".repeat(indent))
                .append(formatKey(entry.getKey()))
                .append(" = ")
                .append(formatScon(entry.getValue(), indent))
                .append('\n');
        }
        return out.toString();
    }

    private static String formatScon(SconValue value, int indent) {
        if (value == SconNull.INSTANCE) return "null";
        if (value instanceof SconBool bool) return bool.value() ? "true" : "false";
        if (value instanceof SconString string) return quote(string.value(), true);
        if (value instanceof SconNumber number) return number.toSconString();
        if (value instanceof SconArray array) {
            if (array.isEmpty()) return "[]";
            var out = new StringBuilder("[\n");
            for (SconValue item : array) {
                out.append(" ".repeat(indent + 2)).append(formatScon(item, indent + 2)).append(",\n");
            }
            return out.append(" ".repeat(indent)).append(']').toString();
        }
        if (value instanceof SconObject object) {
            if (object.isEmpty()) return "{}";
            return "{\n" + formatObjectBody(object, indent + 2) + " ".repeat(indent) + "}";
        }
        throw new SconException(ErrorCode.Serde, "unsupported SCON value");
    }

    private static String formatKey(String key) {
        return key.matches("[A-Za-z_][A-Za-z0-9_-]*") ? key : quote(key, false);
    }

    private static String quote(String value, boolean escapeInterpolation) {
        var out = new StringBuilder("\"");
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            switch (ch) {
                case '"' -> out.append("\\\"");
                case '\\' -> out.append("\\\\");
                case '\n' -> out.append("\\n");
                case '\r' -> out.append("\\r");
                case '\t' -> out.append("\\t");
                case '\b' -> out.append("\\b");
                case '\f' -> out.append("\\f");
                case '$' -> out.append(escapeInterpolation ? "\\$" : "$");
                default -> {
                    if (Character.isISOControl(ch)) out.append(String.format("\\u%04X", (int) ch));
                    else out.append(ch);
                }
            }
        }
        return out.append('"').toString();
    }
}
