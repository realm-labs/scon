namespace RealmLabs.Scon;

internal static class Formatter
{
    public static string FormatValue(SconValue value) => value is SconObject obj ? FormatObjectBody(obj, 0) + "\n" : throw new SconException(ErrorCode.InvalidRootType, "SCON document root must be an object");
    private static string FormatObjectBody(SconObject obj, int indent) { var outText = new StringBuilder(); foreach (var (key, value) in obj) outText.Append(' ', indent).Append(FormatKey(key)).Append(" = ").Append(FormatScon(value, indent)).Append('\n'); return outText.ToString(); }
    private static string FormatScon(SconValue value, int indent) => value switch
    {
        SconNull => "null",
        SconBool b => b.Value ? "true" : "false",
        SconString s => Quote(s.Value, true),
        SconNumberValue n => n.Number.ToSconString(),
        SconArray a => a.Count == 0 ? "[]" : "[\n" + string.Concat(a.Select(item => new string(' ', indent + 2) + FormatScon(item, indent + 2) + ",\n")) + new string(' ', indent) + "]",
        SconObject o => o.Count == 0 ? "{}" : "{\n" + FormatObjectBody(o, indent + 2) + new string(' ', indent) + "}",
        _ => throw new SconException(ErrorCode.Serde, "unsupported SCON value"),
    };
    private static string FormatKey(string key) => Regex.IsMatch(key, "^[A-Za-z_][A-Za-z0-9_-]*$") ? key : Quote(key, false);
    private static string Quote(string value, bool escapeInterpolation)
    {
        var outText = new StringBuilder("\"");
        foreach (var ch in value) outText.Append(ch switch { '"' => "\\\"", '\\' => "\\\\", '\n' => "\\n", '\r' => "\\r", '\t' => "\\t", '\b' => "\\b", '\f' => "\\f", '$' when escapeInterpolation => "\\$", _ when char.IsControl(ch) => $"\\u{(int)ch:X4}", _ => ch.ToString() });
        return outText.Append('"').ToString();
    }
}
