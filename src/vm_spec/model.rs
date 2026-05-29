use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CanonicalDocument {
    pub metadata: Metadata,
    pub virtual_machine: VirtualMachine,
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub schema_version: String,
    pub vm_name: String,
}

#[derive(Debug, Deserialize)]
pub struct VirtualMachine {
    pub system: System,
    #[serde(default)]
    pub storage: Vec<StorageEntry>,
    #[serde(default)]
    pub network: Vec<NetworkEntry>,
    #[serde(default)]
    pub resources: Vec<ResourceRef>,
}

#[derive(Debug, Deserialize)]
pub struct System {
    pub machine: Machine,
    pub cpu: Cpu,
    pub memory: Memory,
}

#[derive(Debug, Deserialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
}

#[derive(Debug, Deserialize)]
pub struct Cpu {
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct Memory {
    pub min: i64,
}

#[derive(Debug, Deserialize)]
pub struct StorageEntry {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct NetworkEntry {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResourceRef {
    pub id: String,
}
