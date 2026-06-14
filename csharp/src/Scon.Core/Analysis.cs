namespace RealmLabs.Scon;

public sealed record SourcePosition(int Line, int Column);
public sealed record SourceRange(SourcePosition Start, SourcePosition End, Span Span);
public sealed record SconComment(string Text, Span Span, SourceRange Range);
public enum DiagnosticSeverity { Error, Warning, Information, Hint }
public sealed record SconDiagnostic(ErrorCode Code, string Message, DiagnosticSeverity Severity, string? File, SourceRange? Range);
public sealed record SconTokenInfo(string Kind, string Text, Span Span, SourceRange Range);
public sealed record SconSymbol(IReadOnlyList<string> Path, string? File, SourceRange Range);
public sealed record SconDefinition(IReadOnlyList<string> Path, string? File, SourceRange Range);
public enum SconReferenceKind { Substitution, Interpolation, ObjectSpread, ArraySpread }
public sealed record SconReference(IReadOnlyList<string> Path, SconReferenceKind Kind, string? File, SourceRange Range)
{
    public SconDefinition? Target { get; internal set; }
}
public sealed record SconIncludeReference(string Path, string? File, SourceRange Range, string? ResolvedPath = null);
public sealed record SconParsedSource(string? File, IReadOnlyList<SconTokenInfo> Tokens, IReadOnlyList<SconComment> Comments, IReadOnlyList<SconSymbol> Symbols);
public sealed record SconAnalysis(
    string? File,
    SconParsedSource? Parsed,
    IReadOnlyList<SconDiagnostic> Diagnostics,
    IReadOnlyList<SconComment> Comments,
    IReadOnlyList<SconSymbol> Symbols,
    IReadOnlyList<SconDefinition> Definitions,
    IReadOnlyList<SconReference> References,
    IReadOnlyList<SconIncludeReference> Includes,
    SconValue? Value);

internal sealed class LineIndex
{
    private readonly List<int> _lines = [0];

    public LineIndex(string source)
    {
        for (var i = 0; i < source.Length; i++)
            if (source[i] == '\n') _lines.Add(i + 1);
    }

    public SourceRange Range(Span span) => new(Position(span.Start), Position(span.End), span);

    private SourcePosition Position(int offset)
    {
        var line = 0;
        while (line + 1 < _lines.Count && _lines[line + 1] <= offset) line++;
        return new SourcePosition(line, offset - _lines[line]);
    }
}

internal static class Analyzer
{
    public static SconParsedSource ParseSource(string source, string? file = null)
    {
        var document = Parser.ParseDocument(source, file);
        var lineIndex = new LineIndex(source);
        var tokens = Lexer.Lex(source).Select(token => new SconTokenInfo(token.Kind, token.Text, token.Span, lineIndex.Range(token.Span))).ToList();
        var comments = tokens.Where(token => token.Kind == "comment").Select(token => new SconComment(token.Text, token.Span, token.Range)).ToList();
        return new SconParsedSource(file, tokens, comments, Symbols(document.Root, lineIndex, file, []));
    }

    public static SconAnalysis AnalyzeSource(string source, string? file = null)
    {
        var lineIndex = new LineIndex(source);
        List<SconTokenInfo> tokens = [];
        try
        {
            tokens = Lexer.Lex(source).Select(token => new SconTokenInfo(token.Kind, token.Text, token.Span, lineIndex.Range(token.Span))).ToList();
            var document = Parser.ParseDocument(source, file);
            var comments = tokens.Where(token => token.Kind == "comment").Select(token => new SconComment(token.Text, token.Span, token.Range)).ToList();
            var parsed = new SconParsedSource(file, tokens, comments, Symbols(document.Root, lineIndex, file, []));
            var definitions = Definitions(document.Root, lineIndex, file, []);
            var references = References(document.Root, lineIndex, file);
            ResolveTargets(references, definitions);
            var diagnostics = new List<SconDiagnostic>();
            SconValue? value = null;
            try { value = Scon.ParseString(source); }
            catch (SconException ex) { diagnostics.Add(Diagnostic(ex, lineIndex, file)); }
            return new SconAnalysis(file, parsed, diagnostics, comments, parsed.Symbols, definitions, references, Includes(document.Root, lineIndex, file), value);
        }
        catch (SconException ex)
        {
            var comments = tokens.Where(token => token.Kind == "comment").Select(token => new SconComment(token.Text, token.Span, token.Range)).ToList();
            return new SconAnalysis(file, null, [Diagnostic(ex, lineIndex, file)], comments, [], [], [], [], null);
        }
    }

    private static List<SconSymbol> Symbols(AstObject obj, LineIndex lineIndex, string? file, List<string> prefix)
    {
        var symbols = new List<SconSymbol>();
        foreach (var member in obj.Members.OfType<AstField>())
        {
            var path = prefix.Concat(Names(member.Path)).ToList();
            symbols.Add(new SconSymbol(path, file, lineIndex.Range(member.Path.Span)));
            if (member.Value is AstObjectValue nested) symbols.AddRange(Symbols(nested.Object, lineIndex, file, path));
        }
        return symbols;
    }

    private static List<SconDefinition> Definitions(AstObject obj, LineIndex lineIndex, string? file, List<string> prefix)
    {
        var definitions = new List<SconDefinition>();
        foreach (var member in obj.Members.OfType<AstField>())
        {
            var path = prefix.Concat(Names(member.Path)).ToList();
            definitions.Add(new SconDefinition(path, file, lineIndex.Range(member.Path.Span)));
            if (member.Value is AstObjectValue nested) definitions.AddRange(Definitions(nested.Object, lineIndex, file, path));
        }
        return definitions;
    }

    private static List<SconReference> References(AstObject obj, LineIndex lineIndex, string? file)
    {
        var references = new List<SconReference>();
        foreach (var member in obj.Members)
        {
            if (member is AstObjectSpread spread) references.Add(Reference(spread.Sub.Path, SconReferenceKind.ObjectSpread, lineIndex, file));
            if (member is AstField field) references.AddRange(ValueReferences(field.Value, lineIndex, file));
        }
        return references;
    }

    private static List<SconReference> ValueReferences(AstValue value, LineIndex lineIndex, string? file) => value switch
    {
        AstSubstitution sub => [Reference(sub.Path, SconReferenceKind.Substitution, lineIndex, file)],
        AstString str => str.Parts.OfType<StringInterpolation>().Select(part => Reference(part.Path, SconReferenceKind.Interpolation, lineIndex, file)).ToList(),
        AstArray array => array.Items.SelectMany(item => item switch
        {
            AstArraySpread spread => [Reference(spread.Sub.Path, SconReferenceKind.ArraySpread, lineIndex, file)],
            AstArrayValue value => ValueReferences(value.Value, lineIndex, file),
            _ => [],
        }).ToList(),
        AstObjectValue obj => References(obj.Object, lineIndex, file),
        _ => [],
    };

    private static List<SconIncludeReference> Includes(AstObject obj, LineIndex lineIndex, string? file)
    {
        var includes = new List<SconIncludeReference>();
        foreach (var member in obj.Members)
        {
            if (member is AstInclude include) includes.Add(new SconIncludeReference(include.Path.Value, file, lineIndex.Range(include.Span)));
            if (member is AstField { Value: AstObjectValue nested }) includes.AddRange(Includes(nested.Object, lineIndex, file));
        }
        return includes;
    }

    private static SconReference Reference(AstPath path, SconReferenceKind kind, LineIndex lineIndex, string? file) =>
        new(Names(path), kind, file, lineIndex.Range(path.Span));

    private static void ResolveTargets(List<SconReference> references, List<SconDefinition> definitions)
    {
        var byPath = definitions.ToDictionary(definition => string.Join('\0', definition.Path), definition => definition);
        foreach (var reference in references)
            if (byPath.TryGetValue(string.Join('\0', reference.Path), out var target)) reference.Target = target;
    }

    private static List<string> Names(AstPath path) => path.Segments.Select(segment => segment.Value).ToList();

    private static SconDiagnostic Diagnostic(SconException ex, LineIndex lineIndex, string? file) =>
        new(ex.Code, ex.Message, DiagnosticSeverity.Error, file, ex.Span is { } span ? lineIndex.Range(span) : null);
}
