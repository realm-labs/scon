namespace RealmLabs.Scon;

public static class SourceFormatter
{
    public static string FormatSource(string source)
    {
        var document = Parser.ParseDocument(source);
        var output = new StringBuilder();
        foreach (var token in Lexer.Lex(source).Where(token => token.Kind == "comment"))
        {
            output.Append(token.Text).Append('\n');
        }
        WriteObjectBody(output, document.Root, 0);
        output.Append('\n');
        return output.ToString();
    }

    private static void WriteObjectBody(StringBuilder output, AstObject obj, int indent)
    {
        foreach (var member in obj.Members)
        {
            output.Append(' ', indent);
            switch (member)
            {
                case AstInclude include:
                    output.Append("include ").Append(include.Path.Raw);
                    break;
                case AstObjectSpread spread:
                    output.Append("...");
                    WriteSubstitution(output, spread.Sub);
                    break;
                case AstField field:
                    WritePath(output, field.Path);
                    output.Append(" = ");
                    WriteValue(output, field.Value, indent);
                    break;
            }
            output.Append('\n');
        }
    }

    private static void WriteValue(StringBuilder output, AstValue value, int indent)
    {
        switch (value)
        {
            case AstNull:
                output.Append("null");
                break;
            case AstBool boolean:
                output.Append(boolean.Value ? "true" : "false");
                break;
            case AstNumber number:
                output.Append(number.Raw);
                break;
            case AstString str:
                output.Append(str.Raw);
                break;
            case AstSubstitution substitution:
                WriteSubstitution(output, substitution);
                break;
            case AstArray array:
                WriteArray(output, array, indent);
                break;
            case AstObjectValue obj:
                if (obj.Object.Members.Count == 0)
                {
                    output.Append("{}");
                    break;
                }
                output.Append("{\n");
                WriteObjectBody(output, obj.Object, indent + 2);
                output.Append(' ', indent).Append('}');
                break;
        }
    }

    private static void WriteArray(StringBuilder output, AstArray array, int indent)
    {
        if (array.Items.Count == 0)
        {
            output.Append("[]");
            return;
        }
        output.Append("[\n");
        foreach (var item in array.Items)
        {
            output.Append(' ', indent + 2);
            switch (item)
            {
                case AstArrayValue value:
                    WriteValue(output, value.Value, indent + 2);
                    break;
                case AstArraySpread spread:
                    output.Append("...");
                    WriteSubstitution(output, spread.Sub);
                    break;
            }
            output.Append(",\n");
        }
        output.Append(' ', indent).Append(']');
    }

    private static void WriteSubstitution(StringBuilder output, AstSubstitution substitution)
    {
        output.Append("${");
        WritePath(output, substitution.Path);
        output.Append('}');
    }

    private static void WritePath(StringBuilder output, AstPath path)
    {
        for (var index = 0; index < path.Segments.Count; index++)
        {
            if (index > 0) output.Append('.');
            var segment = path.Segments[index];
            if (segment.Quoted || !IsUnquotedSegment(segment.Value)) WriteQuoted(output, segment.Value);
            else output.Append(segment.Value);
        }
    }

    private static bool IsUnquotedSegment(string value) => !IsReservedSegment(value) && Regex.IsMatch(value, "^[A-Za-z_][A-Za-z0-9_-]*$");

    private static bool IsReservedSegment(string value) => value is "include" or "true" or "false" or "null";

    private static void WriteQuoted(StringBuilder output, string value)
    {
        output.Append('"');
        foreach (var ch in value)
        {
            output.Append(ch switch
            {
                '"' => "\\\"",
                '\\' => "\\\\",
                '\n' => "\\n",
                '\r' => "\\r",
                '\t' => "\\t",
                '\b' => "\\b",
                '\f' => "\\f",
                _ when char.IsControl(ch) => $"\\u{(int)ch:X4}",
                _ => ch.ToString(),
            });
        }
        output.Append('"');
    }
}
