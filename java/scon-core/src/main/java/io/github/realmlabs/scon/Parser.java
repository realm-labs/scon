package io.github.realmlabs.scon;

import java.util.ArrayList;
import java.util.List;

final class Parser {
    private final List<Lexer.Token> tokens;
    private int index;

    private Parser(List<Lexer.Token> tokens) {
        this.tokens = tokens;
    }

    static Ast.Document parseDocument(String source, String file) {
        return new Ast.Document(new Parser(Lexer.lex(source)).parse(), file);
    }

    private Ast.ObjectNode parse() {
        skipTrivia();
        Ast.ObjectNode root;
        if (match("{")) {
            root = parseObject(previous());
        } else if (check("[")) {
            throw new SconException(ErrorCode.InvalidRootType, "SCON document root must be an object", peek().span());
        } else {
            root = parseObjectBody(peek().span().start());
        }
        skipTrivia();
        expect("eof", "expected end of file");
        return root;
    }

    private Ast.ObjectNode parseObject(Lexer.Token opening) {
        var members = parseMembers("}");
        var closing = expect("}", "expected '}'");
        return new Ast.ObjectNode(members, new Span(opening.span().start(), closing.span().end()));
    }

    private Ast.ObjectNode parseObjectBody(int start) {
        var members = parseMembers("eof");
        int end = members.isEmpty() ? start : members.get(members.size() - 1).span().end();
        return new Ast.ObjectNode(members, new Span(start, end));
    }

    private List<Ast.Member> parseMembers(String end) {
        var members = new ArrayList<Ast.Member>();
        skipTrivia();
        while (!check(end) && !check("eof")) {
            members.add(parseMember());
            skipTrivia();
            if (match(",")) {
                skipTrivia();
                if (check(",")) {
                    throw new SconException(ErrorCode.UnexpectedToken, "consecutive commas are invalid", peek().span());
                }
            }
        }
        return members;
    }

    private Ast.Member parseMember() {
        skipTrivia();
        if (match("include")) {
            var include = previous();
            skipInlineTrivia();
            var path = parseString();
            if (path.parts().stream().anyMatch(Ast.StringInterpolation.class::isInstance)) {
                throw new SconException(ErrorCode.UnexpectedToken, "include path must be a literal string", path.span());
            }
            return new Ast.Include(path, new Span(include.span().start(), path.span().end()));
        }
        if (match("...")) {
            var spread = previous();
            var sub = parseSubstitution();
            return new Ast.ObjectSpread(sub, new Span(spread.span().start(), sub.span().end()));
        }
        var path = parsePath();
        skipInlineTrivia();
        Ast.ValueNode value;
        if (match("=")) {
            skipInlineTrivia();
            if (check("newline")) {
                throw new SconException(ErrorCode.UnexpectedToken, "field value cannot start on the next line", peek().span());
            }
            value = parseValue();
        } else if (match("{")) {
            var object = parseObject(previous());
            value = new Ast.ObjectValueNode(object, object.span());
        } else {
            throw new SconException(ErrorCode.UnexpectedToken, "expected '=' or object shorthand", peek().span());
        }
        return new Ast.Field(path, value, new Span(path.span().start(), value.span().end()));
    }

    private Ast.ValueNode parseValue() {
        skipTrivia();
        if (match("null")) return new Ast.NullNode(previous().span());
        if (match("true")) return new Ast.BoolNode(true, previous().span());
        if (match("false")) return new Ast.BoolNode(false, previous().span());
        if (match("number")) {
            var token = previous();
            return new Ast.NumberNode(token.text(), token.span());
        }
        if (check("string")) return parseString();
        if (match("{")) {
            var object = parseObject(previous());
            return new Ast.ObjectValueNode(object, object.span());
        }
        if (match("[")) return parseArray(previous());
        if (check("subst")) return parseSubstitution();
        throw new SconException(ErrorCode.UnexpectedToken, "expected value", peek().span());
    }

    private Ast.ArrayNode parseArray(Lexer.Token opening) {
        var items = new ArrayList<Ast.ArrayItem>();
        skipTrivia();
        while (!check("]") && !check("eof")) {
            int start = peek().span().start();
            if (match("...")) {
                var sub = parseSubstitution();
                items.add(new Ast.ArraySpread(sub, new Span(start, sub.span().end())));
            } else {
                var value = parseValue();
                items.add(new Ast.ArrayValue(value, value.span()));
            }
            skipTrivia();
            if (!match(",")) break;
            skipTrivia();
            if (check(",")) {
                throw new SconException(ErrorCode.UnexpectedToken, "consecutive commas are invalid", peek().span());
            }
        }
        var closing = expect("]", "expected ']'");
        return new Ast.ArrayNode(items, new Span(opening.span().start(), closing.span().end()));
    }

    private Ast.SubstitutionNode parseSubstitution() {
        var start = expect("subst", "expected '${'");
        var path = parsePath();
        var end = expect("}", "expected '}'");
        return new Ast.SubstitutionNode(path, new Span(start.span().start(), end.span().end()));
    }

    private Ast.PathNode parsePath() {
        var first = parsePathSegment();
        var segments = new ArrayList<Ast.PathSegment>();
        segments.add(first);
        while (match(".")) segments.add(parsePathSegment());
        return new Ast.PathNode(segments, new Span(first.span().start(), segments.get(segments.size() - 1).span().end()));
    }

    private Ast.PathSegment parsePathSegment() {
        if (match("identifier")) {
            var token = previous();
            return new Ast.PathSegment(token.text(), false, token.span());
        }
        if (check("string")) {
            var string = parseString();
            return new Ast.PathSegment(string.value(), true, string.span());
        }
        throw new SconException(ErrorCode.UnexpectedToken, "expected path segment", peek().span());
    }

    private Ast.StringNode parseString() {
        var token = expect("string", "expected string");
        return parseStringParts(token);
    }

    private Ast.StringNode parseStringParts(Lexer.Token token) {
        String raw = token.text();
        var parts = new ArrayList<Ast.StringPart>();
        var out = new StringBuilder();
        var value = new StringBuilder();
        int i = 1;
        while (i < raw.length() - 1) {
            char ch = raw.charAt(i++);
            if (ch == '$' && i < raw.length() && raw.charAt(i) == '{') {
                if (!out.isEmpty()) {
                    parts.add(new Ast.StringLiteral(out.toString()));
                    value.append(out);
                    out.setLength(0);
                }
                int pathStart = i + 1;
                int close = raw.indexOf('}', pathStart);
                if (close < 0) {
                    throw new SconException(ErrorCode.UnterminatedString, "unterminated interpolation", token.span());
                }
                parts.add(new Ast.StringInterpolation(
                    parseInterpolationPath(raw.substring(pathStart, close), token.span().start() + pathStart),
                    new Span(token.span().start() + i - 1, token.span().start() + close + 1)
                ));
                i = close + 1;
                continue;
            }
            if (ch != '\\') {
                out.append(ch);
                continue;
            }
            char escaped = raw.charAt(i++);
            switch (escaped) {
                case '"' -> out.append('"');
                case '\\' -> out.append('\\');
                case '/' -> out.append('/');
                case 'b' -> out.append('\b');
                case 'f' -> out.append('\f');
                case 'n' -> out.append('\n');
                case 'r' -> out.append('\r');
                case 't' -> out.append('\t');
                case '$' -> out.append('$');
                case 'u' -> {
                    out.append((char) Integer.parseInt(raw.substring(i, i + 4), 16));
                    i += 4;
                }
                default -> throw new SconException(ErrorCode.InvalidEscape, "invalid string escape", token.span());
            }
        }
        if (!out.isEmpty() || parts.isEmpty()) {
            parts.add(new Ast.StringLiteral(out.toString()));
            value.append(out);
        }
        return new Ast.StringNode(value.toString(), raw, parts, token.span());
    }

    private Ast.PathNode parseInterpolationPath(String text, int base) {
        var adjusted = new ArrayList<Lexer.Token>();
        for (var token : Lexer.lex(text)) {
            adjusted.add(new Lexer.Token(token.kind(), token.text(), new Span(token.span().start() + base, token.span().end() + base)));
        }
        var parser = new Parser(adjusted);
        var path = parser.parsePath();
        parser.expect("eof", "expected end of interpolation");
        return path;
    }

    private void skipTrivia() {
        while (match("ws") || match("newline") || match("comment")) {}
    }

    private void skipInlineTrivia() {
        while (match("ws") || match("comment")) {}
    }

    private boolean match(String kind) {
        if (!check(kind)) return false;
        index++;
        return true;
    }

    private boolean check(String kind) {
        return peek().kind().equals(kind);
    }

    private Lexer.Token expect(String kind, String message) {
        if (check(kind)) {
            index++;
            return previous();
        }
        throw new SconException(ErrorCode.UnexpectedToken, message, peek().span());
    }

    private Lexer.Token peek() {
        return tokens.get(Math.min(index, tokens.size() - 1));
    }

    private Lexer.Token previous() {
        return tokens.get(index - 1);
    }
}
