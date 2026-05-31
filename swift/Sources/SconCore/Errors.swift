import Foundation

public enum ErrorCode: String, Error, Codable {
    case InvalidCharacter, InvalidWhitespace, InvalidEscape, UnexpectedToken, UnterminatedString, InvalidNumber, InvalidRootType
    case DuplicateKey, PathConflict, MissingReference, TypeMismatch, InvalidSpread
    case InvalidIncludePath, IncludeNotFound, IncludeNotFile, IncludePathDenied, IncludeCycle, IncludeParseError, IncludeRootTypeError
    case ResourceLimitExceeded, Serde
}

public struct Span: Equatable, Sendable { public let start: Int; public let end: Int }

public struct SconError: Error, CustomStringConvertible {
    public let code: ErrorCode
    public let message: String
    public let span: Span?
    public init(_ code: ErrorCode, _ message: String, span: Span? = nil) { self.code = code; self.message = message; self.span = span }
    public var description: String { "\(code.rawValue): \(message)" }
}
