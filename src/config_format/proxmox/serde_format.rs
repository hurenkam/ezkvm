//! Serde text-format bridge for Proxmox `.conf` schema documents.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use super::schema::ProxmoxConfigSchema;

/// Deserializes Proxmox text into the schema using the Stage A parser.
pub fn from_str(source: &str) -> Result<ProxmoxConfigSchema, String> {
    ProxmoxConfigSchema::parse(source).map_err(|error| error.to_string())
}

/// Serializes the schema into Proxmox text using the Stage A renderer.
pub fn to_string(schema: &ProxmoxConfigSchema) -> String {
    schema.render()
}

/// Wrapper that serializes a Proxmox schema as a string in serde payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxmoxText(pub ProxmoxConfigSchema);

impl From<ProxmoxConfigSchema> for ProxmoxText {
    fn from(value: ProxmoxConfigSchema) -> Self {
        Self(value)
    }
}

impl From<ProxmoxText> for ProxmoxConfigSchema {
    fn from(value: ProxmoxText) -> Self {
        value.0
    }
}

impl Serialize for ProxmoxText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.render())
    }
}

impl<'de> Deserialize<'de> for ProxmoxText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        ProxmoxConfigSchema::parse(&source)
            .map(Self)
            .map_err(D::Error::custom)
    }
}

/// Field-level serde helpers for serializing a schema as Proxmox text.
pub mod as_text {
    use super::*;

    pub fn serialize<S>(value: &ProxmoxConfigSchema, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.render())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ProxmoxConfigSchema, D::Error>
    where
        D: Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        ProxmoxConfigSchema::parse(&source).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{ProxmoxConfigSchema, ProxmoxText, as_text};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct WrapperWithText {
        proxmox: ProxmoxText,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct WrapperWithAttribute {
        #[serde(with = "as_text")]
        proxmox: ProxmoxConfigSchema,
    }

    fn sample_source() -> &'static str {
        r#"name: demo
machine: q35
memory: 8192
scsi0: vm1-pool:vm-200-disk-1,cache=writeback,size=32G

[clean]
name: demo
memory: 4096
"#
    }

    #[test]
    fn proxmox_text_roundtrips_in_json_string_payload() {
        let schema = ProxmoxConfigSchema::parse(sample_source()).expect("schema should parse");
        let wrapped = WrapperWithText {
            proxmox: ProxmoxText::from(schema.clone()),
        };

        let json = serde_json::to_string(&wrapped).expect("json encode should succeed");
        let decoded: WrapperWithText =
            serde_json::from_str(&json).expect("json decode should succeed");

        assert_eq!(decoded.proxmox.0, schema);
    }

    #[test]
    fn serde_with_attribute_roundtrips_schema_as_text() {
        let schema = ProxmoxConfigSchema::parse(sample_source()).expect("schema should parse");
        let wrapped = WrapperWithAttribute {
            proxmox: schema.clone(),
        };

        let json = serde_json::to_string(&wrapped).expect("json encode should succeed");
        let decoded: WrapperWithAttribute =
            serde_json::from_str(&json).expect("json decode should succeed");

        assert_eq!(decoded.proxmox, schema);
    }
}
