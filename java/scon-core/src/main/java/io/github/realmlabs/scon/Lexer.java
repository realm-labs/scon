package io.github.realmlabs.scon;

import java.util.ArrayList;
import java.util.List;

final class Lexer {
    record Token(String kind, String text, Span span) {}

    static List<Token> lex(String source) {
        var tokens = new ArrayList<Token>();
        int index = 0;
        while (index < source.length()) {
            int start = index;
            char ch = source.charAt(index);
            if (ch == ' ' || ch == '\t') {
                while (index < source.length() && (source.charAt(index) == ' ' || source.charAt(index) == '\t')) index++;
                add(tokens, source, "ws", start, index);
            } else if (ch == '\n') {
                index++;
                add(tokens, source, "newline", start, index);
            } else if (ch == '\r') {
                if (index + 1 >= source.length() || source.charAt(index + 1) != '\n') {
                    fail(ErrorCode.InvalidCharacter, "standalone CR is invalid", start, start + 1);
                }
                index += 2;
                add(tokens, source, "newline", start, index);
            } else if (ch == '#' || (ch == '/' && index + 1 < source.length() && source.charAt(index + 1) == '/')) {
                index += ch == '#' ? 1 : 2;
                while (index < source.length() && source.charAt(index) != '\n' && source.charAt(index) != '\r') index++;
                add(tokens, source, "comment", start, index);
            } else if (ch == '"') {
                index = lexString(source, index);
                add(tokens, source, "string", start, index);
            } else if (ch == '$') {
                if (index + 1 >= source.length() || source.charAt(index + 1) != '{') {
                    fail(ErrorCode.InvalidCharacter, "unexpected character '$'", start, start + 1);
                }
                index += 2;
                add(tokens, source, "subst", start, index);
            } else if ("{}[]=,".indexOf(ch) >= 0) {
                index++;
                add(tokens, source, Character.toString(ch), start, index);
            } else if (ch == '.') {
                if (source.startsWith("...", index)) {
                    index += 3;
                    add(tokens, source, "...", start, index);
                } else {
                    index++;
                    add(tokens, source, ".", start, index);
                }
            } else if (ch == '-') {
                if (index + 1 >= source.length() || !isDigit(source.charAt(index + 1))) {
                    fail(ErrorCode.UnexpectedToken, "expected digit after '-'", start, start + 1);
                }
                index = lexNumber(source, index);
                add(tokens, source, "number", start, index);
            } else if (ch == '?' || ch == ':') {
                fail(ErrorCode.UnexpectedToken, "unexpected character", start, start + 1);
            } else if (isDigit(ch)) {
                index = lexNumber(source, index);
                add(tokens, source, "number", start, index);
            } else if (isIdentifierStart(ch)) {
                while (index < source.length() && isIdentifierPart(source.charAt(index))) index++;
                String text = source.substring(start, index);
                add(tokens, source, switch (text) {
                    case "true", "false", "null", "include" -> text;
                    default -> "identifier";
                }, start, index);
            } else if (Character.isWhitespace(ch) || Character.isSpaceChar(ch)) {
                fail(ErrorCode.InvalidWhitespace, "invalid whitespace outside strings", start, start + 1);
            } else {
                fail(ErrorCode.InvalidCharacter, "unexpected character", start, start + 1);
            }
        }
        tokens.add(new Token("eof", "", new Span(source.length(), source.length())));
        return tokens;
    }

    private static int lexString(String source, int index) {
        int start = index++;
        while (index < source.length()) {
            char ch = source.charAt(index++);
            if (ch == '"') return index;
            if (ch == '\n' || ch == '\r') {
                fail(ErrorCode.UnterminatedString, "raw multiline strings are invalid", index - 1, index);
            }
            if (ch == '\\') {
                if (index >= source.length()) {
                    fail(ErrorCode.UnterminatedString, "unterminated string escape", index, index);
                }
                char escaped = source.charAt(index++);
                if ("\"\\/bfnrt$".indexOf(escaped) >= 0) continue;
                if (escaped == 'u') {
                    for (int i = 0; i < 4; i++, index++) {
                        if (index >= source.length() || !isHex(source.charAt(index))) {
                            fail(ErrorCode.InvalidEscape, "invalid unicode escape", index, Math.min(index + 1, source.length()));
                        }
                    }
                    continue;
                }
                fail(ErrorCode.InvalidEscape, "invalid string escape", index - 2, index - 1);
            }
        }
        fail(ErrorCode.UnterminatedString, "unterminated string", start, source.length());
        throw new AssertionError();
    }

    private static int lexNumber(String source, int index) {
        int start = index;
        if (source.charAt(index) == '-') index++;
        if (index < source.length() && source.charAt(index) == '0') {
            index++;
            if (index < source.length() && isDigit(source.charAt(index))) {
                fail(ErrorCode.InvalidNumber, "leading zeroes are invalid", start, index);
            }
        } else {
            if (index >= source.length() || source.charAt(index) < '1' || source.charAt(index) > '9') {
                fail(ErrorCode.InvalidNumber, "invalid number", start, index);
            }
            while (index < source.length() && isDigit(source.charAt(index))) index++;
        }
        if (index < source.length() && source.charAt(index) == '.') {
            index++;
            if (index >= source.length() || !isDigit(source.charAt(index))) {
                fail(ErrorCode.InvalidNumber, "expected digit after decimal point", start, index);
            }
            while (index < source.length() && isDigit(source.charAt(index))) index++;
        }
        if (index < source.length() && (source.charAt(index) == 'e' || source.charAt(index) == 'E')) {
            index++;
            if (index < source.length() && (source.charAt(index) == '+' || source.charAt(index) == '-')) index++;
            if (index >= source.length() || !isDigit(source.charAt(index))) {
                fail(ErrorCode.InvalidNumber, "expected exponent digit", start, index);
            }
            while (index < source.length() && isDigit(source.charAt(index))) index++;
        }
        return index;
    }

    private static void add(List<Token> tokens, String source, String kind, int start, int end) {
        tokens.add(new Token(kind, source.substring(start, end), new Span(start, end)));
    }

    private static boolean isDigit(char ch) {
        return ch >= '0' && ch <= '9';
    }

    private static boolean isHex(char ch) {
        return isDigit(ch) || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F');
    }

    private static boolean isIdentifierStart(char ch) {
        return ch == '_' || (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z');
    }

    private static boolean isIdentifierPart(char ch) {
        return isIdentifierStart(ch) || isDigit(ch) || ch == '-';
    }

    private static void fail(ErrorCode code, String message, int start, int end) {
        throw new SconException(code, message, new Span(start, end));
    }
}
