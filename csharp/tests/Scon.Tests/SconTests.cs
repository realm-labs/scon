using System.Text.Json;
using System.Text.Json.Nodes;
using RealmLabs.Scon;
using RealmLabs.Scon.SystemTextJson;
using Xunit;

namespace Scon.Tests;

public sealed class SconTests
{
    private static readonly string Root = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "../../../../../../"));
    private static readonly string Conformance = Path.Combine(Root, "tests", "conformance");

    public static IEnumerable<object[]> Cases()
    {
        using var doc = JsonDocument.Parse(File.ReadAllText(Path.Combine(Conformance, "manifest.json")));
        foreach (var item in doc.RootElement.GetProperty("cases").EnumerateArray()) yield return [item.GetRawText()];
    }

    [Theory]
    [MemberData(nameof(Cases))]
    public void ConformanceCases(string rawCase)
    {
        using var doc = JsonDocument.Parse(rawCase);
        var item = doc.RootElement;
        var entry = Path.Combine(Conformance, item.GetProperty("entry").GetString()!);
        var expectedPath = Path.Combine(Conformance, item.GetProperty("expected").GetString()!);
        if (item.GetProperty("kind").GetString() == "valid")
        {
            var expected = JsonNode.Parse(File.ReadAllText(expectedPath));
            var value = RealmLabs.Scon.Scon.ParseFile(entry);
            Assert.True(JsonEqual(expected, ToJson(value)));
            Assert.True(JsonEqual(expected, ToJson(RealmLabs.Scon.Scon.ParseString(RealmLabs.Scon.Scon.FormatValue(value)))));
        }
        else
        {
            using var expected = JsonDocument.Parse(File.ReadAllText(expectedPath));
            var ex = Assert.Throws<SconException>(() => RealmLabs.Scon.Scon.ParseFile(entry));
            Assert.Equal(Enum.Parse<ErrorCode>(expected.RootElement.GetProperty("code").GetString()!), ex.Code);
        }
    }

    [Fact]
    public void MapsRecordsAndSystemTextJson()
    {
        var cfg = SconMapper.Deserialize<Config>("Name = \"demo\"\nPort = 8080\nTags = [\"a\", \"b\"]\nMode = \"Fast\"");
        Assert.Equal("demo", cfg.Name);
        Assert.Equal(8080, cfg.Port);
        Assert.Equal(["a", "b"], cfg.Tags);
        Assert.Equal(Mode.Fast, cfg.Mode);
        Assert.Equal(cfg.Name, SconMapper.Deserialize<Config>(SconMapper.Serialize(cfg)).Name);
        Assert.Equal(cfg.Port, SconJsonSerializer.Deserialize<Config>(SconJsonSerializer.Serialize(cfg)).Port);
    }

    [Fact]
    public void RejectsTypedErrors()
    {
        Assert.Throws<SconException>(() => SconMapper.Serialize(new Dictionary<int, string> { [1] = "bad" }));
        Assert.Throws<SconException>(() => SconMapper.Deserialize<Config>("Name = 1"));
    }

    private static JsonNode? ToJson(SconValue value) => JsonNode.Parse(JsonSerializer.Serialize(SconMapper.ToPlain(value)));
    private static bool JsonEqual(JsonNode? a, JsonNode? b)
    {
        if (a is JsonValue av && b is JsonValue bv)
        {
            if (av.TryGetValue<double>(out var ad) && bv.TryGetValue<double>(out var bd)) return Math.Abs(ad - bd) < 0.0000001;
        }
        if (a is JsonArray aa && b is JsonArray ba) return aa.Count == ba.Count && aa.Zip(ba).All(pair => JsonEqual(pair.First, pair.Second));
        if (a is JsonObject ao && b is JsonObject bo) return ao.Count == bo.Count && ao.All(kv => bo.ContainsKey(kv.Key) && JsonEqual(kv.Value, bo[kv.Key]));
        return JsonNode.DeepEquals(a, b);
    }

    public enum Mode { Fast, Slow }
    public sealed record Config(string Name, int Port, List<string> Tags, Mode Mode);
}
