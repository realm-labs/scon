package io.github.realmlabs.scon.kotlinx

import io.github.realmlabs.scon.SconException
import io.github.realmlabs.scon.SconNumber
import io.github.realmlabs.scon.SconValue
import io.github.realmlabs.scon.parseValue
import io.github.realmlabs.scon.parseValueFile
import io.github.realmlabs.scon.toSconString
import kotlinx.serialization.DeserializationStrategy
import kotlinx.serialization.SerializationException
import kotlinx.serialization.SerializationStrategy
import kotlinx.serialization.StringFormat
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.StructureKind
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.modules.SerializersModule
import kotlinx.serialization.serializer
import java.nio.file.Path

public object Scon : StringFormat {
    override val serializersModule: SerializersModule = SerializersModule {}

    private val json = Json {
        serializersModule = this@Scon.serializersModule
        encodeDefaults = true
        ignoreUnknownKeys = false
        allowSpecialFloatingPointValues = false
    }

    override fun <T> encodeToString(
        serializer: SerializationStrategy<T>,
        value: T,
    ): String {
        try {
            validateEncodableDescriptor(serializer.descriptor)
            val jsonElement = json.encodeToJsonElement(serializer, value)
            return jsonElement.toSconValue().toSconString()
        } catch (error: SconSerializationException) {
            throw error
        } catch (error: SconException) {
            throw SconSerializationException(error.message ?: "SCON serialization failed", error)
        } catch (error: SerializationException) {
            throw SconSerializationException(error.message ?: "SCON serialization failed", error)
        }
    }

    override fun <T> decodeFromString(
        deserializer: DeserializationStrategy<T>,
        string: String,
    ): T {
        val value = parseValue(string)
        try {
            return json.decodeFromJsonElement(deserializer, value.toJsonElement())
        } catch (error: SerializationException) {
            throw SconSerializationException(error.message ?: "SCON deserialization failed", error)
        }
    }
}

public class SconSerializationException(
    message: String,
    cause: Throwable? = null,
) : SerializationException(message, cause)

public inline fun <reified T> Scon.decodeFromString(source: String): T =
    decodeFromString(serializer(), source)

public inline fun <reified T> Scon.encodeToString(value: T): String =
    encodeToString(serializer(), value)

public fun <T> Scon.decodeFromFile(
    deserializer: DeserializationStrategy<T>,
    path: Path,
): T {
    val value = parseValueFile(path)
    try {
        return Json.decodeFromJsonElement(deserializer, value.toJsonElement())
    } catch (error: SerializationException) {
        throw SconSerializationException(error.message ?: "SCON deserialization failed", error)
    }
}

private fun validateEncodableDescriptor(descriptor: SerialDescriptor) {
    validateEncodableDescriptor(descriptor, mutableSetOf())
}

private fun validateEncodableDescriptor(
    descriptor: SerialDescriptor,
    seen: MutableSet<SerialDescriptor>,
) {
    if (!seen.add(descriptor)) return
    if (descriptor.kind == StructureKind.MAP) {
        val keyDescriptor = descriptor.getElementDescriptor(0)
        if (keyDescriptor.kind != PrimitiveKind.STRING) {
            throw SconSerializationException("SCON object keys must serialize as strings")
        }
    }
    for (index in 0 until descriptor.elementsCount) {
        validateEncodableDescriptor(descriptor.getElementDescriptor(index), seen)
    }
}

private fun JsonElement.toSconValue(): SconValue =
    when (this) {
        JsonNull -> SconValue.Null
        is JsonPrimitive -> toSconPrimitiveValue()
        is JsonArray -> SconValue.ArrayValue(map { it.toSconValue() })
        is JsonObject -> SconValue.ObjectValue(entries.associateTo(linkedMapOf()) { it.key to it.value.toSconValue() })
    }

private fun JsonPrimitive.toSconPrimitiveValue(): SconValue {
    if (toString().startsWith("\"")) return SconValue.StringValue(content)
    booleanOrNull?.let { return SconValue.Bool(it) }
    return SconValue.Number(SconNumber.parse(content))
}

private fun SconValue.toJsonElement(): JsonElement =
    when (this) {
        SconValue.Null -> JsonNull
        is SconValue.Bool -> JsonPrimitive(value)
        is SconValue.Number -> Json.parseToJsonElement(value.toSconString())
        is SconValue.StringValue -> JsonPrimitive(value)
        is SconValue.ArrayValue -> JsonArray(values.map { it.toJsonElement() })
        is SconValue.ObjectValue -> JsonObject(values.mapValues { it.value.toJsonElement() })
    }
