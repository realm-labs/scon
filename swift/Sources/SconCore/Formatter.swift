import Foundation

enum Formatter {
    static func formatValue(_ value: SconValue) throws -> String {
        guard case .object(let object) = value else {
            throw SconError(.InvalidRootType, "SCON document root must be an object")
        }
        return formatObjectBody(object, 0) + "\n"
    }

    private static func formatObjectBody(_ object: SconObject, _ indent: Int) -> String {
        object.pairs.map {
            String(repeating: " ", count: indent) + formatKey($0.0) + " = " + formatScon($0.1, indent) + "\n"
        }.joined()
    }

    private static func formatScon(_ value: SconValue, _ indent: Int) -> String {
        switch value {
        case .null:
            return "null"
        case .bool(let bool):
            return bool ? "true" : "false"
        case .number(let number):
            return number.text
        case .string(let string):
            return quote(string, true)
        case .array(let array):
            return array.isEmpty ? "[]" : "[\n" + array.map {
                String(repeating: " ", count: indent + 2) + formatScon($0, indent + 2) + ",\n"
            }.joined() + String(repeating: " ", count: indent) + "]"
        case .object(let object):
            return object.count == 0 ? "{}" : "{\n" + formatObjectBody(object, indent + 2) + String(repeating: " ", count: indent) + "}"
        }
    }

    private static func formatKey(_ key: String) -> String {
        isUnquotedKey(key) ? key : quote(key, false)
    }

    private static func isUnquotedKey(_ key: String) -> Bool {
        !["include", "true", "false", "null"].contains(key)
            && key.range(of: #"^[A-Za-z_][A-Za-z0-9_-]*$"#, options: .regularExpression) != nil
    }

    private static func quote(_ value: String, _ escapeInterpolation: Bool) -> String {
        "\"" + value.flatMap { ch -> String in
            switch ch {
            case "\"": "\\\""
            case "\\": "\\\\"
            case "\n": "\\n"
            case "\r": "\\r"
            case "\t": "\\t"
            case "\u{0008}": "\\b"
            case "\u{000C}": "\\f"
            case "$" where escapeInterpolation: "\\$"
            case let ch where ch.unicodeScalars.allSatisfy({ $0.value < 0x20 }):
                "\\u" + String(format: "%04X", ch.unicodeScalars.first!.value)
            default: String(ch)
            }
        } + "\""
    }
}
