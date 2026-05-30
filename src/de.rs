use std::fmt;

use serde::de::{
    self, DeserializeSeed, Deserializer as SerdeDeserializer, EnumAccess, Error as DeError,
    IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};

use crate::error::{Error, ErrorCode, Result};
use crate::value::Value;

pub(crate) struct Deserializer {
    value: Value,
}

impl Deserializer {
    pub(crate) fn new(value: Value) -> Self {
        Self { value }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(value) => visitor.visit_bool(value),
            Value::Number(value) => {
                if value.contains(['.', 'e', 'E']) {
                    visitor.visit_f64(value.parse::<f64>().map_err(Error::custom)?)
                } else if value.starts_with('-') {
                    visitor.visit_i64(value.parse::<i64>().map_err(Error::custom)?)
                } else {
                    visitor.visit_u64(value.parse::<u64>().map_err(Error::custom)?)
                }
            }
            Value::String(value) => visitor.visit_string(value),
            Value::Array(values) => visitor.visit_seq(SeqDe {
                iter: values.into_iter(),
            }),
            Value::Object(values) => visitor.visit_map(MapDe {
                iter: values.into_iter(),
                value: None,
            }),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bool(value) => visitor.visit_bool(value),
            other => Err(type_error("bool", &other)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_i8(parse_signed(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_i16(parse_signed(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_i32(parse_signed(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_i64(parse_signed(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_u8(parse_unsigned(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_u16(parse_unsigned(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_u32(parse_unsigned(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(text) => visitor.visit_u64(parse_unsigned(text)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(value) => visitor.visit_f32(value.parse::<f32>().map_err(Error::custom)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Number(value) => visitor.visit_f64(value.parse::<f64>().map_err(Error::custom)?),
            other => Err(type_error("number", &other)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => {
                let mut chars = value.chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) => visitor.visit_char(ch),
                    _ => Err(Error::new(
                        ErrorCode::Serde,
                        "expected single-character string",
                    )),
                }
            }
            other => Err(type_error("string", &other)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => visitor.visit_string(value),
            other => Err(type_error("string", &other)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Array(values) => {
                let mut bytes = Vec::with_capacity(values.len());
                for value in values {
                    let Value::Number(text) = value else {
                        return Err(type_error("byte array", &value));
                    };
                    bytes.push(text.parse::<u8>().map_err(Error::custom)?);
                }
                visitor.visit_byte_buf(bytes)
            }
            other => Err(type_error("byte array", &other)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_none(),
            value => visitor.visit_some(Deserializer::new(value)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            other => Err(type_error("null", &other)),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Array(values) => visitor.visit_seq(SeqDe {
                iter: values.into_iter(),
            }),
            other => Err(type_error("array", &other)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Object(values) => visitor.visit_map(MapDe {
                iter: values.into_iter(),
                value: None,
            }),
            other => Err(type_error("object", &other)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(variant) => visitor.visit_enum(variant.into_deserializer()),
            Value::Object(mut object) if object.len() == 1 => {
                let (variant, value) = object.pop().unwrap();
                visitor.visit_enum(EnumDe { variant, value })
            }
            other => Err(type_error("externally tagged enum", &other)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

fn parse_signed<T>(text: String) -> Result<T>
where
    T: TryFrom<i64>,
    T::Error: fmt::Display,
{
    let parsed = text.parse::<i64>().map_err(Error::custom)?;
    T::try_from(parsed).map_err(Error::custom)
}

fn parse_unsigned<T>(text: String) -> Result<T>
where
    T: TryFrom<u64>,
    T::Error: fmt::Display,
{
    let parsed = text.parse::<u64>().map_err(Error::custom)?;
    T::try_from(parsed).map_err(Error::custom)
}

struct SeqDe {
    iter: std::vec::IntoIter<Value>,
}

impl<'de> SeqAccess<'de> for SeqDe {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(|value| seed.deserialize(Deserializer::new(value)))
            .transpose()
    }
}

struct MapDe {
    iter: indexmap::map::IntoIter<String, Value>,
    value: Option<Value>,
}

impl<'de> MapAccess<'de> for MapDe {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .value
            .take()
            .ok_or_else(|| Error::new(ErrorCode::Serde, "missing map value"))?;
        seed.deserialize(Deserializer::new(value))
    }
}

struct EnumDe {
    variant: String,
    value: Value,
}

impl<'de> EnumAccess<'de> for EnumDe {
    type Error = Error;
    type Variant = VariantDe;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, VariantDe { value: self.value }))
    }
}

struct VariantDe {
    value: Value,
}

impl<'de> VariantAccess<'de> for VariantDe {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.value {
            Value::Null => Ok(()),
            other => Err(type_error("null", &other)),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer::new(self.value))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Deserializer::new(self.value).deserialize_seq(visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Deserializer::new(self.value).deserialize_map(visitor)
    }
}

fn type_error(expected: &str, value: &Value) -> Error {
    Error::new(
        ErrorCode::TypeMismatch,
        format!("expected {expected}, found {}", type_name(value)),
    )
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
