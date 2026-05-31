namespace RealmLabs.Scon;

internal sealed class Resolver
{
    private sealed class EvalObj { public readonly LinkedHashMap Entries = new(); }
    private sealed class EvalEntry(object value, string layer, string kind) { public object Value = value; public string Layer = layer; public string Kind = kind; }
    private sealed class LinkedHashMap : IEnumerable<KeyValuePair<string, EvalEntry>>
    {
        private readonly List<KeyValuePair<string, EvalEntry>> _entries = [];
        private readonly Dictionary<string, int> _index = new(StringComparer.Ordinal);
        public EvalEntry? Get(string key) => _index.TryGetValue(key, out var index) ? _entries[index].Value : null;
        public void Set(string key, EvalEntry value) { if (_index.TryGetValue(key, out var index)) _entries[index] = new(key, value); else { _index[key] = _entries.Count; _entries.Add(new(key, value)); } }
        public IEnumerator<KeyValuePair<string, EvalEntry>> GetEnumerator() => _entries.GetEnumerator(); IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    }
    private readonly LoadOptions _options;
    private readonly EvalObj _root = new();
    private readonly List<List<string>> _inProgress = [[]];
    internal readonly List<string> Stack = [];
    internal readonly HashSet<string> Seen = new(StringComparer.Ordinal);
    private readonly Dictionary<string, Document> _cache = new(StringComparer.Ordinal);
    public Resolver(LoadOptions options) => _options = options;
    public SconValue Eval(Document doc) { EvaluateObject(doc.Root, [], doc.File); return PublicObject(_root); }
    private void EvaluateObject(AstObject obj, List<string> path, string? file)
    {
        if (path.Count > _options.MaxObjectDepth) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum object depth exceeded", obj.Span);
        var localSeen = false;
        foreach (var member in obj.Members)
        {
            switch (member)
            {
                case AstObjectSpread spread:
                    if (localSeen) throw new SconException(ErrorCode.InvalidSpread, "object spread must appear before local members", spread.Span);
                    if (Lookup(spread.Sub.Path, spread.Span).Value is not EvalObj source) throw new SconException(ErrorCode.TypeMismatch, "object spread target is not an object", spread.Span);
                    OverlayBase(ObjectAt(path, spread.Span), source);
                    break;
                case AstInclude include:
                    var included = LoadInclude(file, include); EvaluateObject(included.Root, path, included.File); break;
                case AstField field:
                    localSeen = true; EvalField(field, path, file); break;
            }
        }
    }
    private void EvalField(AstField field, List<string> current, string? file)
    {
        var target = current.Concat(field.Path.Segments.Select(s => s.Value)).ToList();
        if (field.Value is AstObjectValue obj) { EnsureObject(target, field.Span); _inProgress.Add(target); try { EvaluateObject(obj.Object, target, file); } finally { _inProgress.RemoveAt(_inProgress.Count - 1); } return; }
        Insert(target, EvalValue(field.Value, file), "ordinary", field.Span);
    }
    private object EvalValue(AstValue value, string? file) => value switch
    {
        AstNull => SconNull.Instance,
        AstBool b => new SconBool(b.Value),
        AstNumber n => ParseNumber(n),
        AstString s => new SconString(EvalString(s)),
        AstSubstitution s => CloneAny(Lookup(s.Path, s.Span).Value),
        AstArray a => EvalArray(a, file),
        AstObjectValue o => EvalNested(o, file),
        _ => throw new SconException(ErrorCode.UnexpectedToken, "unknown value"),
    };
    private static SconValue ParseNumber(AstNumber n) { try { return new SconNumberValue(SconNumber.Parse(n.Raw)); } catch (SconException ex) { throw new SconException(ex.Code, ex.Message, n.Span); } }
    private EvalObj EvalNested(AstObjectValue value, string? file) { var nested = new Resolver(_options); nested.Stack.AddRange(Stack); foreach (var s in Seen) nested.Seen.Add(s); foreach (var kv in _cache) nested._cache[kv.Key] = kv.Value; nested.EvaluateObject(value.Object, [], file); return nested._root; }
    private SconArray EvalArray(AstArray array, string? file)
    {
        var outArray = new SconArray();
        foreach (var item in array.Items)
        {
            if (outArray.Count >= _options.MaxArrayLength) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum array length exceeded", item.Span);
            if (item is AstArrayValue value) outArray.Add(PublicMaybe(EvalValue(value.Value, file)));
            else if (item is AstArraySpread spread) { if (Lookup(spread.Sub.Path, spread.Span).Value is not SconArray values) throw new SconException(ErrorCode.TypeMismatch, "array spread target is not an array", spread.Span); foreach (var v in values) outArray.Add(CloneValue(v)); }
        }
        return outArray;
    }
    private string EvalString(AstString value)
    {
        if (value.Parts.Count == 1 && value.Parts[0] is StringLiteral lit) return lit.Value;
        var outText = new StringBuilder();
        foreach (var part in value.Parts)
        {
            if (part is StringLiteral literal) outText.Append(literal.Value);
            else if (part is StringInterpolation interpolation)
            {
                var replacement = Lookup(interpolation.Path, interpolation.Span).Value;
                if (replacement is SconString s) outText.Append(s.Value);
                else if (replacement is SconBool b) outText.Append(b.Value ? "true" : "false");
                else if (replacement is SconNumberValue n) outText.Append(n.Number.ToSconString());
                else throw new SconException(ErrorCode.TypeMismatch, "interpolation requires string, number, or boolean", interpolation.Span);
            }
        }
        return outText.ToString();
    }
    private EvalEntry Lookup(AstPath path, Span span)
    {
        var names = path.Segments.Select(s => s.Value).ToList();
        if (_inProgress.Any(active => active.SequenceEqual(names))) throw new SconException(ErrorCode.MissingReference, "reference is not completed yet", span);
        var obj = _root; EvalEntry? entry = null;
        for (var i = 0; i < names.Count; i++) { entry = obj.Entries.Get(names[i]); if (entry is null) throw new SconException(ErrorCode.MissingReference, $"missing reference '{names[i]}'", span); if (i < names.Count - 1) { if (entry.Value is not EvalObj next) throw new SconException(ErrorCode.TypeMismatch, "reference path crosses non-object value", span); obj = next; } }
        return entry!;
    }
    private void EnsureObject(List<string> path, Span span)
    {
        var obj = _root;
        for (var i = 0; i < path.Count; i++)
        {
            var entry = obj.Entries.Get(path[i]);
            if (entry is null) { var child = new EvalObj(); obj.Entries.Set(path[i], new(child, "local", "structural")); obj = child; continue; }
            if (entry.Value is not EvalObj next) throw new SconException(ErrorCode.PathConflict, "path conflicts with scalar value", span);
            if (i == path.Count - 1 && entry.Layer == "local" && entry.Kind != "structural") throw new SconException(ErrorCode.PathConflict, "object field conflicts with ordinary value", span);
            entry.Layer = "local"; entry.Kind = "structural"; obj = next;
        }
    }
    private void Insert(List<string> path, object value, string kind, Span span)
    {
        var obj = _root;
        foreach (var name in path.Take(path.Count - 1))
        {
            var entry = obj.Entries.Get(name);
            if (entry is null) { var child = new EvalObj(); obj.Entries.Set(name, new(child, "local", "structural")); obj = child; }
            else { if (entry.Value is not EvalObj next) throw new SconException(ErrorCode.PathConflict, "path conflicts with scalar value", span); obj = next; }
        }
        var leaf = path[^1]; var existing = obj.Entries.Get(leaf);
        if (existing is null) obj.Entries.Set(leaf, new(value, "local", kind));
        else if (existing.Layer == "base") OverlayLocal(existing, value, kind);
        else throw new SconException(ErrorCode.DuplicateKey, $"duplicate key '{leaf}'", span);
    }
    private EvalObj ObjectAt(List<string> path, Span span)
    {
        var obj = _root;
        foreach (var name in path) { var entry = obj.Entries.Get(name); if (entry is null) throw new SconException(ErrorCode.PathConflict, "target object does not exist", span); if (entry.Value is not EvalObj next) throw new SconException(ErrorCode.PathConflict, "target path is not an object", span); obj = next; }
        return obj;
    }
    private Document LoadInclude(string? file, AstInclude include)
    {
        var path = include.Path.Value;
        if (InvalidIncludePath(path)) throw new SconException(ErrorCode.InvalidIncludePath, "invalid include path", include.Span);
        var rootPath = Path.GetFullPath(_options.IncludeRoot ?? (file is null ? "." : Path.GetDirectoryName(file)!));
        var basePath = file is null ? rootPath : Path.GetDirectoryName(file)!;
        var candidate = Path.GetFullPath(Path.Combine(basePath, path));
        if (!candidate.StartsWith(rootPath, StringComparison.Ordinal)) throw new SconException(ErrorCode.IncludePathDenied, "include path escapes include root", include.Span);
        if (Stack.Contains(candidate)) throw new SconException(ErrorCode.IncludeCycle, $"include cycle: {candidate}", include.Span);
        if (Stack.Count >= _options.MaxIncludeDepth) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum include depth exceeded", include.Span);
        Seen.Add(candidate); if (Seen.Count > _options.MaxIncludeFiles) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum include file count exceeded", include.Span);
        if (_cache.TryGetValue(candidate, out var cached)) return cached;
        if (!File.Exists(candidate)) throw new SconException(ErrorCode.IncludeNotFound, $"include file not found: {candidate}", include.Span);
        if ((File.GetAttributes(candidate) & FileAttributes.Directory) != 0) throw new SconException(ErrorCode.IncludeNotFile, "include path is not a file", include.Span);
        if (new FileInfo(candidate).Length > _options.MaxFileSize) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum file size exceeded", include.Span);
        Stack.Add(candidate);
        try { var doc = Parser.ParseDocument(File.ReadAllText(candidate), candidate); _cache[candidate] = doc; return doc; }
        catch (SconException ex) { throw new SconException(ex.Code == ErrorCode.InvalidRootType ? ErrorCode.IncludeRootTypeError : ErrorCode.IncludeParseError, ex.Message, ex.Span); }
        finally { Stack.RemoveAt(Stack.Count - 1); }
    }
    private static bool InvalidIncludePath(string path) => path.Contains("://", StringComparison.Ordinal) || path.StartsWith("classpath:", StringComparison.Ordinal) || path.Contains('*') || path.StartsWith('~') || path.StartsWith('$') || Path.IsPathRooted(path) || Regex.IsMatch(path, "^[A-Za-z]:[\\\\/]");
    private static SconObject PublicObject(EvalObj obj) { var outObj = new SconObject(); foreach (var (key, entry) in obj.Entries) outObj.Set(key, PublicMaybe(entry.Value)); return outObj; }
    private static SconValue PublicMaybe(object value) => value is EvalObj obj ? PublicObject(obj) : (SconValue)value;
    private static object CloneAny(object value) { if (value is not EvalObj obj) return CloneValue((SconValue)value); var outObj = new EvalObj(); foreach (var (key, entry) in obj.Entries) outObj.Entries.Set(key, new(CloneAny(entry.Value), entry.Layer, entry.Kind)); return outObj; }
    private static SconValue CloneValue(SconValue value) => value switch { SconArray a => new SconArray(a.Select(CloneValue)), SconObject o => CloneObject(o), _ => value };
    private static SconObject CloneObject(SconObject obj) { var outObj = new SconObject(); foreach (var (key, value) in obj) outObj.Set(key, CloneValue(value)); return outObj; }
    private static void OverlayBase(EvalObj target, EvalObj source) { foreach (var (key, entry) in source.Entries) { var existing = target.Entries.Get(key); if (existing is null) target.Entries.Set(key, new(CloneAny(entry.Value), "base", "ordinary")); else if (existing.Layer == "base") OverlayLocal(existing, CloneAny(entry.Value), entry.Kind); } }
    private static void OverlayLocal(EvalEntry existing, object value, string kind) { if (existing.Value is EvalObj a && value is EvalObj b) { MergeOverride(a, b); existing.Layer = "local"; existing.Kind = kind; } else { existing.Value = value; existing.Layer = "local"; existing.Kind = kind; } }
    private static void MergeOverride(EvalObj target, EvalObj source) { foreach (var (key, entry) in source.Entries) { var existing = target.Entries.Get(key); if (existing?.Value is EvalObj a && entry.Value is EvalObj b) MergeOverride(a, b); else target.Entries.Set(key, new(CloneAny(entry.Value), entry.Layer, entry.Kind)); } }
}
