using System.Text.Json;
using RealmLabs.Scon;

namespace RealmLabs.Scon.SystemTextJson;

public static class SconJsonSerializer
{
    private static readonly JsonSerializerOptions Options = new(JsonSerializerDefaults.Web);

    public static T Deserialize<T>(string source)
    {
        var plain = SconMapper.ToPlain(Scon.ParseString(source));
        var json = JsonSerializer.Serialize(plain, Options);
        return JsonSerializer.Deserialize<T>(json, Options)!;
    }

    public static string Serialize<T>(T value)
    {
        var plain = JsonSerializer.Deserialize<object>(JsonSerializer.Serialize(value, Options), Options);
        return Scon.FormatValue(SconMapper.Encode(plain));
    }
}
