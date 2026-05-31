import Foundation

struct Token { let kind: String; let text: String; let span: Span }

enum Lexer {
    static func lex(_ source: String) throws -> [Token] {
        let chars = Array(source)
        var tokens: [Token] = []
        var i = 0
        func add(_ kind: String, _ start: Int, _ end: Int) { tokens.append(Token(kind: kind, text: String(chars[start..<end]), span: Span(start: start, end: end))) }
        while i < chars.count {
            let start = i
            let ch = chars[i]
            if ch == " " || ch == "\t" {
                while i < chars.count && (chars[i] == " " || chars[i] == "\t") { i += 1 }
                add("ws", start, i)
            } else if ch == "\n" {
                i += 1; add("newline", start, i)
            } else if String(ch) == "\r\n" {
                i += 1; add("newline", start, i)
            } else if ch == "\r" {
                guard i + 1 < chars.count && chars[i + 1] == "\n" else { throw SconError(.InvalidCharacter, "standalone CR is invalid", span: Span(start: start, end: start + 1)) }
                i += 2; add("newline", start, i)
            } else if ch == "#" || (ch == "/" && i + 1 < chars.count && chars[i + 1] == "/") {
                i += ch == "#" ? 1 : 2
                while i < chars.count && chars[i] != "\n" && chars[i] != "\r" { i += 1 }
                add("comment", start, i)
            } else if ch == "\"" {
                i = try lexString(chars, i); add("string", start, i)
            } else if ch == "$" {
                guard i + 1 < chars.count && chars[i + 1] == "{" else { throw SconError(.InvalidCharacter, "unexpected character '$'", span: Span(start: start, end: start + 1)) }
                i += 2; add("subst", start, i)
            } else if "{}[]=,".contains(ch) {
                i += 1; add(String(ch), start, i)
            } else if ch == "." {
                if i + 2 < chars.count && chars[i + 1] == "." && chars[i + 2] == "." { i += 3; add("...", start, i) } else { i += 1; add(".", start, i) }
            } else if ch == "-" {
                guard i + 1 < chars.count && chars[i + 1].isNumber else { throw SconError(.UnexpectedToken, "expected digit after '-'", span: Span(start: start, end: start + 1)) }
                i = try lexNumber(chars, i); add("number", start, i)
            } else if ch == "?" || ch == ":" {
                throw SconError(.UnexpectedToken, "unexpected character", span: Span(start: start, end: start + 1))
            } else if ch.isNumber {
                i = try lexNumber(chars, i); add("number", start, i)
            } else if isIdentifierStart(ch) {
                while i < chars.count && isIdentifierPart(chars[i]) { i += 1 }
                let text = String(chars[start..<i])
                add(["true", "false", "null", "include"].contains(text) ? text : "identifier", start, i)
            } else if ch.isWhitespace {
                throw SconError(.InvalidWhitespace, "invalid whitespace outside strings", span: Span(start: start, end: start + 1))
            } else {
                throw SconError(.InvalidCharacter, "unexpected character", span: Span(start: start, end: start + 1))
            }
        }
        tokens.append(Token(kind: "eof", text: "", span: Span(start: chars.count, end: chars.count)))
        return tokens
    }
    private static func lexString(_ chars: [Character], _ index: Int) throws -> Int {
        let start = index
        var i = index + 1
        while i < chars.count {
            let ch = chars[i]; i += 1
            if ch == "\"" { return i }
            if ch == "\n" || ch == "\r" { throw SconError(.UnterminatedString, "raw multiline strings are invalid", span: Span(start: i - 1, end: i)) }
            if ch == "\\" {
                guard i < chars.count else { throw SconError(.UnterminatedString, "unterminated string escape", span: Span(start: i, end: i)) }
                let escaped = chars[i]; i += 1
                if "\"\\/bfnrt$".contains(escaped) { continue }
                if escaped == "u" {
                    for _ in 0..<4 {
                        guard i < chars.count && chars[i].isHexDigit else { throw SconError(.InvalidEscape, "invalid unicode escape", span: Span(start: i, end: min(i + 1, chars.count))) }
                        i += 1
                    }
                    continue
                }
                throw SconError(.InvalidEscape, "invalid string escape", span: Span(start: i - 2, end: i - 1))
            }
        }
        throw SconError(.UnterminatedString, "unterminated string", span: Span(start: start, end: chars.count))
    }
    private static func lexNumber(_ chars: [Character], _ index: Int) throws -> Int {
        let start = index
        var i = index
        if chars[i] == "-" { i += 1 }
        if i < chars.count && chars[i] == "0" {
            i += 1
            if i < chars.count && chars[i].isNumber { throw SconError(.InvalidNumber, "leading zeroes are invalid", span: Span(start: start, end: i)) }
        } else {
            guard i < chars.count && "123456789".contains(chars[i]) else { throw SconError(.InvalidNumber, "invalid number", span: Span(start: start, end: i)) }
            while i < chars.count && chars[i].isNumber { i += 1 }
        }
        if i < chars.count && chars[i] == "." {
            i += 1; guard i < chars.count && chars[i].isNumber else { throw SconError(.InvalidNumber, "expected digit after decimal point", span: Span(start: start, end: i)) }
            while i < chars.count && chars[i].isNumber { i += 1 }
        }
        if i < chars.count && (chars[i] == "e" || chars[i] == "E") {
            i += 1; if i < chars.count && (chars[i] == "+" || chars[i] == "-") { i += 1 }
            guard i < chars.count && chars[i].isNumber else { throw SconError(.InvalidNumber, "expected exponent digit", span: Span(start: start, end: i)) }
            while i < chars.count && chars[i].isNumber { i += 1 }
        }
        return i
    }
    private static func isIdentifierStart(_ ch: Character) -> Bool { ch == "_" || ("A"..."Z").contains(ch) || ("a"..."z").contains(ch) }
    private static func isIdentifierPart(_ ch: Character) -> Bool { isIdentifierStart(ch) || ch.isNumber || ch == "-" }
}
