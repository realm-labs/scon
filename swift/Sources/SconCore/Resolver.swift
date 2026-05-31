import Foundation

final class Resolver {
    private final class EvalObject {
        var entries: [(String, EvalEntry)] = []
        var index: [String: Int] = [:]

        func get(_ key: String) -> EvalEntry? {
            index[key].map { entries[$0].1 }
        }

        func set(_ key: String, _ value: EvalEntry) {
            if let i = index[key] {
                entries[i] = (key, value)
            } else {
                index[key] = entries.count
                entries.append((key, value))
            }
        }
    }

    private final class EvalEntry {
        var value: Any
        var layer: String
        var kind: String

        init(_ value: Any, _ layer: String, _ kind: String) {
            self.value = value
            self.layer = layer
            self.kind = kind
        }
    }

    private var options: LoadOptions
    private var root = EvalObject()
    private var inProgress: [[String]] = [[]]
    var stack: [String] = []
    var seen: Set<String> = []
    private var cache: [String: Document] = [:]

    init(_ options: LoadOptions) {
        self.options = options
    }

    func eval(_ doc: Document) throws -> SconValue {
        try evalObject(doc.root, [], doc.file)
        return .object(publicObject(root))
    }

    private func evalObject(_ object: AstObject, _ path: [String], _ file: String?) throws {
        if path.count > options.maxObjectDepth {
            throw SconError(.ResourceLimitExceeded, "maximum object depth exceeded", span: object.span)
        }

        var localSeen = false
        for member in object.members {
            switch member {
            case .objectSpread(let spread):
                if localSeen {
                    throw SconError(.InvalidSpread, "object spread must appear before local members", span: spread.span)
                }
                guard let source = try lookup(spread.sub.path, spread.span).value as? EvalObject else {
                    throw SconError(.TypeMismatch, "object spread target is not an object", span: spread.span)
                }
                overlayBase(try objectAt(path, spread.span), source)
            case .include(let include):
                let included = try loadInclude(file, include)
                try evalObject(included.root, path, included.file)
            case .field(let field):
                localSeen = true
                try evalField(field, path, file)
            }
        }
    }

    private func evalField(_ field: AstField, _ current: [String], _ file: String?) throws {
        let target = current + field.path.segments.map(\.value)
        if case .object(let object, _) = field.value {
            try ensureObject(target, field.span)
            inProgress.append(target)
            defer { inProgress.removeLast() }
            try evalObject(object, target, file)
            return
        }
        try insert(target, evalValue(field.value, file), "ordinary", field.span)
    }

    private func evalValue(_ value: AstValue, _ file: String?) throws -> Any {
        switch value {
        case .null:
            return SconValue.null
        case .bool(let bool, _):
            return SconValue.bool(bool)
        case .number(let raw, let span):
            do {
                return SconValue.number(try SconNumber.parse(raw))
            } catch let error as SconError {
                throw SconError(error.code, error.message, span: span)
            }
        case .string(let string):
            return SconValue.string(try evalString(string))
        case .substitution(let sub):
            return cloneAny(try lookup(sub.path, sub.span).value)
        case .array(let array):
            return try evalArray(array, file)
        case .object(let object, _):
            let nested = Resolver(options)
            nested.stack = stack
            nested.seen = seen
            nested.cache = cache
            try nested.evalObject(object, [], file)
            return nested.root
        }
    }

    private func evalArray(_ array: AstArray, _ file: String?) throws -> SconValue {
        var out: [SconValue] = []
        for item in array.items {
            if out.count >= options.maxArrayLength {
                throw SconError(.ResourceLimitExceeded, "maximum array length exceeded", span: item.span)
            }
            switch item {
            case .value(let value, _):
                out.append(publicMaybe(try evalValue(value, file)))
            case .spread(let sub, let span):
                guard case .array(let values) = try lookup(sub.path, span).value as? SconValue else {
                    throw SconError(.TypeMismatch, "array spread target is not an array", span: span)
                }
                out.append(contentsOf: values.map(cloneValue))
            }
        }
        return .array(out)
    }

    private func evalString(_ string: AstString) throws -> String {
        if string.parts.count == 1, case .literal(let value) = string.parts[0] {
            return value
        }

        var out = ""
        for part in string.parts {
            switch part {
            case .literal(let value):
                out += value
            case .interpolation(let path, let span):
                let replacement = try lookup(path, span).value
                if case .string(let string) = replacement as? SconValue {
                    out += string
                } else if case .bool(let bool) = replacement as? SconValue {
                    out += bool ? "true" : "false"
                } else if case .number(let number) = replacement as? SconValue {
                    out += number.text
                } else {
                    throw SconError(.TypeMismatch, "interpolation requires string, number, or boolean", span: span)
                }
            }
        }
        return out
    }

    private func lookup(_ path: AstPath, _ span: Span) throws -> EvalEntry {
        let names = path.segments.map(\.value)
        if inProgress.contains(where: { $0 == names }) {
            throw SconError(.MissingReference, "reference is not completed yet", span: span)
        }

        var object = root
        var entry: EvalEntry?
        for (index, name) in names.enumerated() {
            entry = object.get(name)
            guard let current = entry else {
                throw SconError(.MissingReference, "missing reference '\(name)'", span: span)
            }
            if index < names.count - 1 {
                guard let next = current.value as? EvalObject else {
                    throw SconError(.TypeMismatch, "reference path crosses non-object value", span: span)
                }
                object = next
            }
        }
        return entry!
    }

    private func ensureObject(_ path: [String], _ span: Span) throws {
        var object = root
        for (index, name) in path.enumerated() {
            guard let entry = object.get(name) else {
                let child = EvalObject()
                object.set(name, EvalEntry(child, "local", "structural"))
                object = child
                continue
            }
            guard let next = entry.value as? EvalObject else {
                throw SconError(.PathConflict, "path conflicts with scalar value", span: span)
            }
            if index == path.count - 1 && entry.layer == "local" && entry.kind != "structural" {
                throw SconError(.PathConflict, "object field conflicts with ordinary value", span: span)
            }
            entry.layer = "local"
            entry.kind = "structural"
            object = next
        }
    }

    private func insert(_ path: [String], _ value: Any, _ kind: String, _ span: Span) throws {
        var object = root
        for name in path.dropLast() {
            if let entry = object.get(name) {
                guard let next = entry.value as? EvalObject else {
                    throw SconError(.PathConflict, "path conflicts with scalar value", span: span)
                }
                object = next
            } else {
                let child = EvalObject()
                object.set(name, EvalEntry(child, "local", "structural"))
                object = child
            }
        }

        let leaf = path.last!
        if let existing = object.get(leaf) {
            if existing.layer == "base" {
                overlayLocal(existing, value, kind)
            } else {
                throw SconError(.DuplicateKey, "duplicate key '\(leaf)'", span: span)
            }
        } else {
            object.set(leaf, EvalEntry(value, "local", kind))
        }
    }

    private func objectAt(_ path: [String], _ span: Span) throws -> EvalObject {
        var object = root
        for name in path {
            guard let entry = object.get(name) else {
                throw SconError(.PathConflict, "target object does not exist", span: span)
            }
            guard let next = entry.value as? EvalObject else {
                throw SconError(.PathConflict, "target path is not an object", span: span)
            }
            object = next
        }
        return object
    }

    private func loadInclude(_ file: String?, _ include: AstInclude) throws -> Document {
        let path = include.path.value
        if invalidIncludePath(path) {
            throw SconError(.InvalidIncludePath, "invalid include path", span: include.span)
        }

        let rootPath = URL(fileURLWithPath: options.includeRoot ?? (file.map { URL(fileURLWithPath: $0).deletingLastPathComponent().path } ?? ".")).standardizedFileURL.path
        let base = file.map { URL(fileURLWithPath: $0).deletingLastPathComponent().path } ?? rootPath
        let candidate = URL(fileURLWithPath: path, relativeTo: URL(fileURLWithPath: base)).standardizedFileURL.path
        if !candidate.hasPrefix(rootPath) {
            throw SconError(.IncludePathDenied, "include path escapes include root", span: include.span)
        }
        if stack.contains(candidate) {
            throw SconError(.IncludeCycle, "include cycle: \(candidate)", span: include.span)
        }
        if stack.count >= options.maxIncludeDepth {
            throw SconError(.ResourceLimitExceeded, "maximum include depth exceeded", span: include.span)
        }

        seen.insert(candidate)
        if seen.count > options.maxIncludeFiles {
            throw SconError(.ResourceLimitExceeded, "maximum include file count exceeded", span: include.span)
        }
        if let cached = cache[candidate] {
            return cached
        }

        var isDir: ObjCBool = false
        guard FileManager.default.fileExists(atPath: candidate, isDirectory: &isDir) else {
            throw SconError(.IncludeNotFound, "include file not found: \(candidate)", span: include.span)
        }
        if isDir.boolValue {
            throw SconError(.IncludeNotFile, "include path is not a file", span: include.span)
        }

        stack.append(candidate)
        defer { stack.removeLast() }
        do {
            let doc = try Parser.parseDocument(String(contentsOfFile: candidate, encoding: .utf8), file: candidate)
            cache[candidate] = doc
            return doc
        } catch let error as SconError {
            throw SconError(error.code == .InvalidRootType ? .IncludeRootTypeError : .IncludeParseError, error.message, span: error.span)
        }
    }

    private func invalidIncludePath(_ path: String) -> Bool {
        path.contains("://")
            || path.hasPrefix("classpath:")
            || path.contains("*")
            || path.hasPrefix("~")
            || path.hasPrefix("$")
            || path.hasPrefix("/")
            || path.range(of: #"^[A-Za-z]:[\\/]"#, options: .regularExpression) != nil
    }

    private func publicObject(_ object: EvalObject) -> SconObject {
        var out = SconObject()
        for (key, entry) in object.entries {
            out.set(key, publicMaybe(entry.value))
        }
        return out
    }

    private func publicMaybe(_ value: Any) -> SconValue {
        value is EvalObject ? .object(publicObject(value as! EvalObject)) : value as! SconValue
    }

    private func cloneAny(_ value: Any) -> Any {
        guard let object = value as? EvalObject else {
            return cloneValue(value as! SconValue)
        }

        let out = EvalObject()
        for (key, entry) in object.entries {
            out.set(key, EvalEntry(cloneAny(entry.value), entry.layer, entry.kind))
        }
        return out
    }

    private func cloneValue(_ value: SconValue) -> SconValue {
        switch value {
        case .array(let array):
            return .array(array.map(cloneValue))
        case .object(let object):
            var out = SconObject()
            for (key, value) in object.pairs {
                out.set(key, cloneValue(value))
            }
            return .object(out)
        default:
            return value
        }
    }

    private func overlayBase(_ target: EvalObject, _ source: EvalObject) {
        for (key, entry) in source.entries {
            if let existing = target.get(key) {
                if existing.layer == "base" {
                    overlayLocal(existing, cloneAny(entry.value), entry.kind)
                }
            } else {
                target.set(key, EvalEntry(cloneAny(entry.value), "base", "ordinary"))
            }
        }
    }

    private func overlayLocal(_ existing: EvalEntry, _ value: Any, _ kind: String) {
        if let target = existing.value as? EvalObject, let source = value as? EvalObject {
            mergeOverride(target, source)
            existing.layer = "local"
            existing.kind = kind
        } else {
            existing.value = value
            existing.layer = "local"
            existing.kind = kind
        }
    }

    private func mergeOverride(_ target: EvalObject, _ source: EvalObject) {
        for (key, entry) in source.entries {
            if let existing = target.get(key), let targetObject = existing.value as? EvalObject, let sourceObject = entry.value as? EvalObject {
                mergeOverride(targetObject, sourceObject)
            } else {
                target.set(key, EvalEntry(cloneAny(entry.value), entry.layer, entry.kind))
            }
        }
    }
}
