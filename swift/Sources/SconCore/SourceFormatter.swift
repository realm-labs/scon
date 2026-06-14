import Foundation

enum SourceFormatter {
    static func formatSource(_ source: String) throws -> String {
        let document = try Parser.parseDocument(source)
        let comments = try Lexer.lex(source)
            .filter { $0.kind == "comment" }
            .map { $0.text + "\n" }
            .joined()
        return comments + formatObjectBody(document.root, 0) + "\n"
    }

    private static func formatObjectBody(_ object: AstObject, _ indent: Int) -> String {
        object.members.map { String(repeating: " ", count: indent) + formatMember($0, indent) + "\n" }.joined()
    }

    private static func formatMember(_ member: AstMember, _ indent: Int) -> String {
        switch member {
        case .include(let include):
            "include \(include.path.raw)"
        case .objectSpread(let spread):
            "...\(formatSubstitution(spread.sub))"
        case .field(let field):
            "\(formatPath(field.path)) = \(formatValue(field.value, indent))"
        }
    }

    private static func formatValue(_ value: AstValue, _ indent: Int) -> String {
        switch value {
        case .null:
            "null"
        case .bool(let value, _):
            value ? "true" : "false"
        case .number(let raw, _):
            raw
        case .string(let value):
            value.raw
        case .substitution(let value):
            formatSubstitution(value)
        case .array(let array):
            formatArray(array, indent)
        case .object(let object, _):
            object.members.isEmpty ? "{}" : "{\n" + formatObjectBody(object, indent + 2) + String(repeating: " ", count: indent) + "}"
        }
    }

    private static func formatArray(_ array: AstArray, _ indent: Int) -> String {
        if array.items.isEmpty { return "[]" }
        return "[\n" + array.items.map { item in
            let content: String
            switch item {
            case .value(let value, _):
                content = formatValue(value, indent + 2)
            case .spread(let substitution, _):
                content = "..." + formatSubstitution(substitution)
            }
            return String(repeating: " ", count: indent + 2) + content + ",\n"
        }.joined() + String(repeating: " ", count: indent) + "]"
    }

    private static func formatSubstitution(_ substitution: AstSubstitution) -> String {
        "${\(formatPath(substitution.path))}"
    }

    private static func formatPath(_ path: AstPath) -> String {
        path.segments.map { $0.quoted ? quote($0.value) : $0.value }.joined(separator: ".")
    }

    private static func quote(_ value: String) -> String {
        "\"" + value.flatMap { ch -> String in
            switch ch {
            case "\"": "\\\""
            case "\\": "\\\\"
            case "\n": "\\n"
            case "\r": "\\r"
            case "\t": "\\t"
            case "\u{0008}": "\\b"
            case "\u{000C}": "\\f"
            default: String(ch)
            }
        } + "\""
    }
}
