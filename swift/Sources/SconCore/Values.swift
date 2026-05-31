import Foundation

public enum SconNumber: Equatable {
    case i64(Int64)
    case u64(UInt64)
    case f64(Double)
    static func parse(_ raw: String) throws -> SconNumber {
        if raw.contains(".") || raw.contains("e") || raw.contains("E") {
            guard let value = Double(raw), value.isFinite else { throw SconError(.InvalidNumber, "invalid SCON number \(raw)") }
            return .f64(value)
        }
        if raw.hasPrefix("-") {
            guard let value = Int64(raw) else { throw SconError(.InvalidNumber, "invalid SCON number \(raw)") }
            return .i64(value)
        }
        guard let value = UInt64(raw) else { throw SconError(.InvalidNumber, "invalid SCON number \(raw)") }
        return .u64(value)
    }
    var text: String {
        switch self {
        case .i64(let value): return String(value)
        case .u64(let value): return String(value)
        case .f64(let value): return String(value)
        }
    }
    var plain: Any {
        switch self {
        case .i64(let value): return value
        case .u64(let value): return value <= UInt64(Int64.max) ? Int64(value) : String(value)
        case .f64(let value): return value
        }
    }
}

public enum SconValue: Equatable {
    case null
    case bool(Bool)
    case number(SconNumber)
    case string(String)
    case array([SconValue])
    case object(SconObject)
}

public struct SconObject: Equatable {
    private var entries: [(String, SconValue)] = []
    private var index: [String: Int] = [:]
    public init() {}
    public var count: Int { entries.count }
    public var pairs: [(String, SconValue)] { entries }
    public subscript(_ key: String) -> SconValue? { get { index[key].map { entries[$0].1 } } set { if let value = newValue { set(key, value) } } }
    public mutating func set(_ key: String, _ value: SconValue) {
        if let i = index[key] { entries[i] = (key, value) } else { index[key] = entries.count; entries.append((key, value)) }
    }
    public static func == (lhs: SconObject, rhs: SconObject) -> Bool {
        lhs.entries.elementsEqual(rhs.entries) { $0.0 == $1.0 && $0.1 == $1.1 }
    }
}

public struct LoadOptions: Sendable {
    public var includeRoot: String?
    public var maxFileSize = 16 * 1024 * 1024
    public var maxIncludeDepth = 64
    public var maxIncludeFiles = 1024
    public var maxArrayLength = 1_000_000
    public var maxObjectDepth = 512
    public static let `default` = LoadOptions()
    public init(includeRoot: String? = nil) { self.includeRoot = includeRoot }
}
