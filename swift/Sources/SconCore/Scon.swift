import Foundation

public enum Scon {
    public static func parseString(_ source: String) throws -> SconValue { try Resolver(.default).eval(Parser.parseDocument(source)) }
    public static func parseFile(_ path: String, options: LoadOptions = .default) throws -> SconValue {
        var opts = options
        let file = URL(fileURLWithPath: path).standardizedFileURL.path
        if opts.includeRoot == nil { opts.includeRoot = URL(fileURLWithPath: file).deletingLastPathComponent().path }
        let resolver = Resolver(opts); resolver.stack.append(file); resolver.seen.insert(file)
        return try resolver.eval(Parser.parseDocument(String(contentsOfFile: file, encoding: .utf8), file: file))
    }
    public static func formatValue(_ value: SconValue) throws -> String { try Formatter.formatValue(value) }
    public static func getPath(_ value: SconValue, _ path: String) throws -> SconValue { var current = value; for segment in path.split(separator: ".").map(String.init) { guard case .object(let object) = current else { throw SconError(.TypeMismatch, "path segment requires object") }; guard let next = object[segment] else { throw SconError(.MissingReference, "path is not defined") }; current = next }; return current }
    public static func decode<T: Decodable>(_ source: String, as type: T.Type) throws -> T { try JSONDecoder().decode(T.self, from: JSONSerialization.data(withJSONObject: plain(try parseString(source)))) }
    public static func encode<T: Encodable>(_ value: T) throws -> String { let any = try JSONSerialization.jsonObject(with: JSONEncoder().encode(value)); return try formatValue(fromPlain(any)) }
    public static func plain(_ value: SconValue) -> Any { switch value { case .null: return NSNull(); case .bool(let b): return b; case .number(let n): return n.plain; case .string(let s): return s; case .array(let a): return a.map(plain); case .object(let o): var dict: [String: Any] = [:]; for (k, v) in o.pairs { dict[k] = plain(v) }; return dict } }
    private static func fromPlain(_ value: Any) throws -> SconValue { if value is NSNull { return .null }; if let b = value as? Bool { return .bool(b) }; if let s = value as? String { return .string(s) }; if let i = value as? Int { return .number(i < 0 ? .i64(Int64(i)) : .u64(UInt64(i))) }; if let d = value as? Double { guard d.isFinite else { throw SconError(.Serde, "non-finite floats cannot be serialized") }; return d.rounded() == d && d >= 0 ? .number(.u64(UInt64(d))) : .number(.f64(d)) }; if let a = value as? [Any] { return .array(try a.map(fromPlain)) }; if let dict = value as? [String: Any] { var obj = SconObject(); for (k, v) in dict { obj.set(k, try fromPlain(v)) }; return .object(obj) }; throw SconError(.Serde, "unsupported value") }
}
