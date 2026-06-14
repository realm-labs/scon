using System.Text;
using RealmLabs.Scon;
using SharpFuzz;

var target = args.Length > 0 ? args[0] : "parse";
var utf8 = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

Fuzzer.Run(stream =>
{
    using var memory = new MemoryStream();
    stream.CopyTo(memory);

    string source;
    try
    {
        source = utf8.GetString(memory.ToArray());
    }
    catch (DecoderFallbackException)
    {
        return;
    }

    switch (target)
    {
        case "parse":
            Parse(source);
            break;
        case "format-source":
            FormatSource(source);
            break;
        default:
            throw new ArgumentException($"unknown fuzz target: {target}");
    }
});

static void Parse(string source)
{
    try
    {
        Scon.ParseString(source);
    }
    catch (SconException)
    {
        // Expected syntax and semantic errors are not fuzz failures.
    }
}

static void FormatSource(string source)
{
    string formatted;
    try
    {
        formatted = Scon.FormatSource(source);
    }
    catch (SconException)
    {
        return;
    }

    if (Scon.AnalyzeSource(formatted).Parsed is null)
    {
        throw new InvalidOperationException("formatted source does not parse");
    }

    try
    {
        var original = Scon.ParseString(source);
        var roundTrip = Scon.ParseString(formatted);
        if (Scon.FormatValue(original) != Scon.FormatValue(roundTrip))
        {
            throw new InvalidOperationException("formatted source changed resolved value");
        }
    }
    catch (SconException)
    {
        // If either source fails semantic resolution, formatting parseability is
        // still the invariant under test.
    }
}
