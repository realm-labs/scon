package io.github.realmlabs.scon;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;

final class Resolver {
    final List<Path> stack = new ArrayList<>();
    final Set<Path> seen = new HashSet<>();
    private final LoadOptions options;
    private final Map<Path, Ast.Document> cache = new HashMap<>();
    private final EvalObject root = new EvalObject();
    private final List<List<String>> inProgress = new ArrayList<>(List.of(List.of()));

    Resolver(LoadOptions options) {
        this.options = options;
    }

    SconValue eval(Ast.Document document) {
        evalObject(document.root(), List.of(), document.file());
        return publicObject(root);
    }

    private void evalObject(Ast.ObjectNode object, List<String> path, String file) {
        if (path.size() > options.maxObjectDepth()) {
            throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum object depth exceeded", object.span());
        }
        boolean localSeen = false;
        for (Ast.Member member : object.members()) {
            if (member instanceof Ast.ObjectSpread spread) {
                if (localSeen) throw new SconException(ErrorCode.InvalidSpread, "object spread must appear before local members", spread.span());
                EvalEntry target = lookup(spread.sub().path(), spread.span());
                if (!(target.value instanceof EvalObject source)) {
                    throw new SconException(ErrorCode.TypeMismatch, "object spread target is not an object", spread.span());
                }
                overlayBase(objectAt(path, spread.span()), source);
            } else if (member instanceof Ast.Include include) {
                Ast.Document included = loadInclude(file, include);
                evalObject(included.root(), path, included.file());
            } else if (member instanceof Ast.Field field) {
                localSeen = true;
                evalField(field, path, file);
            }
        }
    }

    private void evalField(Ast.Field field, List<String> current, String file) {
        var target = new ArrayList<>(current);
        for (var segment : field.path().segments()) target.add(segment.value());
        if (field.value() instanceof Ast.ObjectValueNode objectValue) {
            ensureObject(target, field.span());
            inProgress.add(target);
            try {
                evalObject(objectValue.object(), target, file);
            } finally {
                inProgress.remove(inProgress.size() - 1);
            }
            return;
        }
        insert(target, evalValue(field.value(), file), "ordinary", field.span());
    }

    private Object evalValue(Ast.ValueNode value, String file) {
        if (value instanceof Ast.NullNode) return SconNull.INSTANCE;
        if (value instanceof Ast.BoolNode bool) return new SconBool(bool.value());
        if (value instanceof Ast.NumberNode number) {
            try {
                return SconNumber.parse(number.raw());
            } catch (SconException ex) {
                throw new SconException(ex.code(), ex.getMessage(), number.span());
            }
        }
        if (value instanceof Ast.StringNode string) return new SconString(evalString(string));
        if (value instanceof Ast.SubstitutionNode substitution) return cloneAny(lookup(substitution.path(), substitution.span()).value);
        if (value instanceof Ast.ArrayNode array) return evalArray(array, file);
        if (value instanceof Ast.ObjectValueNode objectValue) {
            var nested = new Resolver(options);
            nested.stack.addAll(stack);
            nested.seen.addAll(seen);
            nested.cache.putAll(cache);
            nested.evalObject(objectValue.object(), List.of(), file);
            return nested.root;
        }
        throw new SconException(ErrorCode.UnexpectedToken, "unknown value");
    }

    private SconArray evalArray(Ast.ArrayNode array, String file) {
        var out = new SconArray();
        for (Ast.ArrayItem item : array.items()) {
            if (out.size() >= options.maxArrayLength()) {
                throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum array length exceeded", item.span());
            }
            if (item instanceof Ast.ArrayValue value) {
                out.add(publicMaybe(evalValue(value.value(), file)));
            } else if (item instanceof Ast.ArraySpread spread) {
                Object target = lookup(spread.sub().path(), spread.span()).value;
                if (!(target instanceof SconArray values)) {
                    throw new SconException(ErrorCode.TypeMismatch, "array spread target is not an array", spread.span());
                }
                for (SconValue value : values) out.add(cloneValue(value));
            }
        }
        return out;
    }

    private String evalString(Ast.StringNode string) {
        if (string.parts().size() == 1 && string.parts().get(0) instanceof Ast.StringLiteral literal) {
            return literal.value();
        }
        var out = new StringBuilder();
        for (Ast.StringPart part : string.parts()) {
            if (part instanceof Ast.StringLiteral literal) {
                out.append(literal.value());
            } else if (part instanceof Ast.StringInterpolation interpolation) {
                Object replacement = lookup(interpolation.path(), interpolation.span()).value;
                if (replacement instanceof SconString s) out.append(s.value());
                else if (replacement instanceof SconBool b) out.append(b.value() ? "true" : "false");
                else if (replacement instanceof SconNumber n) out.append(n.toSconString());
                else throw new SconException(ErrorCode.TypeMismatch, "interpolation requires string, number, or boolean", interpolation.span());
            }
        }
        return out.toString();
    }

    private EvalEntry lookup(Ast.PathNode path, Span span) {
        var names = path.segments().stream().map(Ast.PathSegment::value).toList();
        for (var active : inProgress) {
            if (active.equals(names)) throw new SconException(ErrorCode.MissingReference, "reference is not completed yet", span);
        }
        EvalObject object = root;
        EvalEntry entry = null;
        for (int i = 0; i < names.size(); i++) {
            entry = object.entries.get(names.get(i));
            if (entry == null) throw new SconException(ErrorCode.MissingReference, "missing reference '" + names.get(i) + "'", span);
            if (i < names.size() - 1) {
                if (!(entry.value instanceof EvalObject next)) {
                    throw new SconException(ErrorCode.TypeMismatch, "reference path crosses non-object value", span);
                }
                object = next;
            }
        }
        return entry;
    }

    private void ensureObject(List<String> path, Span span) {
        EvalObject object = root;
        for (int i = 0; i < path.size(); i++) {
            String name = path.get(i);
            EvalEntry entry = object.entries.get(name);
            if (entry == null) {
                var child = new EvalObject();
                object.entries.put(name, new EvalEntry(child, "local", "structural"));
                object = child;
                continue;
            }
            if (!(entry.value instanceof EvalObject next)) throw new SconException(ErrorCode.PathConflict, "path conflicts with scalar value", span);
            if (i == path.size() - 1 && entry.layer.equals("local") && !entry.kind.equals("structural")) {
                throw new SconException(ErrorCode.PathConflict, "object field conflicts with ordinary value", span);
            }
            entry.layer = "local";
            entry.kind = "structural";
            object = next;
        }
    }

    private void insert(List<String> path, Object value, String kind, Span span) {
        EvalObject object = root;
        for (int i = 0; i < path.size() - 1; i++) {
            String name = path.get(i);
            EvalEntry entry = object.entries.get(name);
            if (entry == null) {
                var child = new EvalObject();
                object.entries.put(name, new EvalEntry(child, "local", "structural"));
                object = child;
            } else {
                if (!(entry.value instanceof EvalObject next)) throw new SconException(ErrorCode.PathConflict, "path conflicts with scalar value", span);
                object = next;
            }
        }
        String leaf = path.get(path.size() - 1);
        EvalEntry existing = object.entries.get(leaf);
        if (existing == null) {
            object.entries.put(leaf, new EvalEntry(value, "local", kind));
        } else if (existing.layer.equals("base")) {
            overlayLocal(existing, value, kind);
        } else {
            throw new SconException(ErrorCode.DuplicateKey, "duplicate key '" + leaf + "'", span);
        }
    }

    private EvalObject objectAt(List<String> path, Span span) {
        EvalObject object = root;
        for (String name : path) {
            EvalEntry entry = object.entries.get(name);
            if (entry == null) throw new SconException(ErrorCode.PathConflict, "target object does not exist", span);
            if (!(entry.value instanceof EvalObject next)) throw new SconException(ErrorCode.PathConflict, "target path is not an object", span);
            object = next;
        }
        return object;
    }

    private Ast.Document loadInclude(String file, Ast.Include include) {
        String path = include.path().value();
        if (invalidIncludePath(path)) throw new SconException(ErrorCode.InvalidIncludePath, "invalid include path", include.span());
        Path rootPath = (options.includeRoot() != null ? options.includeRoot() : (file == null ? Path.of(".") : Path.of(file).getParent())).toAbsolutePath().normalize();
        Path base = file == null ? rootPath : Path.of(file).getParent();
        Path candidate = base.resolve(path).toAbsolutePath().normalize();
        if (!candidate.startsWith(rootPath)) throw new SconException(ErrorCode.IncludePathDenied, "include path escapes include root", include.span());
        if (stack.contains(candidate)) throw new SconException(ErrorCode.IncludeCycle, "include cycle: " + candidate, include.span());
        if (stack.size() >= options.maxIncludeDepth()) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum include depth exceeded", include.span());
        seen.add(candidate);
        if (seen.size() > options.maxIncludeFiles()) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum include file count exceeded", include.span());
        if (cache.containsKey(candidate)) return cache.get(candidate);
        if (!Files.exists(candidate)) throw new SconException(ErrorCode.IncludeNotFound, "include file not found: " + candidate, include.span());
        if (!Files.isRegularFile(candidate)) throw new SconException(ErrorCode.IncludeNotFile, "include path is not a file", include.span());
        try {
            if (Files.size(candidate) > options.maxFileSize()) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum file size exceeded", include.span());
            stack.add(candidate);
            try {
                var doc = Parser.parseDocument(Files.readString(candidate), candidate.toString());
                cache.put(candidate, doc);
                return doc;
            } catch (SconException ex) {
                ErrorCode code = ex.code() == ErrorCode.InvalidRootType ? ErrorCode.IncludeRootTypeError : ErrorCode.IncludeParseError;
                throw new SconException(code, ex.getMessage(), ex.span());
            } finally {
                stack.remove(stack.size() - 1);
            }
        } catch (IOException ex) {
            throw new SconException(ErrorCode.IncludeNotFound, "include file not found: " + ex.getMessage(), include.span());
        }
    }

    private static SconObject publicObject(EvalObject object) {
        var out = new SconObject();
        for (var entry : object.entries.entrySet()) out.put(entry.getKey(), publicMaybe(entry.getValue().value));
        return out;
    }

    private static SconValue publicMaybe(Object value) {
        return value instanceof EvalObject object ? publicObject(object) : (SconValue) value;
    }

    private static Object cloneAny(Object value) {
        if (value instanceof EvalObject object) {
            var out = new EvalObject();
            for (var entry : object.entries.entrySet()) {
                var e = entry.getValue();
                out.entries.put(entry.getKey(), new EvalEntry(cloneAny(e.value), e.layer, e.kind));
            }
            return out;
        }
        return cloneValue((SconValue) value);
    }

    private static SconValue cloneValue(SconValue value) {
        if (value instanceof SconArray array) {
            var out = new SconArray();
            for (SconValue item : array) out.add(cloneValue(item));
            return out;
        }
        if (value instanceof SconObject object) {
            var out = new SconObject();
            for (var entry : object.entrySet()) out.put(entry.getKey(), cloneValue(entry.getValue()));
            return out;
        }
        return value;
    }

    private static void overlayBase(EvalObject target, EvalObject source) {
        for (var entry : source.entries.entrySet()) {
            EvalEntry existing = target.entries.get(entry.getKey());
            if (existing == null) target.entries.put(entry.getKey(), new EvalEntry(cloneAny(entry.getValue().value), "base", "ordinary"));
            else if (existing.layer.equals("base")) overlayLocal(existing, cloneAny(entry.getValue().value), entry.getValue().kind);
        }
    }

    private static void overlayLocal(EvalEntry existing, Object value, String kind) {
        if (existing.value instanceof EvalObject a && value instanceof EvalObject b) {
            mergeOverride(a, b);
            existing.layer = "local";
            existing.kind = kind;
        } else {
            existing.value = value;
            existing.layer = "local";
            existing.kind = kind;
        }
    }

    private static void mergeOverride(EvalObject target, EvalObject source) {
        for (var entry : source.entries.entrySet()) {
            EvalEntry existing = target.entries.get(entry.getKey());
            if (existing != null && existing.value instanceof EvalObject a && entry.getValue().value instanceof EvalObject b) mergeOverride(a, b);
            else target.entries.put(entry.getKey(), new EvalEntry(cloneAny(entry.getValue().value), entry.getValue().layer, entry.getValue().kind));
        }
    }

    private static boolean invalidIncludePath(String path) {
        return hasPathControlChar(path) || path.contains("://") || path.startsWith("classpath:") || path.contains("*") || path.startsWith("~")
            || path.startsWith("$") || Path.of(path).isAbsolute() || path.matches("^[A-Za-z]:[\\\\/].*");
    }

    private static boolean hasPathControlChar(String path) {
        for (int index = 0; index < path.length(); index++) {
            if (path.charAt(index) < 0x20) return true;
        }
        return false;
    }

    private static final class EvalObject {
        final LinkedHashMap<String, EvalEntry> entries = new LinkedHashMap<>();
    }

    private static final class EvalEntry {
        Object value;
        String layer;
        String kind;

        EvalEntry(Object value, String layer, String kind) {
            this.value = value;
            this.layer = layer;
            this.kind = kind;
        }
    }
}
