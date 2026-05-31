namespace RealmLabs.Scon;

public abstract record SconValue;
public sealed record SconNull : SconValue { public static readonly SconNull Instance = new(); private SconNull() {} }
public sealed record SconBool(bool Value) : SconValue;
public sealed record SconString(string Value) : SconValue;
public sealed record SconNumber
{
    public enum NumberKind { I64, U64, F64 }
    private SconNumber(NumberKind kind, long i64, ulong u64, double f64) => (Kind, I64Value, U64Value, F64Value) = (kind, i64, u64, f64);
    public NumberKind Kind { get; }
    public long I64Value { get; }
    public ulong U64Value { get; }
    public double F64Value { get; }
    public static SconNumber FromI64(long value) => new(NumberKind.I64, value, 0, 0);
    public static SconNumber FromU64(ulong value) => new(NumberKind.U64, 0, value, 0);
    public static SconNumber FromF64(double value) => double.IsFinite(value) ? new(NumberKind.F64, 0, 0, value) : throw new SconException(ErrorCode.InvalidNumber, "float value must be finite");
    public static SconNumber Parse(string raw)
    {
        try
        {
            if (raw.Contains('.') || raw.Contains('e') || raw.Contains('E')) return FromF64(double.Parse(raw, CultureInfo.InvariantCulture));
            return raw.StartsWith('-') ? FromI64(long.Parse(raw, CultureInfo.InvariantCulture)) : FromU64(ulong.Parse(raw, CultureInfo.InvariantCulture));
        }
        catch (Exception ex) when (ex is FormatException or OverflowException or SconException)
        {
            throw new SconException(ErrorCode.InvalidNumber, $"invalid SCON number {raw}");
        }
    }
    public long AsI64() => Kind switch
    {
        NumberKind.I64 => I64Value,
        NumberKind.U64 when U64Value <= long.MaxValue => (long)U64Value,
        _ => throw new SconException(ErrorCode.Serde, "integer overflow"),
    };
    public ulong AsU64() => Kind switch
    {
        NumberKind.U64 => U64Value,
        NumberKind.I64 when I64Value >= 0 => (ulong)I64Value,
        _ => throw new SconException(ErrorCode.Serde, "integer overflow"),
    };
    public double AsF64() => Kind switch { NumberKind.I64 => I64Value, NumberKind.U64 => U64Value, _ => F64Value };
    public object PlainValue() => Kind switch { NumberKind.I64 => I64Value, NumberKind.U64 => U64Value <= long.MaxValue ? U64Value : U64Value.ToString(CultureInfo.InvariantCulture), _ => F64Value };
    public string ToSconString() => Kind switch
    {
        NumberKind.I64 => I64Value.ToString(CultureInfo.InvariantCulture),
        NumberKind.U64 => U64Value.ToString(CultureInfo.InvariantCulture),
        _ => F64Value.ToString("G17", CultureInfo.InvariantCulture),
    };
}
public sealed record SconNumberValue(SconNumber Number) : SconValue;

public sealed record SconArray : SconValue, IEnumerable<SconValue>
{
    private readonly List<SconValue> _items = [];
    public SconArray() {}
    public SconArray(IEnumerable<SconValue> values) => _items.AddRange(values);
    public int Count => _items.Count;
    public SconValue this[int index] => _items[index];
    public void Add(SconValue value) => _items.Add(value);
    public void AddRange(IEnumerable<SconValue> values) => _items.AddRange(values);
    public IEnumerator<SconValue> GetEnumerator() => _items.GetEnumerator();
    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}

public sealed record SconObject : SconValue, IEnumerable<KeyValuePair<string, SconValue>>
{
    private readonly List<KeyValuePair<string, SconValue>> _entries = [];
    private readonly Dictionary<string, int> _index = new(StringComparer.Ordinal);
    public int Count => _entries.Count;
    public SconValue this[string key] { get => Get(key) ?? throw new KeyNotFoundException(key); set => Set(key, value); }
    public void Set(string key, SconValue value)
    {
        if (_index.TryGetValue(key, out var index)) _entries[index] = new(key, value);
        else { _index[key] = _entries.Count; _entries.Add(new(key, value)); }
    }
    public SconValue? Get(string key) => _index.TryGetValue(key, out var index) ? _entries[index].Value : null;
    public IEnumerator<KeyValuePair<string, SconValue>> GetEnumerator() => _entries.GetEnumerator();
    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}

public sealed record LoadOptions(string? IncludeRoot = null, int MaxFileSize = 16 * 1024 * 1024, int MaxIncludeDepth = 64, int MaxIncludeFiles = 1024, int MaxArrayLength = 1_000_000, int MaxObjectDepth = 512);
