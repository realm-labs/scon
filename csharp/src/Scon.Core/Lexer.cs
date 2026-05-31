namespace RealmLabs.Scon;

internal readonly record struct Token(string Kind, string Text, Span Span);

internal static class Lexer
{
    public static List<Token> Lex(string source)
    {
        var tokens = new List<Token>();
        var index = 0;
        void Add(string kind, int start, int end) => tokens.Add(new(kind, source[start..end], new(start, end)));
        while (index < source.Length)
        {
            var start = index;
            var ch = source[index];
            if (ch is ' ' or '\t') { while (index < source.Length && source[index] is ' ' or '\t') index++; Add("ws", start, index); }
            else if (ch == '\n') { index++; Add("newline", start, index); }
            else if (ch == '\r') { if (index + 1 >= source.Length || source[index + 1] != '\n') Fail(ErrorCode.InvalidCharacter, "standalone CR is invalid", start, start + 1); index += 2; Add("newline", start, index); }
            else if (ch == '#' || (ch == '/' && index + 1 < source.Length && source[index + 1] == '/')) { index += ch == '#' ? 1 : 2; while (index < source.Length && source[index] is not '\n' and not '\r') index++; Add("comment", start, index); }
            else if (ch == '"') { index = LexString(source, index); Add("string", start, index); }
            else if (ch == '$') { if (index + 1 >= source.Length || source[index + 1] != '{') Fail(ErrorCode.InvalidCharacter, "unexpected character '$'", start, start + 1); index += 2; Add("subst", start, index); }
            else if ("{}[]=,".Contains(ch)) { index++; Add(ch.ToString(), start, index); }
            else if (ch == '.') { if (source.AsSpan(index).StartsWith("...")) { index += 3; Add("...", start, index); } else { index++; Add(".", start, index); } }
            else if (ch == '-') { if (index + 1 >= source.Length || !IsDigit(source[index + 1])) Fail(ErrorCode.UnexpectedToken, "expected digit after '-'", start, start + 1); index = LexNumber(source, index); Add("number", start, index); }
            else if (ch is '?' or ':') Fail(ErrorCode.UnexpectedToken, "unexpected character", start, start + 1);
            else if (IsDigit(ch)) { index = LexNumber(source, index); Add("number", start, index); }
            else if (IsIdentifierStart(ch)) { while (index < source.Length && IsIdentifierPart(source[index])) index++; var text = source[start..index]; Add(text is "true" or "false" or "null" or "include" ? text : "identifier", start, index); }
            else if (char.IsWhiteSpace(ch) || char.GetUnicodeCategory(ch) == System.Globalization.UnicodeCategory.SpaceSeparator) Fail(ErrorCode.InvalidWhitespace, "invalid whitespace outside strings", start, start + 1);
            else Fail(ErrorCode.InvalidCharacter, "unexpected character", start, start + 1);
        }
        tokens.Add(new("eof", "", new(source.Length, source.Length)));
        return tokens;
    }

    private static int LexString(string source, int index)
    {
        var start = index++;
        while (index < source.Length)
        {
            var ch = source[index++];
            if (ch == '"') return index;
            if (ch is '\n' or '\r') Fail(ErrorCode.UnterminatedString, "raw multiline strings are invalid", index - 1, index);
            if (ch == '\\')
            {
                if (index >= source.Length) Fail(ErrorCode.UnterminatedString, "unterminated string escape", index, index);
                var escaped = source[index++];
                if ("\"\\/bfnrt$".Contains(escaped)) continue;
                if (escaped == 'u') { for (var i = 0; i < 4; i++, index++) if (index >= source.Length || !Uri.IsHexDigit(source[index])) Fail(ErrorCode.InvalidEscape, "invalid unicode escape", index, Math.Min(index + 1, source.Length)); continue; }
                Fail(ErrorCode.InvalidEscape, "invalid string escape", index - 2, index - 1);
            }
        }
        Fail(ErrorCode.UnterminatedString, "unterminated string", start, source.Length);
        return index;
    }
    private static int LexNumber(string source, int index)
    {
        var start = index;
        if (source[index] == '-') index++;
        if (index < source.Length && source[index] == '0') { index++; if (index < source.Length && IsDigit(source[index])) Fail(ErrorCode.InvalidNumber, "leading zeroes are invalid", start, index); }
        else { if (index >= source.Length || source[index] is < '1' or > '9') Fail(ErrorCode.InvalidNumber, "invalid number", start, index); while (index < source.Length && IsDigit(source[index])) index++; }
        if (index < source.Length && source[index] == '.') { index++; if (index >= source.Length || !IsDigit(source[index])) Fail(ErrorCode.InvalidNumber, "expected digit after decimal point", start, index); while (index < source.Length && IsDigit(source[index])) index++; }
        if (index < source.Length && source[index] is 'e' or 'E') { index++; if (index < source.Length && source[index] is '+' or '-') index++; if (index >= source.Length || !IsDigit(source[index])) Fail(ErrorCode.InvalidNumber, "expected exponent digit", start, index); while (index < source.Length && IsDigit(source[index])) index++; }
        return index;
    }
    private static bool IsDigit(char ch) => ch is >= '0' and <= '9';
    private static bool IsIdentifierStart(char ch) => ch == '_' || ch is >= 'A' and <= 'Z' || ch is >= 'a' and <= 'z';
    private static bool IsIdentifierPart(char ch) => IsIdentifierStart(ch) || IsDigit(ch) || ch == '-';
    private static void Fail(ErrorCode code, string message, int start, int end) => throw new SconException(code, message, new(start, end));
}
