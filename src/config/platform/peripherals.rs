use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// SPICE display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiceConfig {
    /// Enable SPICE display
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// SPICE server port
    #[serde(default = "default_spice_port")]
    pub port: u16,

    /// SPICE server address
    #[serde(default = "default_spice_addr")]
    pub addr: String,

    /// Disable ticketing (password authentication)
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_ticketing: bool,

    /// Enable SPICE audio
    #[serde(default, skip_serializing_if = "is_false")]
    pub audio: bool,

    /// Enable vdagent (clipboard sharing)
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub vdagent: bool,
}

fn default_spice_port() -> u16 {
    5900
}

fn default_spice_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_vnc_display() -> String {
    "127.0.0.1:0".to_string()
}

/// VNC display server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VncConfig {
    /// Enable VNC display server
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// VNC display endpoint (for example 127.0.0.1:0 or unix:/path)
    #[serde(default = "default_vnc_display")]
    pub display: String,

    /// Enable VNC password requirement
    #[serde(default, skip_serializing_if = "is_false")]
    pub password: bool,
}

/// Audio device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceConfig {
    /// QEMU audio device type
    pub r#type: String,

    /// Unique identifier for the audio device
    pub id: String,

    /// Bus placement for the device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,

    /// Codec address on the parent HDA controller
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cad: Option<u8>,

    /// Backend ID used by codec devices
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audiodev: Option<String>,
}

/// Input device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDeviceConfig {
    /// QEMU input device type
    pub r#type: String,
}

/// Looking Glass shared memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvshmemConfig {
    /// Enable Looking Glass shared memory
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Shared memory size in MiB
    #[serde(default = "default_ivshmem_size")]
    pub size: u32,

    /// Shared memory device vectors
    #[serde(default = "default_ivshmem_vectors")]
    pub vectors: u32,

    /// Shared memory device ID
    #[serde(default = "default_ivshmem_id")]
    pub id: String,

    /// Bus placement for the ivshmem device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Shared memory file path
    #[serde(default = "default_ivshmem_mem_path")]
    pub mem_path: String,
}

fn default_ivshmem_size() -> u32 {
    32
}

fn default_ivshmem_vectors() -> u32 {
    1
}

fn default_ivshmem_id() -> String {
    "ivshmem0".to_string()
}

fn default_ivshmem_mem_path() -> String {
    "/dev/kvmfr0".to_string()
}
