use serde::{Deserialize, Serialize};

/// SCSI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScsiControllerConfig {
    /// Unique identifier for the controller
    pub id: String,

    /// Controller type (pvscsi, virtio-scsi, lsi, etc.)
    #[serde(default = "default_scsi_controller_type")]
    pub r#type: String,

    /// Number of I/O queues (for virtio-scsi)
    #[serde(default)]
    pub iothread: Option<String>,

    /// Maximum number of targets
    #[serde(default)]
    pub max_targets: Option<u32>,

    /// PCI/PCIe bus placement for the controller
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    pub addr: Option<String>,
}

fn default_scsi_controller_type() -> String {
    "virtio-scsi-pci".to_string()
}

/// iSCSI disk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IscsiDiskConfig {
    /// Unique identifier for the disk
    pub id: String,

    /// iSCSI target portal (host:port)
    pub portal: String,

    /// iSCSI target IQN
    pub target: String,

    /// LUN number
    #[serde(default)]
    pub lun: u32,

    /// Initiator IQN (optional)
    pub initiator: Option<String>,

    /// Username for authentication
    pub username: Option<String>,

    /// Password for authentication
    pub password: Option<String>,

    /// SCSI controller to attach to
    pub controller: Option<String>,
}
