use serde::{Deserialize, Serialize};

/// TPM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmConfig {
    /// TPM version (1.2 or 2.0)
    pub version: String,

    /// TPM backend type (emulator or passthrough)
    pub backend: String,

    /// Path to TPM socket file (overrides the default run-dir path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_path: Option<String>,

    /// Directory containing swtpm state files.
    /// When set, swtpm is invoked with --tpmstate dir=<state_dir> using this path
    /// instead of the default empty run-dir subdirectory.
    /// Use this to point at a mounted Proxmox TPM-state disk so Windows keeps its
    /// existing BitLocker keys (e.g. mount /dev/vm1/vm-108-tpmstate → /mnt/tpmstate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_dir: Option<String>,

    /// URI for swtpm state backend, passed as `--tpmstate backend-uri=<uri>`.
    /// This can be used instead of mounting a TPM-state volume and configuring
    /// `state_dir`.
    /// Example: `file:///dev/vm1/vm-108-tpmstate`
    #[serde(alias = "state_backend_url", skip_serializing_if = "Option::is_none")]
    pub state_backend_uri: Option<String>,

    /// Device model (tpm-tis or tpm-crb)
    #[serde(default = "default_tpm_model")]
    pub model: String,
}

fn default_tpm_model() -> String {
    "tpm-tis".to_string()
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Guest agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestAgentConfig {
    /// Enable guest agent
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// Path to guest agent socket
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,

    /// Freeze CPU on suspend
    #[serde(default, skip_serializing_if = "is_false")]
    pub freeze_cpu: bool,

    /// PCI/PCIe bus placement for the virtio-serial controller
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
}

/// Memory ballooning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallooningConfig {
    /// Enable memory ballooning
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,

    /// Enable free page reporting
    #[serde(default, skip_serializing_if = "is_false")]
    pub free_page_reporting: bool,

    /// Balloon device model
    #[serde(default = "default_balloon_model")]
    pub model: String,

    /// Optional device identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// PCI/PCIe bus placement for the balloon device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
}

fn default_balloon_model() -> String {
    "virtio-balloon-pci".to_string()
}
