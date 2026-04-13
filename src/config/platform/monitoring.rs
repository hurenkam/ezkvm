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
    pub manufacturer: Option<String>,

    /// Product name
    pub product: Option<String>,

    /// Version string
    pub version: Option<String>,

    /// Serial number
    pub serial: Option<String>,

    /// UUID for the VM
    pub uuid: Option<String>,

    /// SKU number
    pub sku: Option<String>,

    /// Family name
    pub family: Option<String>,

    /// VM generation ID (for Windows Server 2016+)
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
    pub host_node: Option<u32>,
}
