use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// QMP monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmpConfig {
    /// Enable QMP monitoring
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Socket path for QMP connection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,

    /// Socket type (unix, tcp)
    #[serde(default)]
    pub socket_type: QmpSocketType,
}

/// QMP socket type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QmpSocketType {
    #[default]
    Unix,
    Tcp,
}

/// SMBIOS system information configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbiosConfig {
    /// Manufacturer name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,

    /// Product name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,

    /// Version string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Serial number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,

    /// UUID for the VM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    /// SKU number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,

    /// Family name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,

    /// VM generation ID (for Windows Server 2016+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_generation_id: Option<String>,
}

/// NUMA node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumaConfig {
    /// NUMA node ID
    pub id: u32,

    /// Memory size for this node in MiB
    pub memory: u32,

    /// CPU cores assigned to this node
    pub cpus: Vec<u32>,

    /// Host NUMA node to bind to (for host-passthrough)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_node: Option<u32>,
}
