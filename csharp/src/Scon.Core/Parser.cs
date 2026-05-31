namespace RealmLabs.Scon;

internal sealed class Parser
{
    private readonly List<Token> _tokens;
    private int _index;
    private Parser(List<Token> tokens) => _tokens = tokens;
    public static Document ParseDocument(string source, string? file = null) => new(new Parser(Lexer.Lex(source)).Parse(), file);
    private AstObject Parse()
    {
        SkipTrivia();
        var root = Match("{") ? ParseObject(Previous()) : Check("[") ? throw new SconException(ErrorCode.InvalidRootType, "SCON document root must be an object", Peek().Span) : ParseObjectBody(Peek().Span.Start);
        SkipTrivia();
        Expect("eof", "expected end of file");
        return root;
    }
    private AstObject ParseObject(Token opening) { var members = ParseMembers("}"); var closing = Expect("}", "expected '}'"); return new(members, new(opening.Span.Start, closing.Span.End)); }
    private AstObject ParseObjectBody(int start) { var members = ParseMembers("eof"); return new(members, new(start, members.Count == 0 ? start : members[^1].Span.End)); }
    private List<AstMember> ParseMembers(string end)
    {
        var members = new List<AstMember>(); SkipTrivia();
        while (!Check(end) && !Check("eof")) { members.Add(ParseMember()); SkipTrivia(); if (Match(",")) { SkipTrivia(); if (Check(",")) throw new SconException(ErrorCode.UnexpectedToken, "consecutive commas are invalid", Peek().Span); } }
        return members;
    }
    private AstMember ParseMember()
    {
        SkipTrivia();
        if (Match("include")) { var include = Previous(); SkipInlineTrivia(); var path = ParseString(); if (path.Parts.Any(p => p is StringInterpolation)) throw new SconException(ErrorCode.UnexpectedToken, "include path must be a literal string", path.Span); return new AstInclude(path, new(include.Span.Start, path.Span.End)); }
        if (Match("...")) { var spread = Previous(); var sub = ParseSubstitution(); return new AstObjectSpread(sub, new(spread.Span.Start, sub.Span.End)); }
        var pathNode = ParsePath(); SkipInlineTrivia();
        AstValue value;
        if (Match("=")) { SkipInlineTrivia(); if (Check("newline")) throw new SconException(ErrorCode.UnexpectedToken, "field value cannot start on the next line", Peek().Span); value = ParseValue(); }
        else if (Match("{")) { var obj = ParseObject(Previous()); value = new AstObjectValue(obj, obj.Span); }
        else throw new SconException(ErrorCode.UnexpectedToken, "expected '=' or object shorthand", Peek().Span);
        return new AstField(pathNode, value, new(pathNode.Span.Start, value.Span.End));
    }
    private AstValue ParseValue()
    {
        SkipTrivia();
        if (Match("null")) return new AstNull(Previous().Span);
        if (Match("true")) return new AstBool(true, Previous().Span);
        if (Match("false")) return new AstBool(false, Previous().Span);
        if (Match("number")) return new AstNumber(Previous().Text, Previous().Span);
        if (Check("string")) return ParseString();
        if (Match("{")) { var obj = ParseObject(Previous()); return new AstObjectValue(obj, obj.Span); }
        if (Match("[")) return ParseArray(Previous());
        if (Check("subst")) return ParseSubstitution();
        throw new SconException(ErrorCode.UnexpectedToken, "expected value", Peek().Span);
    }
    private AstArray ParseArray(Token opening)
    {
        var items = new List<AstArrayItem>(); SkipTrivia();
        while (!Check("]") && !Check("eof"))
        {
            var start = Peek().Span.Start;
            if (Match("...")) { var sub = ParseSubstitution(); items.Add(new AstArraySpread(sub, new(start, sub.Span.End))); }
            else { var value = ParseValue(); items.Add(new AstArrayValue(value, value.Span)); }
            SkipTrivia(); if (!Match(",")) break; SkipTrivia(); if (Check(",")) throw new SconException(ErrorCode.UnexpectedToken, "consecutive commas are invalid", Peek().Span);
        }
        var closing = Expect("]", "expected ']'");
        return new(items, new(opening.Span.Start, closing.Span.End));
    }
    private AstSubstitution ParseSubstitution() { var start = Expect("subst", "expected '${'"); var path = ParsePath(); var end = Expect("}", "expected '}'"); return new(path, new(start.Span.Start, end.Span.End)); }
    private AstPath ParsePath()
    {
        var first = ParsePathSegment(); var segments = new List<AstPathSegment> { first };
        while (Match(".")) segments.Add(ParsePathSegment());
        return new(segments, new(first.Span.Start, segments[^1].Span.End));
    }
    private AstPathSegment ParsePathSegment()
    {
        if (Match("identifier")) return new(Previous().Text, false, Previous().Span);
        if (Check("string")) { var s = ParseString(); return new(s.Value, true, s.Span); }
        throw new SconException(ErrorCode.UnexpectedToken, "expected path segment", Peek().Span);
    }
    private AstString ParseString()
    {
        var token = Expect("string", "expected string");
        var (parts, value) = ParseStringParts(token);
        return new(value, token.Text, parts, token.Span);
    }
    private (List<StringPart>, string) ParseStringParts(Token token)
    {
        var raw = token.Text; var parts = new List<StringPart>(); var outText = new StringBuilder(); var value = new StringBuilder();
        for (var i = 1; i < raw.Length - 1;)
        {
            var ch = raw[i++];
            if (ch == '$' && i < raw.Length && raw[i] == '{') { if (outText.Length > 0) { parts.Add(new StringLiteral(outText.ToString())); value.Append(outText); outText.Clear(); } var pathStart = i + 1; var close = raw.IndexOf('}', pathStart); if (close < 0) throw new SconException(ErrorCode.UnterminatedString, "unterminated interpolation", token.Span); parts.Add(new StringInterpolation(ParseInterpolationPath(raw[pathStart..close], token.Span.Start + pathStart), new(token.Span.Start + i - 1, token.Span.Start + close + 1))); i = close + 1; continue; }
            if (ch != '\\') { outText.Append(ch); continue; }
            var escaped = raw[i++];
            outText.Append(escaped switch { '"' => '"', '\\' => '\\', '/' => '/', 'b' => '\b', 'f' => '\f', 'n' => '\n', 'r' => '\r', 't' => '\t', '$' => '$', 'u' => (char)int.Parse(raw.Substring(i, 4), NumberStyles.HexNumber), _ => throw new SconException(ErrorCode.InvalidEscape, "invalid string escape", token.Span) });
            if (escaped == 'u') i += 4;
        }
        if (outText.Length > 0 || parts.Count == 0) { parts.Add(new StringLiteral(outText.ToString())); value.Append(outText); }
        return (parts, value.ToString());
    }
    private AstPath ParseInterpolationPath(string text, int baseOffset)
    {
        var adjusted = Lexer.Lex(text).Select(t => new Token(t.Kind, t.Text, new(t.Span.Start + baseOffset, t.Span.End + baseOffset))).ToList();
        var parser = new Parser(adjusted); var path = parser.ParsePath(); parser.Expect("eof", "expected end of interpolation"); return path;
    }
    private void SkipTrivia() { while (Match("ws") || Match("newline") || Match("comment")) {} }
    private void SkipInlineTrivia() { while (Match("ws") || Match("comment")) {} }
    private bool Match(string kind) { if (!Check(kind)) return false; _index++; return true; }
    private bool Check(string kind) => Peek().Kind == kind;
    private Token Expect(string kind, string message) { if (Check(kind)) { _index++; return Previous(); } throw new SconException(ErrorCode.UnexpectedToken, message, Peek().Span); }
    private Token Peek() => _tokens[Math.Min(_index, _tokens.Count - 1)];
    private Token Previous() => _tokens[_index - 1];
}
