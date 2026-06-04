use std::fmt;

use serde::{Deserialize, Serialize};

/// Schema version for ezkvm runtime config specification.
pub const EZKVM_CONFIG_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub metadata: Metadata,
    pub virtual_machine: VirtualMachine,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub schema_version: String,
    pub vm_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub system: System,
    #[serde(default)]
    pub storage: Vec<StorageEntry>,
    #[serde(default)]
    pub network: Vec<NetworkEntry>,
    #[serde(default)]
    pub resources: Vec<ResourceRef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct System {
    pub machine: Machine,
    pub cpu: Cpu,
    pub memory: Memory,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cpu {
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Memory {
    pub min: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageEntry {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkEntry {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResourceRef {
    pub id: String,
}

impl fmt::Display for RuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = serde_yaml::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&rendered)
    }
}

// Conversion implementations for YAML parsing
impl From<String> for StorageEntry {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for NetworkEntry {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for ResourceRef {
    fn from(id: String) -> Self {
        Self { id }
    }
}
