package io.github.realmlabs.scon

public sealed interface SconValue {
    public data object Null : SconValue
    public data class Bool(val value: Boolean) : SconValue
    public data class Number(val value: SconNumber) : SconValue
    public data class StringValue(val value: String) : SconValue
    public data class ArrayValue(val values: List<SconValue>) : SconValue
    public data class ObjectValue(val values: LinkedHashMap<String, SconValue>) : SconValue
}

public sealed interface SconNumber {
    public fun toSconString(): String
    public fun asLongOrNull(): Long?
    public fun asULongOrNull(): ULong?
    public fun asDouble(): Double

    public data class I64(val value: Long) : SconNumber {
        override fun toSconString(): String = value.toString()
        override fun asLongOrNull(): Long = value
        override fun asULongOrNull(): ULong? = if (value >= 0) value.toULong() else null
        override fun asDouble(): Double = value.toDouble()
    }

    public data class U64(val value: ULong) : SconNumber {
        override fun toSconString(): String = value.toString()
        override fun asLongOrNull(): Long? = if (value <= Long.MAX_VALUE.toULong()) value.toLong() else null
        override fun asULongOrNull(): ULong = value
        override fun asDouble(): Double = value.toDouble()
    }

    public data class F64(val value: Double) : SconNumber {
        override fun toSconString(): String = value.toString()
        override fun asLongOrNull(): Long? = null
        override fun asULongOrNull(): ULong? = null
        override fun asDouble(): Double = value
    }

    public companion object {
        public fun parse(raw: String): SconNumber {
            return if (raw.indexOf('.') >= 0 || raw.indexOf('e') >= 0 || raw.indexOf('E') >= 0) {
                val value = raw.toDoubleOrNull()
                    ?: throw invalidNumber(raw)
                if (!value.isFinite()) throw invalidNumber(raw)
                F64(value)
            } else if (raw.startsWith("-")) {
                I64(raw.toLongOrNull() ?: throw invalidNumber(raw))
            } else {
                U64(raw.toULongOrNull() ?: throw invalidNumber(raw))
            }
        }

        private fun invalidNumber(raw: String): SconException =
            SconException(
                SconError(
                    code = SconErrorCode.InvalidNumber,
                    message = "invalid SCON number: $raw",
                ),
            )
    }

}
