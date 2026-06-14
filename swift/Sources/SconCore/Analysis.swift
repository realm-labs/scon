import Foundation

public struct SourcePosition: Equatable, Sendable { public let line: Int; public let column: Int }
public struct SourceRange: Equatable, Sendable { public let start: SourcePosition; public let end: SourcePosition; public let span: Span }
public struct SconComment: Equatable { public let text: String; public let span: Span; public let range: SourceRange }
public enum DiagnosticSeverity: String { case error, warning, information, hint }
public struct SconDiagnostic { public let code: ErrorCode; public let message: String; public let severity: DiagnosticSeverity; public let file: String?; public let range: SourceRange? }
public struct TokenInfo { public let kind: String; public let text: String; public let span: Span; public let range: SourceRange }
public struct SconSymbol { public let path: [String]; public let file: String?; public let range: SourceRange }
public struct SconDefinition { public let path: [String]; public let file: String?; public let range: SourceRange }
public enum SconReferenceKind: String { case substitution, interpolation, objectSpread, arraySpread }
public struct SconReference { public let path: [String]; public let kind: SconReferenceKind; public let file: String?; public let range: SourceRange; public var target: SconDefinition? }
public struct SconIncludeReference { public let path: String; public let file: String?; public let range: SourceRange; public let resolvedPath: String? }
public struct ParsedSource { public let file: String?; public let tokens: [TokenInfo]; public let comments: [SconComment]; public let symbols: [SconSymbol] }
public struct SconAnalysis {
    public let file: String?
    public let parsed: ParsedSource?
    public let diagnostics: [SconDiagnostic]
    public let comments: [SconComment]
    public let symbols: [SconSymbol]
    public let definitions: [SconDefinition]
    public let references: [SconReference]
    public let includes: [SconIncludeReference]
    public let value: SconValue?
}

final class LineIndex {
    private var lines = [0]

    init(_ source: String) {
        for (index, char) in source.enumerated() where char == "\n" {
            lines.append(index + 1)
        }
    }

    func range(_ span: Span) -> SourceRange {
        SourceRange(start: position(span.start), end: position(span.end), span: span)
    }

    private func position(_ offset: Int) -> SourcePosition {
        var line = 0
        while line + 1 < lines.count && lines[line + 1] <= offset { line += 1 }
        return SourcePosition(line: line, column: offset - lines[line])
    }
}

enum Analyzer {
    static func parseSource(_ source: String, file: String? = nil) throws -> ParsedSource {
        let document = try Parser.parseDocument(source, file: file)
        let lineIndex = LineIndex(source)
        let tokens = try Lexer.lex(source).map { TokenInfo(kind: $0.kind, text: $0.text, span: $0.span, range: lineIndex.range($0.span)) }
        let comments = tokens.filter { $0.kind == "comment" }.map { SconComment(text: $0.text, span: $0.span, range: $0.range) }
        return ParsedSource(file: file, tokens: tokens, comments: comments, symbols: symbols(document.root, lineIndex, file, []))
    }

    static func analyzeSource(_ source: String, file: String? = nil) -> SconAnalysis {
        let lineIndex = LineIndex(source)
        let tokens = (try? Lexer.lex(source).map { TokenInfo(kind: $0.kind, text: $0.text, span: $0.span, range: lineIndex.range($0.span)) }) ?? []
        let comments = tokens.filter { $0.kind == "comment" }.map { SconComment(text: $0.text, span: $0.span, range: $0.range) }
        do {
            let document = try Parser.parseDocument(source, file: file)
            let parsed = ParsedSource(file: file, tokens: tokens, comments: comments, symbols: symbols(document.root, lineIndex, file, []))
            let definitions = definitions(document.root, lineIndex, file, [])
            var references = references(document.root, lineIndex, file)
            resolveTargets(&references, definitions)
            var diagnostics: [SconDiagnostic] = []
            var value: SconValue?
            do { value = try Scon.parseString(source) }
            catch let error as SconError { diagnostics.append(diagnostic(error, lineIndex, file)) }
            catch { diagnostics.append(SconDiagnostic(code: .Serde, message: String(describing: error), severity: .error, file: file, range: nil)) }
            return SconAnalysis(file: file, parsed: parsed, diagnostics: diagnostics, comments: comments, symbols: parsed.symbols, definitions: definitions, references: references, includes: includes(document.root, lineIndex, file), value: value)
        } catch let error as SconError {
            return SconAnalysis(file: file, parsed: nil, diagnostics: [diagnostic(error, lineIndex, file)], comments: comments, symbols: [], definitions: [], references: [], includes: [], value: nil)
        } catch {
            return SconAnalysis(file: file, parsed: nil, diagnostics: [SconDiagnostic(code: .Serde, message: String(describing: error), severity: .error, file: file, range: nil)], comments: comments, symbols: [], definitions: [], references: [], includes: [], value: nil)
        }
    }

    private static func symbols(_ object: AstObject, _ lineIndex: LineIndex, _ file: String?, _ prefix: [String]) -> [SconSymbol] {
        object.members.flatMap { member -> [SconSymbol] in
            guard case .field(let field) = member else { return [] }
            let path = prefix + names(field.path)
            let nested: [SconSymbol] = {
                if case .object(let object, _) = field.value { return symbols(object, lineIndex, file, path) }
                return []
            }()
            return [SconSymbol(path: path, file: file, range: lineIndex.range(field.path.span))] + nested
        }
    }

    private static func definitions(_ object: AstObject, _ lineIndex: LineIndex, _ file: String?, _ prefix: [String]) -> [SconDefinition] {
        object.members.flatMap { member -> [SconDefinition] in
            guard case .field(let field) = member else { return [] }
            let path = prefix + names(field.path)
            let nested: [SconDefinition] = {
                if case .object(let object, _) = field.value { return definitions(object, lineIndex, file, path) }
                return []
            }()
            return [SconDefinition(path: path, file: file, range: lineIndex.range(field.path.span))] + nested
        }
    }

    private static func references(_ object: AstObject, _ lineIndex: LineIndex, _ file: String?) -> [SconReference] {
        object.members.flatMap { member -> [SconReference] in
            switch member {
            case .objectSpread(let spread):
                return [reference(spread.sub.path, .objectSpread, lineIndex, file)]
            case .field(let field):
                return valueReferences(field.value, lineIndex, file)
            case .include:
                return []
            }
        }
    }

    private static func valueReferences(_ value: AstValue, _ lineIndex: LineIndex, _ file: String?) -> [SconReference] {
        switch value {
        case .substitution(let substitution):
            return [reference(substitution.path, .substitution, lineIndex, file)]
        case .string(let string):
            return string.parts.compactMap {
                if case .interpolation(let path, _) = $0 { return reference(path, .interpolation, lineIndex, file) }
                return nil
            }
        case .array(let array):
            return array.items.flatMap { item -> [SconReference] in
                switch item {
                case .spread(let substitution, _): return [reference(substitution.path, .arraySpread, lineIndex, file)]
                case .value(let value, _): return valueReferences(value, lineIndex, file)
                }
            }
        case .object(let object, _):
            return references(object, lineIndex, file)
        default:
            return []
        }
    }

    private static func includes(_ object: AstObject, _ lineIndex: LineIndex, _ file: String?) -> [SconIncludeReference] {
        object.members.flatMap { member -> [SconIncludeReference] in
            switch member {
            case .include(let include):
                return [SconIncludeReference(path: include.path.value, file: file, range: lineIndex.range(include.span), resolvedPath: nil)]
            case .field(let field):
                if case .object(let object, _) = field.value { return includes(object, lineIndex, file) }
                return []
            case .objectSpread:
                return []
            }
        }
    }

    private static func reference(_ path: AstPath, _ kind: SconReferenceKind, _ lineIndex: LineIndex, _ file: String?) -> SconReference {
        SconReference(path: names(path), kind: kind, file: file, range: lineIndex.range(path.span))
    }

    private static func resolveTargets(_ references: inout [SconReference], _ definitions: [SconDefinition]) {
        var byPath: [String: SconDefinition] = [:]
        for definition in definitions {
            byPath[definition.path.joined(separator: "\u{0}")] = definition
        }
        for index in references.indices {
            references[index].target = byPath[references[index].path.joined(separator: "\u{0}")]
        }
    }

    private static func names(_ path: AstPath) -> [String] {
        path.segments.map(\.value)
    }

    private static func diagnostic(_ error: SconError, _ lineIndex: LineIndex, _ file: String?) -> SconDiagnostic {
        SconDiagnostic(code: error.code, message: error.message, severity: .error, file: file, range: error.span.map(lineIndex.range))
    }
}
