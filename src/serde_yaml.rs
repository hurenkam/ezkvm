//! Serde-compatible YAML deserializer and serializer using saphyr.
//!
//! This module provides a drop-in replacement for `serde_yaml` functions, using saphyr's
//! YAML parser and emitter. It implements `serde::Deserializer` and `serde::Serializer`
//! traits directly over saphyr's AST, avoiding JSON intermediate formats.

mod de;
mod error;
mod ser;

pub use de::Deserializer;
pub use error::Error;
pub use ser::Serializer;

use saphyr::{LoadableYamlNode, YamlEmitter, YamlOwned};

/// Deserialize a value from a YAML string.
pub fn from_str<T: for<'de> serde::Deserialize<'de>>(s: &str) -> Result<T, Error> {
    let docs = YamlOwned::load_from_str(s).map_err(|e| Error::Parse(e.to_string()))?;
    if docs.is_empty() {
        return Err(Error::Message("empty YAML document".to_string()));
    }
    T::deserialize(Deserializer::new(&docs[0]))
}

/// Serialize a value to a YAML string.
pub fn to_string<T: serde::Serialize>(value: &T) -> Result<String, Error> {
    let yaml = value.serialize(Serializer)?;
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    let yaml_ref: saphyr::Yaml = (&yaml).into();
    emitter
        .dump(&yaml_ref)
        .map_err(|e| Error::Message(format!("YAML emit error: {:?}", e)))?;
    Ok(output
        .strip_prefix("---\n")
        .unwrap_or(&output)
        .trim_end_matches('\n')
        .to_string())
}

/// Deserialize a value from a `YamlOwned` value.
#[allow(dead_code)]
pub fn from_value<'a, T: serde::Deserialize<'a>>(value: &'a YamlOwned) -> Result<T, Error> {
    T::deserialize(Deserializer::new(value))
}

/// Serialize a value to a `YamlOwned` value.
pub fn to_value<T: serde::Serialize>(value: &T) -> Result<YamlOwned, Error> {
    value.serialize(Serializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_bool() {
        let result: bool = from_str("true").unwrap();
        assert!(result);
    }

    #[test]
    fn test_deserialize_string() {
        let result: String = from_str("hello").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_deserialize_int() {
        let result: i32 = from_str("42").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_serialize_bool() {
        let result = to_string(&true).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn test_serialize_string() {
        let result = to_string(&"hello").unwrap();
        assert!(result.contains("hello"));
    }
}
