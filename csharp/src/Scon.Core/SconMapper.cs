namespace RealmLabs.Scon;

public static class SconMapper
{
    public static T Deserialize<T>(string source) => (T)Decode(Scon.ParseString(source), typeof(T))!;
    public static string Serialize<T>(T value) => Scon.FormatValue(Encode(value));
    public static object? ToPlain(SconValue value) => value switch { SconNull => null, SconBool b => b.Value, SconString s => s.Value, SconNumberValue n => n.Number.PlainValue(), SconArray a => a.Select(ToPlain).ToList(), SconObject o => o.ToDictionary(kv => kv.Key, kv => ToPlain(kv.Value)), _ => throw new SconException(ErrorCode.Serde, "unsupported SCON value") };
    public static SconValue Encode(object? value)
    {
        if (value is null) return SconNull.Instance;
        if (value is SconValue scon) return scon;
        if (value is JsonElement json) return EncodeJsonElement(json);
        if (value is bool b) return new SconBool(b);
        if (value is string s) return new SconString(s);
        if (value is byte or sbyte or short or ushort or int or uint or long) { var n = Convert.ToInt64(value, CultureInfo.InvariantCulture); return new SconNumberValue(n < 0 ? SconNumber.FromI64(n) : SconNumber.FromU64((ulong)n)); }
        if (value is ulong ul) return new SconNumberValue(SconNumber.FromU64(ul));
        if (value is float or double or decimal) { var d = Convert.ToDouble(value, CultureInfo.InvariantCulture); if (!double.IsFinite(d)) throw new SconException(ErrorCode.Serde, "non-finite floats cannot be serialized"); return new SconNumberValue(SconNumber.FromF64(d)); }
        if (value is Enum e) return new SconString(e.ToString());
        if (value is IDictionary dict) { var obj = new SconObject(); foreach (DictionaryEntry entry in dict) { if (entry.Key is not string key) throw new SconException(ErrorCode.Serde, "SCON map keys must be strings"); obj.Set(key, Encode(entry.Value)); } return obj; }
        if (value is IEnumerable enumerable && value is not string) return new SconArray(enumerable.Cast<object?>().Select(Encode));
        var outObj = new SconObject();
        foreach (var prop in value.GetType().GetProperties(BindingFlags.Instance | BindingFlags.Public).Where(p => p.GetIndexParameters().Length == 0 && p.CanRead)) outObj.Set(prop.Name, Encode(prop.GetValue(value)));
        foreach (var field in value.GetType().GetFields(BindingFlags.Instance | BindingFlags.Public)) outObj.Set(field.Name, Encode(field.GetValue(value)));
        return outObj;
    }
    public static object? Decode(SconValue value, Type type)
    {
        var underlying = Nullable.GetUnderlyingType(type);
        if (underlying is not null) return value is SconNull ? null : Decode(value, underlying);
        if (type == typeof(object)) return ToPlain(value);
        if (type == typeof(string)) return value is SconString s ? s.Value : throw new SconException(ErrorCode.Serde, "expected string");
        if (type == typeof(bool)) return value is SconBool b ? b.Value : throw new SconException(ErrorCode.Serde, "expected bool");
        if (type.IsEnum) return value is SconString s ? Enum.Parse(type, s.Value) : throw new SconException(ErrorCode.Serde, "expected enum string");
        if (IsNumberType(type)) return DecodeNumber(value, type);
        if (type.IsArray) { if (value is not SconArray a) throw new SconException(ErrorCode.Serde, "expected array"); var itemType = type.GetElementType()!; var outArray = Array.CreateInstance(itemType, a.Count); for (var i = 0; i < a.Count; i++) outArray.SetValue(Decode(a[i], itemType), i); return outArray; }
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(List<>)) { if (value is not SconArray a) throw new SconException(ErrorCode.Serde, "expected array"); var itemType = type.GetGenericArguments()[0]; var list = (IList)Activator.CreateInstance(type)!; foreach (var item in a) list.Add(Decode(item, itemType)); return list; }
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(Dictionary<,>)) { if (value is not SconObject o) throw new SconException(ErrorCode.Serde, "expected object"); var args = type.GetGenericArguments(); if (args[0] != typeof(string)) throw new SconException(ErrorCode.Serde, "SCON map keys must be strings"); var dict = (IDictionary)Activator.CreateInstance(type)!; foreach (var (key, item) in o) dict.Add(key, Decode(item, args[1])); return dict; }
        if (value is not SconObject obj) throw new SconException(ErrorCode.Serde, "expected object");
        var ctor = type.GetConstructors().OrderByDescending(c => c.GetParameters().Length).FirstOrDefault();
        object instance;
        if (ctor is not null && ctor.GetParameters().Length > 0)
        {
            var args = ctor.GetParameters().Select(p => obj.Get(p.Name!) is { } item ? Decode(item, p.ParameterType) : throw new SconException(ErrorCode.Serde, $"missing field {p.Name}")).ToArray();
            instance = ctor.Invoke(args);
        }
        else instance = Activator.CreateInstance(type) ?? throw new SconException(ErrorCode.Serde, $"unsupported target type {type}");
        foreach (var prop in type.GetProperties(BindingFlags.Instance | BindingFlags.Public).Where(p => p.CanWrite)) if (obj.Get(prop.Name) is { } item) prop.SetValue(instance, Decode(item, prop.PropertyType));
        foreach (var field in type.GetFields(BindingFlags.Instance | BindingFlags.Public)) if (obj.Get(field.Name) is { } item) field.SetValue(instance, Decode(item, field.FieldType));
        return instance;
    }
    private static object DecodeNumber(SconValue value, Type type)
    {
        if (value is not SconNumberValue number) throw new SconException(ErrorCode.Serde, "expected number");
        checked { if (type == typeof(byte)) return (byte)number.Number.AsU64(); if (type == typeof(sbyte)) return (sbyte)number.Number.AsI64(); if (type == typeof(short)) return (short)number.Number.AsI64(); if (type == typeof(ushort)) return (ushort)number.Number.AsU64(); if (type == typeof(int)) return (int)number.Number.AsI64(); if (type == typeof(uint)) return (uint)number.Number.AsU64(); if (type == typeof(long)) return number.Number.AsI64(); if (type == typeof(ulong)) return number.Number.AsU64(); }
        if (type == typeof(float)) return (float)number.Number.AsF64();
        if (type == typeof(double)) return number.Number.AsF64();
        throw new SconException(ErrorCode.Serde, "unsupported numeric type");
    }
    private static bool IsNumberType(Type type) => type == typeof(byte) || type == typeof(sbyte) || type == typeof(short) || type == typeof(ushort) || type == typeof(int) || type == typeof(uint) || type == typeof(long) || type == typeof(ulong) || type == typeof(float) || type == typeof(double);
    private static SconValue EncodeJsonElement(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Null => SconNull.Instance,
        JsonValueKind.True => new SconBool(true),
        JsonValueKind.False => new SconBool(false),
        JsonValueKind.String => new SconString(value.GetString()!),
        JsonValueKind.Number when value.TryGetInt64(out var i64) => new SconNumberValue(i64 < 0 ? SconNumber.FromI64(i64) : SconNumber.FromU64((ulong)i64)),
        JsonValueKind.Number when value.TryGetUInt64(out var u64) => new SconNumberValue(SconNumber.FromU64(u64)),
        JsonValueKind.Number => new SconNumberValue(SconNumber.FromF64(value.GetDouble())),
        JsonValueKind.Array => new SconArray(value.EnumerateArray().Select(EncodeJsonElement)),
        JsonValueKind.Object => EncodeJsonObject(value),
        _ => throw new SconException(ErrorCode.Serde, "unsupported JSON value"),
    };
    private static SconObject EncodeJsonObject(JsonElement value)
    {
        var obj = new SconObject();
        foreach (var prop in value.EnumerateObject()) obj.Set(prop.Name, EncodeJsonElement(prop.Value));
        return obj;
    }
}
