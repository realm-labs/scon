package io.github.realmlabs.scon;

final class SourceFormatter {
    private SourceFormatter() {}

    static String formatSource(String source) {
        var document = Parser.parseDocument(source, null);
        var out = new StringBuilder();
        for (var token : Lexer.lex(source)) {
            if (token.kind().equals("comment")) out.append(token.text()).append('\n');
        }
        writeObjectBody(out, document.root(), 0);
        out.append('\n');
        return out.toString();
    }

    private static void writeObjectBody(StringBuilder out, Ast.ObjectNode object, int indent) {
        for (var member : object.members()) {
            out.append(" ".repeat(indent));
            if (member instanceof Ast.Include include) {
                out.append("include ").append(include.path().raw());
            } else if (member instanceof Ast.ObjectSpread spread) {
                out.append("...");
                writeSubstitution(out, spread.sub());
            } else if (member instanceof Ast.Field field) {
                writePath(out, field.path());
                out.append(" = ");
                writeValue(out, field.value(), indent);
            }
            out.append('\n');
        }
    }

    private static void writeValue(StringBuilder out, Ast.ValueNode value, int indent) {
        if (value instanceof Ast.NullNode) {
            out.append("null");
        } else if (value instanceof Ast.BoolNode bool) {
            out.append(bool.value() ? "true" : "false");
        } else if (value instanceof Ast.NumberNode number) {
            out.append(number.raw());
        } else if (value instanceof Ast.StringNode string) {
            out.append(string.raw());
        } else if (value instanceof Ast.SubstitutionNode substitution) {
            writeSubstitution(out, substitution);
        } else if (value instanceof Ast.ArrayNode array) {
            writeArray(out, array, indent);
        } else if (value instanceof Ast.ObjectValueNode object) {
            if (object.object().members().isEmpty()) {
                out.append("{}");
            } else {
                out.append("{\n");
                writeObjectBody(out, object.object(), indent + 2);
                out.append(" ".repeat(indent)).append('}');
            }
        }
    }

    private static void writeArray(StringBuilder out, Ast.ArrayNode array, int indent) {
        if (array.items().isEmpty()) {
            out.append("[]");
            return;
        }
        out.append("[\n");
        for (var item : array.items()) {
            out.append(" ".repeat(indent + 2));
            if (item instanceof Ast.ArrayValue value) {
                writeValue(out, value.value(), indent + 2);
            } else if (item instanceof Ast.ArraySpread spread) {
                out.append("...");
                writeSubstitution(out, spread.sub());
            }
            out.append(",\n");
        }
        out.append(" ".repeat(indent)).append(']');
    }

    private static void writeSubstitution(StringBuilder out, Ast.SubstitutionNode substitution) {
        out.append("${");
        writePath(out, substitution.path());
        out.append('}');
    }

    private static void writePath(StringBuilder out, Ast.PathNode path) {
        for (int i = 0; i < path.segments().size(); i++) {
            if (i > 0) out.append('.');
            var segment = path.segments().get(i);
            if (segment.quoted()) quote(out, segment.value());
            else out.append(segment.value());
        }
    }

    private static void quote(StringBuilder out, String value) {
        out.append('"');
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
                default -> out.append(ch);
            }
        }
        out.append('"');
    }
}
