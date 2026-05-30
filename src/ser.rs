use indexmap::IndexMap;
use serde::ser::{
    self, Impossible, Serialize, SerializeMap, SerializeSeq, SerializeStruct,
    SerializeStructVariant, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};

use crate::error::{Error, ErrorCode, Result};
use crate::value::Value;

pub(crate) struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SeqSer;
    type SerializeTuple = SeqSer;
    type SerializeTupleStruct = SeqSer;
    type SerializeTupleVariant = TupleVariantSer;
    type SerializeMap = MapSer;
    type SerializeStruct = MapSer;
    type SerializeStructVariant = StructVariantSer;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_i16(self, v: i16) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_i32(self, v: i32) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_u8(self, v: u8) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_u16(self, v: u16) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_u32(self, v: u32) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }
    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::Number(v.to_string()))
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        serialize_float(v)
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        serialize_float(v)
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::Array(
            v.iter()
                .map(|byte| Value::Number(byte.to_string()))
                .collect(),
        ))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        let mut object = IndexMap::new();
        object.insert(variant.to_string(), value.serialize(Serializer)?);
        Ok(Value::Object(object))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SeqSer> {
        Ok(SeqSer {
            values: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SeqSer> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SeqSer> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantSer> {
        Ok(TupleVariantSer {
            variant: variant.to_string(),
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<MapSer> {
        Ok(MapSer {
            object: IndexMap::with_capacity(len.unwrap_or(0)),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<MapSer> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<StructVariantSer> {
        Ok(StructVariantSer {
            variant: variant.to_string(),
            inner: MapSer {
                object: IndexMap::with_capacity(len),
                next_key: None,
            },
        })
    }
}

fn serialize_float<T: ToString + Copy + Into<f64>>(value: T) -> Result<Value> {
    let as_f64: f64 = value.into();
    if !as_f64.is_finite() {
        return Err(Error::new(
            ErrorCode::Serde,
            "SCON cannot serialize NaN or infinite floats",
        ));
    }
    Ok(Value::Number(value.to_string()))
}

pub(crate) struct SeqSer {
    values: Vec<Value>,
}

impl SerializeSeq for SeqSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.values.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(self.values))
    }
}

impl SerializeTuple for SeqSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for SeqSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        SerializeSeq::end(self)
    }
}

pub(crate) struct TupleVariantSer {
    variant: String,
    values: Vec<Value>,
}

impl SerializeTupleVariant for TupleVariantSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.values.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut object = IndexMap::new();
        object.insert(self.variant, Value::Array(self.values));
        Ok(Value::Object(object))
    }
}

pub(crate) struct MapSer {
    object: IndexMap<String, Value>,
    next_key: Option<String>,
}

impl SerializeMap for MapSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.next_key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| Error::new(ErrorCode::Serde, "serialize_value called before key"))?;
        self.object.insert(key, value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Object(self.object))
    }
}

impl SerializeStruct for MapSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.object
            .insert(key.to_string(), value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Object(self.object))
    }
}

pub(crate) struct StructVariantSer {
    variant: String,
    inner: MapSer,
}

impl SerializeStructVariant for StructVariantSer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.inner.serialize_field(key, value)
    }

    fn end(self) -> Result<Value> {
        let mut object = IndexMap::new();
        object.insert(self.variant, Value::Object(self.inner.object));
        Ok(Value::Object(object))
    }
}

struct KeySerializer;

impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Impossible<String, Error>;
    type SerializeTuple = Impossible<String, Error>;
    type SerializeTupleStruct = Impossible<String, Error>;
    type SerializeTupleVariant = Impossible<String, Error>;
    type SerializeMap = Impossible<String, Error>;
    type SerializeStruct = Impossible<String, Error>;
    type SerializeStructVariant = Impossible<String, Error>;

    fn serialize_str(self, value: &str) -> Result<String> {
        Ok(value.to_string())
    }

    fn serialize_char(self, value: char) -> Result<String> {
        Ok(value.to_string())
    }

    fn serialize_bool(self, _v: bool) -> Result<String> {
        Err(key_error())
    }
    fn serialize_i8(self, _v: i8) -> Result<String> {
        Err(key_error())
    }
    fn serialize_i16(self, _v: i16) -> Result<String> {
        Err(key_error())
    }
    fn serialize_i32(self, _v: i32) -> Result<String> {
        Err(key_error())
    }
    fn serialize_i64(self, _v: i64) -> Result<String> {
        Err(key_error())
    }
    fn serialize_u8(self, _v: u8) -> Result<String> {
        Err(key_error())
    }
    fn serialize_u16(self, _v: u16) -> Result<String> {
        Err(key_error())
    }
    fn serialize_u32(self, _v: u32) -> Result<String> {
        Err(key_error())
    }
    fn serialize_u64(self, _v: u64) -> Result<String> {
        Err(key_error())
    }
    fn serialize_f32(self, _v: f32) -> Result<String> {
        Err(key_error())
    }
    fn serialize_f64(self, _v: f64) -> Result<String> {
        Err(key_error())
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<String> {
        Err(key_error())
    }
    fn serialize_none(self) -> Result<String> {
        Err(key_error())
    }
    fn serialize_some<T>(self, _value: &T) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Err(key_error())
    }
    fn serialize_unit(self) -> Result<String> {
        Err(key_error())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String> {
        Err(key_error())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<String> {
        Ok(variant.to_string())
    }
    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Err(key_error())
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Ok(variant.to_string())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(key_error())
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(key_error())
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(key_error())
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(key_error())
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(key_error())
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(key_error())
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(key_error())
    }
}

fn key_error() -> Error {
    Error::new(ErrorCode::Serde, "SCON map keys must serialize as strings")
}
