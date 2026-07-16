use hashlink::LinkedHashMap;
use ordered_float::OrderedFloat;
use saphyr::{ScalarOwned, YamlOwned};
use serde::ser;

use super::error::Error;

/// Serializer for Rust types to `YamlOwned` values.
pub struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = YamlOwned;
    type Error = Error;
    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeSeq;
    type SerializeTupleStruct = SerializeSeq;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::Boolean(v)))
    }

    fn serialize_i8(self, v: i8) -> Result<YamlOwned, Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> Result<YamlOwned, Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> Result<YamlOwned, Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::Integer(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<YamlOwned, Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u16(self, v: u16) -> Result<YamlOwned, Error> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u32(self, v: u32) -> Result<YamlOwned, Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::Integer(v as i64)))
    }

    fn serialize_f32(self, v: f32) -> Result<YamlOwned, Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::FloatingPoint(
            OrderedFloat::from(v),
        )))
    }

    fn serialize_char(self, v: char) -> Result<YamlOwned, Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::String(v.to_string())))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<YamlOwned, Error> {
        self.serialize_str(&String::from_utf8_lossy(v))
    }

    fn serialize_none(self) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::Null))
    }

    fn serialize_some<T: ser::Serialize + ?Sized>(self, value: &T) -> Result<YamlOwned, Error> {
        value.serialize(Serializer)
    }

    fn serialize_unit(self) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Value(ScalarOwned::Null))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<YamlOwned, Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<YamlOwned, Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<YamlOwned, Error> {
        value.serialize(Serializer)
    }

    fn serialize_newtype_variant<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<YamlOwned, Error> {
        let mut map = LinkedHashMap::new();
        let inner = value.serialize(Serializer)?;
        map.insert(
            YamlOwned::Value(ScalarOwned::String(variant.to_string())),
            inner,
        );
        Ok(YamlOwned::Mapping(map))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<SerializeSeq, Error> {
        Ok(SerializeSeq { items: Vec::new() })
    }

    fn serialize_tuple(self, len: usize) -> Result<SerializeSeq, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<SerializeSeq, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<SerializeTupleVariant, Error> {
        Ok(SerializeTupleVariant {
            variant: variant.to_string(),
            items: Vec::new(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<SerializeMap, Error> {
        Ok(SerializeMap {
            entries: LinkedHashMap::new(),
            current_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<SerializeMap, Error> {
        self.serialize_map(None)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<SerializeStructVariant, Error> {
        Ok(SerializeStructVariant {
            variant: variant.to_string(),
            entries: LinkedHashMap::new(),
            current_key: None,
        })
    }
}

// ── SerializeSeq ──────────────────────────────────────────────────────────────

pub struct SerializeSeq {
    items: Vec<YamlOwned>,
}

impl ser::SerializeSeq for SerializeSeq {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.items.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Sequence(self.items))
    }
}

impl ser::SerializeTuple for SerializeSeq {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<YamlOwned, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SerializeSeq {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<YamlOwned, Error> {
        ser::SerializeSeq::end(self)
    }
}

// ── SerializeTupleVariant ─────────────────────────────────────────────────────

pub struct SerializeTupleVariant {
    variant: String,
    items: Vec<YamlOwned>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.items.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<YamlOwned, Error> {
        let mut map = LinkedHashMap::new();
        map.insert(
            YamlOwned::Value(ScalarOwned::String(self.variant)),
            YamlOwned::Sequence(self.items),
        );
        Ok(YamlOwned::Mapping(map))
    }
}

// ── SerializeMap / SerializeStruct ────────────────────────────────────────────

pub struct SerializeMap {
    entries: LinkedHashMap<YamlOwned, YamlOwned>,
    current_key: Option<YamlOwned>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_key<T: ser::Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.current_key = Some(key.serialize(Serializer)?);
        Ok(())
    }

    fn serialize_value<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self.current_key.take().ok_or(Error::UnexpectedEnd)?;
        let val = value.serialize(Serializer)?;
        self.entries.insert(key, val);
        Ok(())
    }

    fn end(self) -> Result<YamlOwned, Error> {
        Ok(YamlOwned::Mapping(self.entries))
    }
}

impl ser::SerializeStruct for SerializeMap {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        let val = value.serialize(Serializer)?;
        self.entries
            .insert(YamlOwned::Value(ScalarOwned::String(key.to_string())), val);
        Ok(())
    }

    fn end(self) -> Result<YamlOwned, Error> {
        ser::SerializeMap::end(self)
    }
}

// ── SerializeStructVariant ────────────────────────────────────────────────────

pub struct SerializeStructVariant {
    variant: String,
    entries: LinkedHashMap<YamlOwned, YamlOwned>,
    #[allow(dead_code)]
    current_key: Option<YamlOwned>,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = YamlOwned;
    type Error = Error;

    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        let val = value.serialize(Serializer)?;
        self.entries
            .insert(YamlOwned::Value(ScalarOwned::String(key.to_string())), val);
        Ok(())
    }

    fn end(self) -> Result<YamlOwned, Error> {
        let mut map = LinkedHashMap::new();
        map.insert(
            YamlOwned::Value(ScalarOwned::String(self.variant)),
            YamlOwned::Mapping(self.entries),
        );
        Ok(YamlOwned::Mapping(map))
    }
}
