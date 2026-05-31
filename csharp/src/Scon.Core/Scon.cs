namespace RealmLabs.Scon;

public static class Scon
{
    public static SconValue ParseString(string source) => new Resolver(new LoadOptions()).Eval(Parser.ParseDocument(source));
    public static SconValue ParseFile(string path, LoadOptions? options = null)
    {
        options ??= new LoadOptions();
        var file = Path.GetFullPath(path);
        var root = Path.GetFullPath(options.IncludeRoot ?? Path.GetDirectoryName(file)!);
        var source = File.ReadAllText(file);
        if (Encoding.UTF8.GetByteCount(source) > options.MaxFileSize) throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum file size exceeded");
        var resolver = new Resolver(options with { IncludeRoot = root });
        resolver.Stack.Add(file); resolver.Seen.Add(file);
        return resolver.Eval(Parser.ParseDocument(source, file));
    }
    public static string FormatValue(SconValue value) => Formatter.FormatValue(value);
    public static SconValue GetPath(SconValue value, string path)
    {
        var current = value;
        foreach (var segment in path.Split('.')) { if (current is not SconObject obj) throw new SconException(ErrorCode.TypeMismatch, "path segment requires object"); current = obj.Get(segment) ?? throw new SconException(ErrorCode.MissingReference, "path is not defined"); }
        return current;
    }
}
