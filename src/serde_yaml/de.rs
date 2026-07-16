use saphyr::{ScalarOwned, YamlOwned};
use serde::de::{self, DeserializeSeed, Deserializer as DeserializerTrait, Visitor};

use super::error::Error;

/// Deserializer for YAML values.
pub struct Deserializer<'de> {
    value: &'de YamlOwned,
}

impl<'de> Deserializer<'de> {
    pub(super) fn new(value: &'de YamlOwned) -> Self {
        Deserializer { value }
    }

    fn deserialize_scalar<V: Visitor<'de>>(
        self,
        scalar: &'de ScalarOwned,
        visitor: V,
    ) -> Result<V::Value, Error> {
        match scalar {
            ScalarOwned::Boolean(b) => visitor.visit_bool(*b),
            ScalarOwned::Integer(i) => visitor.visit_i64(*i),
            ScalarOwned::FloatingPoint(f) => visitor.visit_f64(f.into_inner()),
            ScalarOwned::String(s) => visitor.visit_str(s),
            ScalarOwned::Null => visitor.visit_none(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self.value {
            YamlOwned::Value(_) => "scalar",
            YamlOwned::Mapping(_) => "mapping",
            YamlOwned::Sequence(_) => "sequence",
            YamlOwned::Tagged(_, _) => "tagged",
            YamlOwned::Representation(_, _, _) => "representation",
            YamlOwned::Alias(_) => "alias",
            YamlOwned::BadValue => "bad value",
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(scalar) => self.deserialize_scalar(scalar, visitor),
            YamlOwned::Mapping(_) => self.deserialize_map(visitor),
            YamlOwned::Sequence(_) => self.deserialize_seq(visitor),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_any(visitor),
            YamlOwned::Representation(s, _, _) => visitor.visit_str(s),
            YamlOwned::Alias(_) => Err(Error::Message("unresolved alias".to_string())),
            YamlOwned::BadValue => Err(Error::Message("bad value".to_string())),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::Boolean(b)) => visitor.visit_bool(*b),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_bool(visitor),
            _ => Err(Error::InvalidType {
                expected: "bool".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::Integer(i)) => visitor.visit_i64(*i),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_i64(visitor),
            _ => Err(Error::InvalidType {
                expected: "integer".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::Integer(i)) => {
                if *i >= 0 {
                    visitor.visit_u64(*i as u64)
                } else {
                    Err(Error::Message(format!("negative integer for u64: {}", i)))
                }
            }
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_u64(visitor),
            _ => Err(Error::InvalidType {
                expected: "unsigned integer".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::FloatingPoint(f)) => visitor.visit_f64(f.into_inner()),
            YamlOwned::Value(ScalarOwned::Integer(i)) => visitor.visit_f64(*i as f64),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_f64(visitor),
            _ => Err(Error::InvalidType {
                expected: "float".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::String(s)) => visitor.visit_str(s),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_str(visitor),
            _ => Err(Error::InvalidType {
                expected: "string".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::Null) => visitor.visit_none(),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_option(visitor),
            _ => visitor.visit_some(Deserializer::new(self.value)),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::Null) => visitor.visit_unit(),
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_unit(visitor),
            _ => Err(Error::InvalidType {
                expected: "null".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(Deserializer::new(self.value))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Sequence(seq) => {
                let access = SeqAccess::new(seq.iter());
                visitor.visit_seq(access)
            }
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_seq(visitor),
            _ => Err(Error::InvalidType {
                expected: "sequence".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Mapping(map) => {
                let access = MapAccess::new(map.iter());
                visitor.visit_map(access)
            }
            YamlOwned::Tagged(_, inner) => Deserializer::new(inner).deserialize_map(visitor),
            _ => Err(Error::InvalidType {
                expected: "mapping".to_string(),
                got: self.type_name().to_string(),
            }),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::String(s)) => {
                // String-based enum variant (like "Host" for CpuModel).
                // Create a static null value that represents the unit variant.
                const NULL_YAML: YamlOwned = YamlOwned::Value(ScalarOwned::Null);
                // SAFETY: null_ref is never mutated and lives for the duration of deserialization.
                let null_ref: &'de YamlOwned = unsafe {
                    std::mem::transmute::<&'static YamlOwned, &'de YamlOwned>(&NULL_YAML)
                };
                visitor.visit_enum(EnumAccess::unit_variant(s, null_ref))
            }
            YamlOwned::Mapping(map) => {
                if map.len() == 1 {
                    if let Some((k, v)) = map.iter().next() {
                        let variant_name = match k {
                            YamlOwned::Value(ScalarOwned::String(s)) => s,
                            _ => {
                                return Err(Error::Message(
                                    "enum variant must be a string key".to_string(),
                                ));
                            }
                        };
                        visitor.visit_enum(EnumAccess::new(variant_name, v))
                    } else {
                        Err(Error::Message("empty enum mapping".to_string()))
                    }
                } else {
                    // For untagged enums with multiple keys, deserialize as a map
                    // and let serde try each variant in turn.
                    self.deserialize_map(visitor)
                }
            }
            YamlOwned::Tagged(_, inner) => {
                Deserializer::new(inner).deserialize_enum(_name, _variants, visitor)
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
}

// ── SeqAccess ────────────────────────────────────────────────────────────────

struct SeqAccess<'de> {
    iter: std::vec::IntoIter<&'de YamlOwned>,
}

impl<'de> SeqAccess<'de> {
    fn new(seq: impl ExactSizeIterator<Item = &'de YamlOwned>) -> Self {
        SeqAccess {
            iter: seq.collect::<Vec<_>>().into_iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if let Some(value) = self.iter.next() {
            seed.deserialize(Deserializer::new(value)).map(Some)
        } else {
            Ok(None)
        }
    }
}

// ── MapAccess ─────────────────────────────────────────────────────────────────

struct MapAccess<'de> {
    iter: std::vec::IntoIter<(&'de YamlOwned, &'de YamlOwned)>,
    current_value: Option<&'de YamlOwned>,
}

impl<'de> MapAccess<'de> {
    fn new(map: impl ExactSizeIterator<Item = (&'de YamlOwned, &'de YamlOwned)>) -> Self {
        MapAccess {
            iter: map.collect::<Vec<_>>().into_iter(),
            current_value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        if let Some((k, v)) = self.iter.next() {
            self.current_value = Some(v);
            seed.deserialize(Deserializer::new(k)).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let value = self.current_value.take().ok_or(Error::UnexpectedEnd)?;
        seed.deserialize(Deserializer::new(value))
    }
}

// ── BorrowedStrDeserializer ───────────────────────────────────────────────────

/// Deserializer for a borrowed string slice, used for enum variant identifiers.
struct BorrowedStrDeserializer<'de> {
    value: &'de str,
}

impl<'de> BorrowedStrDeserializer<'de> {
    fn new(value: &'de str) -> Self {
        BorrowedStrDeserializer { value }
    }
}

impl<'de> de::Deserializer<'de> for BorrowedStrDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_str(self.value)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_str(self.value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_str(self.value)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_str(self.value)
    }

    #[inline]
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
}

// ── UnitDeserializer ──────────────────────────────────────────────────────────

/// Deserializer that always produces a unit value, used for untagged enums.
struct UnitDeserializer;

impl<'de> de::Deserializer<'de> for UnitDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_none()
    }

    #[inline]
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
    #[inline]
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_any(visitor)
    }
}

// ── EnumAccess / VariantAccess ────────────────────────────────────────────────

struct EnumAccess<'de> {
    variant: Option<Box<str>>,
    value: &'de YamlOwned,
}

impl<'de> EnumAccess<'de> {
    fn new(variant: &str, value: &'de YamlOwned) -> Self {
        EnumAccess {
            variant: Some(variant.into()),
            value,
        }
    }

    fn unit_variant(variant: &str, null_value: &'de YamlOwned) -> Self {
        EnumAccess {
            variant: Some(variant.into()),
            value: null_value,
        }
    }
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Error> {
        if let Some(variant_name) = self.variant {
            // SAFETY: The leaked string is immediately re-borrowed with the shorter 'de lifetime,
            // so it won't outlive the deserialization call-stack.
            let variant_str: &'static str = Box::leak(variant_name.clone());
            let variant_str: &'de str =
                unsafe { std::mem::transmute::<&'static str, &'de str>(variant_str) };
            let variant_value = seed.deserialize(BorrowedStrDeserializer::new(variant_str))?;
            Ok((variant_value, VariantAccess { value: self.value }))
        } else {
            let unit_value = seed.deserialize(UnitDeserializer)?;
            Ok((unit_value, VariantAccess { value: self.value }))
        }
    }
}

struct VariantAccess<'de> {
    value: &'de YamlOwned,
}

impl<'de> de::VariantAccess<'de> for VariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            YamlOwned::Value(ScalarOwned::Null) => Ok(()),
            _ => Err(Error::Message("expected null for unit variant".to_string())),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(Deserializer::new(self.value))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        DeserializerTrait::deserialize_seq(Deserializer::new(self.value), visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        DeserializerTrait::deserialize_map(Deserializer::new(self.value), visitor)
    }
}
