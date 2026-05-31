import Foundation

final class Parser {
    private let tokens: [Token]
    private var index = 0
    private init(_ tokens: [Token]) { self.tokens = tokens }
    static func parseDocument(_ source: String, file: String? = nil) throws -> Document { Document(root: try Parser(try Lexer.lex(source)).parse(), file: file) }
    private func parse() throws -> AstObject {
        skipTrivia()
        let root = try match("{") ? parseObject(previous()) : (check("[") ? invalidRoot() : parseObjectBody(peek().span.start))
        skipTrivia(); _ = try expect("eof", "expected end of file")
        return root
    }
    private func parseObject(_ opening: Token) throws -> AstObject { let members = try parseMembers("}"); let closing = try expect("}", "expected '}'"); return AstObject(members: members, span: Span(start: opening.span.start, end: closing.span.end)) }
    private func parseObjectBody(_ start: Int) throws -> AstObject { let members = try parseMembers("eof"); return AstObject(members: members, span: Span(start: start, end: members.last?.span.end ?? start)) }
    private func parseMembers(_ end: String) throws -> [AstMember] {
        var members: [AstMember] = []; skipTrivia()
        while !check(end) && !check("eof") {
            members.append(try parseMember()); skipTrivia()
            if match(",") { skipTrivia(); if check(",") { throw SconError(.UnexpectedToken, "consecutive commas are invalid", span: peek().span) } }
        }
        return members
    }
    private func parseMember() throws -> AstMember {
        skipTrivia()
        if match("include") { let inc = previous(); skipInlineTrivia(); let path = try parseString(); if path.parts.contains(where: { if case .interpolation = $0 { true } else { false } }) { throw SconError(.UnexpectedToken, "include path must be a literal string", span: path.span) }; return .include(AstInclude(path: path, span: Span(start: inc.span.start, end: path.span.end))) }
        if match("...") { let spread = previous(); let sub = try parseSubstitution(); return .objectSpread(AstObjectSpread(sub: sub, span: Span(start: spread.span.start, end: sub.span.end))) }
        let path = try parsePath(); skipInlineTrivia()
        let value: AstValue
        if match("=") { skipInlineTrivia(); if check("newline") { throw SconError(.UnexpectedToken, "field value cannot start on the next line", span: peek().span) }; value = try parseValue() }
        else if match("{") { let obj = try parseObject(previous()); value = .object(obj, obj.span) }
        else { throw SconError(.UnexpectedToken, "expected '=' or object shorthand", span: peek().span) }
        return .field(AstField(path: path, value: value, span: Span(start: path.span.start, end: value.span.end)))
    }
    private func parseValue() throws -> AstValue {
        skipTrivia()
        if match("null") { return .null(previous().span) }
        if match("true") { return .bool(true, previous().span) }
        if match("false") { return .bool(false, previous().span) }
        if match("number") { let t = previous(); return .number(t.text, t.span) }
        if check("string") { return .string(try parseString()) }
        if match("{") { let obj = try parseObject(previous()); return .object(obj, obj.span) }
        if match("[") { return .array(try parseArray(previous())) }
        if check("subst") { return .substitution(try parseSubstitution()) }
        throw SconError(.UnexpectedToken, "expected value", span: peek().span)
    }
    private func parseArray(_ opening: Token) throws -> AstArray {
        var items: [AstArrayItem] = []; skipTrivia()
        while !check("]") && !check("eof") {
            let start = peek().span.start
            if match("...") { let sub = try parseSubstitution(); items.append(.spread(sub, Span(start: start, end: sub.span.end))) }
            else { let value = try parseValue(); items.append(.value(value, value.span)) }
            skipTrivia(); if !match(",") { break }; skipTrivia(); if check(",") { throw SconError(.UnexpectedToken, "consecutive commas are invalid", span: peek().span) }
        }
        let closing = try expect("]", "expected ']'")
        return AstArray(items: items, span: Span(start: opening.span.start, end: closing.span.end))
    }
    private func parseSubstitution() throws -> AstSubstitution { let start = try expect("subst", "expected '${'"); let path = try parsePath(); let end = try expect("}", "expected '}'"); return AstSubstitution(path: path, span: Span(start: start.span.start, end: end.span.end)) }
    private func parsePath() throws -> AstPath { let first = try parsePathSegment(); var segments = [first]; while match(".") { segments.append(try parsePathSegment()) }; return AstPath(segments: segments, span: Span(start: first.span.start, end: segments.last!.span.end)) }
    private func parsePathSegment() throws -> AstPathSegment {
        if match("identifier") { let t = previous(); return AstPathSegment(value: t.text, quoted: false, span: t.span) }
        if check("string") { let s = try parseString(); return AstPathSegment(value: s.value, quoted: true, span: s.span) }
        throw SconError(.UnexpectedToken, "expected path segment", span: peek().span)
    }
    private func parseString() throws -> AstString { let token = try expect("string", "expected string"); let parsed = try parseStringParts(token); return AstString(value: parsed.value, raw: token.text, parts: parsed.parts, span: token.span) }
    private func parseStringParts(_ token: Token) throws -> (parts: [StringPart], value: String) {
        let raw = Array(token.text); var parts: [StringPart] = []; var out = ""; var value = ""; var i = 1
        while i < raw.count - 1 {
            let ch = raw[i]; i += 1
            if ch == "$" && i < raw.count && raw[i] == "{" { if !out.isEmpty { parts.append(.literal(out)); value += out; out = "" }; let pathStart = i + 1; guard let close = raw[pathStart...].firstIndex(of: "}") else { throw SconError(.UnterminatedString, "unterminated interpolation", span: token.span) }; parts.append(.interpolation(try parseInterpolationPath(String(raw[pathStart..<close]), token.span.start + pathStart), Span(start: token.span.start + i - 1, end: token.span.start + close + 1))); i = close + 1; continue }
            if ch != "\\" { out.append(ch); continue }
            let escaped = raw[i]; i += 1
            switch escaped { case "\"": out.append("\""); case "\\": out.append("\\"); case "/": out.append("/"); case "b": out.append("\u{0008}"); case "f": out.append("\u{000C}"); case "n": out.append("\n"); case "r": out.append("\r"); case "t": out.append("\t"); case "$": out.append("$"); case "u": out.append(Character(UnicodeScalar(Int(String(raw[i..<i+4]), radix: 16)!)!)); i += 4; default: throw SconError(.InvalidEscape, "invalid string escape", span: token.span) }
        }
        if !out.isEmpty || parts.isEmpty { parts.append(.literal(out)); value += out }
        return (parts, value)
    }
    private func parseInterpolationPath(_ text: String, _ base: Int) throws -> AstPath {
        let adjusted = try Lexer.lex(text).map { Token(kind: $0.kind, text: $0.text, span: Span(start: $0.span.start + base, end: $0.span.end + base)) }
        let parser = Parser(adjusted); let path = try parser.parsePath(); _ = try parser.expect("eof", "expected end of interpolation"); return path
    }
    private func invalidRoot() throws -> AstObject { throw SconError(.InvalidRootType, "SCON document root must be an object", span: peek().span) }
    private func skipTrivia() { while match("ws") || match("newline") || match("comment") {} }
    private func skipInlineTrivia() { while match("ws") || match("comment") {} }
    private func match(_ kind: String) -> Bool { if !check(kind) { return false }; index += 1; return true }
    private func check(_ kind: String) -> Bool { peek().kind == kind }
    private func expect(_ kind: String, _ message: String) throws -> Token { if check(kind) { index += 1; return previous() }; throw SconError(.UnexpectedToken, message, span: peek().span) }
    private func peek() -> Token { tokens[min(index, tokens.count - 1)] }
    private func previous() -> Token { tokens[index - 1] }
}
