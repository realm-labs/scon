package io.github.realmlabs.scon;

public final class SconNumber implements SconValue {
    public enum Kind { I64, U64, F64 }

    private final Kind kind;
    private final long i64;
    private final long u64;
    private final double f64;

    private SconNumber(Kind kind, long i64, long u64, double f64) {
        this.kind = kind;
        this.i64 = i64;
        this.u64 = u64;
        this.f64 = f64;
    }

    public static SconNumber ofI64(long value) {
        return new SconNumber(Kind.I64, value, 0, 0);
    }

    public static SconNumber ofU64(long value) {
        return new SconNumber(Kind.U64, 0, value, 0);
    }

    public static SconNumber ofF64(double value) {
        if (!Double.isFinite(value)) {
            throw new SconException(ErrorCode.InvalidNumber, "float value must be finite");
        }
        return new SconNumber(Kind.F64, 0, 0, value);
    }

    public static SconNumber parse(String raw) {
        try {
            if (raw.indexOf('.') >= 0 || raw.indexOf('e') >= 0 || raw.indexOf('E') >= 0) {
                return ofF64(Double.parseDouble(raw));
            }
            if (raw.startsWith("-")) {
                return ofI64(Long.parseLong(raw));
            }
            return ofU64(Long.parseUnsignedLong(raw));
        } catch (RuntimeException ex) {
            throw new SconException(ErrorCode.InvalidNumber, "invalid SCON number " + raw);
        }
    }

    public Kind kind() {
        return kind;
    }

    public long asI64() {
        return switch (kind) {
            case I64 -> i64;
            case U64 -> {
                if (Long.compareUnsigned(u64, Long.MAX_VALUE) > 0) {
                    throw new SconException(ErrorCode.Serde, "integer overflow");
                }
                yield u64;
            }
            case F64 -> throw new SconException(ErrorCode.Serde, "float cannot be decoded as integer");
        };
    }

    public long asU64() {
        return switch (kind) {
            case U64 -> u64;
            case I64 -> {
                if (i64 < 0) {
                    throw new SconException(ErrorCode.Serde, "integer overflow");
                }
                yield i64;
            }
            case F64 -> throw new SconException(ErrorCode.Serde, "float cannot be decoded as integer");
        };
    }

    public double asF64() {
        return switch (kind) {
            case I64 -> (double) i64;
            case U64 -> Double.parseDouble(Long.toUnsignedString(u64));
            case F64 -> f64;
        };
    }

    public Object plainValue() {
        return switch (kind) {
            case I64 -> i64;
            case U64 -> Long.compareUnsigned(u64, Long.MAX_VALUE) <= 0 ? u64 : Long.toUnsignedString(u64);
            case F64 -> f64;
        };
    }

    public String toSconString() {
        return switch (kind) {
            case I64 -> Long.toString(i64);
            case U64 -> Long.toUnsignedString(u64);
            case F64 -> Double.toString(f64);
        };
    }

    @Override
    public boolean equals(Object obj) {
        if (!(obj instanceof SconNumber other)) return false;
        return kind == other.kind && i64 == other.i64 && u64 == other.u64 && Double.compare(f64, other.f64) == 0;
    }

    @Override
    public int hashCode() {
        return java.util.Objects.hash(kind, i64, u64, f64);
    }
}
