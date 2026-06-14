package io.github.realmlabs.scon;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

public final class Analysis {
    public record SourcePosition(int line, int column) {}
    public record SourceRange(SourcePosition start, SourcePosition end, Span span) {}
    public record Comment(String text, Span span, SourceRange range) {}
    public enum DiagnosticSeverity { Error, Warning, Information, Hint }
    public record Diagnostic(ErrorCode code, String message, DiagnosticSeverity severity, String file, SourceRange range) {}
    public record TokenInfo(String kind, String text, Span span, SourceRange range) {}
    public record Symbol(List<String> path, String file, SourceRange range) {}
    public record Definition(List<String> path, String file, SourceRange range) {}
    public enum ReferenceKind { Substitution, Interpolation, ObjectSpread, ArraySpread }
    public record Reference(List<String> path, ReferenceKind kind, String file, SourceRange range, Definition target) {
        Reference withTarget(Definition target) { return new Reference(path, kind, file, range, target); }
    }
    public record IncludeReference(String path, String file, SourceRange range, String resolvedPath) {}
    public record ParsedSource(String file, List<TokenInfo> tokens, List<Comment> comments, List<Symbol> symbols) {}
    public record DocumentAnalysis(
        String file,
        ParsedSource parsed,
        List<Diagnostic> diagnostics,
        List<Comment> comments,
        List<Symbol> symbols,
        List<Definition> definitions,
        List<Reference> references,
        List<IncludeReference> includes,
        SconValue value
    ) {}

    private Analysis() {}

    public static ParsedSource parseSource(String source, String file) {
        var document = Parser.parseDocument(source, file);
        var lineIndex = new LineIndex(source);
        var tokens = Lexer.lex(source).stream().map(t -> new TokenInfo(t.kind(), t.text(), t.span(), lineIndex.range(t.span()))).toList();
        var comments = comments(tokens);
        return new ParsedSource(file, tokens, comments, symbols(document.root(), lineIndex, file, List.of()));
    }

    public static DocumentAnalysis analyzeSource(String source, String file) {
        var lineIndex = new LineIndex(source);
        List<TokenInfo> tokens = List.of();
        try {
            tokens = Lexer.lex(source).stream().map(t -> new TokenInfo(t.kind(), t.text(), t.span(), lineIndex.range(t.span()))).toList();
            var document = Parser.parseDocument(source, file);
            var comments = comments(tokens);
            var parsed = new ParsedSource(file, tokens, comments, symbols(document.root(), lineIndex, file, List.of()));
            var definitions = definitions(document.root(), lineIndex, file, List.of());
            var references = resolveTargets(references(document.root(), lineIndex, file), definitions);
            var diagnostics = new ArrayList<Diagnostic>();
            SconValue value = null;
            try {
                value = Scon.parseString(source);
            } catch (SconException ex) {
                diagnostics.add(diagnostic(ex, lineIndex, file));
            }
            return new DocumentAnalysis(file, parsed, diagnostics, comments, parsed.symbols(), definitions, references, includes(document.root(), lineIndex, file), value);
        } catch (SconException ex) {
            var comments = comments(tokens);
            return new DocumentAnalysis(file, null, List.of(diagnostic(ex, lineIndex, file)), comments, List.of(), List.of(), List.of(), List.of(), null);
        }
    }

    private static List<Symbol> symbols(Ast.ObjectNode object, LineIndex lineIndex, String file, List<String> prefix) {
        var out = new ArrayList<Symbol>();
        for (var member : object.members()) {
            if (member instanceof Ast.Field field) {
                var path = append(prefix, names(field.path()));
                out.add(new Symbol(path, file, lineIndex.range(field.path().span())));
                if (field.value() instanceof Ast.ObjectValueNode nested) out.addAll(symbols(nested.object(), lineIndex, file, path));
            }
        }
        return out;
    }

    private static List<Definition> definitions(Ast.ObjectNode object, LineIndex lineIndex, String file, List<String> prefix) {
        var out = new ArrayList<Definition>();
        for (var member : object.members()) {
            if (member instanceof Ast.Field field) {
                var path = append(prefix, names(field.path()));
                out.add(new Definition(path, file, lineIndex.range(field.path().span())));
                if (field.value() instanceof Ast.ObjectValueNode nested) out.addAll(definitions(nested.object(), lineIndex, file, path));
            }
        }
        return out;
    }

    private static List<Reference> references(Ast.ObjectNode object, LineIndex lineIndex, String file) {
        var out = new ArrayList<Reference>();
        for (var member : object.members()) {
            if (member instanceof Ast.ObjectSpread spread) out.add(reference(spread.sub().path(), ReferenceKind.ObjectSpread, lineIndex, file));
            if (member instanceof Ast.Field field) out.addAll(valueReferences(field.value(), lineIndex, file));
        }
        return out;
    }

    private static List<Reference> valueReferences(Ast.ValueNode value, LineIndex lineIndex, String file) {
        var out = new ArrayList<Reference>();
        if (value instanceof Ast.SubstitutionNode sub) {
            out.add(reference(sub.path(), ReferenceKind.Substitution, lineIndex, file));
        } else if (value instanceof Ast.StringNode str) {
            for (var part : str.parts()) if (part instanceof Ast.StringInterpolation interpolation) out.add(reference(interpolation.path(), ReferenceKind.Interpolation, lineIndex, file));
        } else if (value instanceof Ast.ArrayNode array) {
            for (var item : array.items()) {
                if (item instanceof Ast.ArraySpread spread) out.add(reference(spread.sub().path(), ReferenceKind.ArraySpread, lineIndex, file));
                if (item instanceof Ast.ArrayValue arrayValue) out.addAll(valueReferences(arrayValue.value(), lineIndex, file));
            }
        } else if (value instanceof Ast.ObjectValueNode object) {
            out.addAll(references(object.object(), lineIndex, file));
        }
        return out;
    }

    private static List<IncludeReference> includes(Ast.ObjectNode object, LineIndex lineIndex, String file) {
        var out = new ArrayList<IncludeReference>();
        for (var member : object.members()) {
            if (member instanceof Ast.Include include) out.add(new IncludeReference(include.path().value(), file, lineIndex.range(include.span()), null));
            if (member instanceof Ast.Field field && field.value() instanceof Ast.ObjectValueNode nested) out.addAll(includes(nested.object(), lineIndex, file));
        }
        return out;
    }

    private static Reference reference(Ast.PathNode path, ReferenceKind kind, LineIndex lineIndex, String file) {
        return new Reference(names(path), kind, file, lineIndex.range(path.span()), null);
    }

    private static List<Reference> resolveTargets(List<Reference> references, List<Definition> definitions) {
        Map<String, Definition> byPath = definitions.stream().collect(Collectors.toMap(d -> String.join("\0", d.path()), d -> d, (a, b) -> a));
        return references.stream().map(r -> r.withTarget(byPath.get(String.join("\0", r.path())))).toList();
    }

    private static List<Comment> comments(List<TokenInfo> tokens) {
        return tokens.stream().filter(token -> token.kind().equals("comment")).map(token -> new Comment(token.text(), token.span(), token.range())).toList();
    }

    private static List<String> names(Ast.PathNode path) {
        return path.segments().stream().map(Ast.PathSegment::value).toList();
    }

    private static List<String> append(List<String> prefix, List<String> suffix) {
        var out = new ArrayList<String>(prefix);
        out.addAll(suffix);
        return out;
    }

    private static Diagnostic diagnostic(SconException ex, LineIndex lineIndex, String file) {
        return new Diagnostic(ex.code(), ex.getMessage(), DiagnosticSeverity.Error, file, ex.span() == null ? null : lineIndex.range(ex.span()));
    }

    private static final class LineIndex {
        private final List<Integer> lines = new ArrayList<>(List.of(0));

        LineIndex(String source) {
            for (int i = 0; i < source.length(); i++) if (source.charAt(i) == '\n') lines.add(i + 1);
        }

        SourceRange range(Span span) {
            return new SourceRange(position(span.start()), position(span.end()), span);
        }

        private SourcePosition position(int offset) {
            int line = 0;
            while (line + 1 < lines.size() && lines.get(line + 1) <= offset) line++;
            return new SourcePosition(line, offset - lines.get(line));
        }
    }
}
