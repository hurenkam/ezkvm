use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// SCSI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScsiControllerConfig {
    /// Unique identifier for the controller.
    /// When empty, auto-generated as "scsihw{index}" at load time.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub id: String,

    /// Controller type (pvscsi, virtio-scsi, lsi, etc.)
    #[serde(default = "default_scsi_controller_type")]
    pub r#type: String,

    /// Number of I/O queues (for virtio-scsi)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iothread: Option<String>,

    /// Maximum number of targets
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_targets: Option<u32>,

    /// PCI/PCIe bus placement for the controller
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
}

fn default_scsi_controller_type() -> String {
    "virtio-scsi-pci".to_string()
}

impl ScsiControllerConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = format!("scsihw{}", candidate_index);
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }
}

/// SATA controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SataControllerConfig {
    /// Unique identifier for the controller.
    /// When empty, auto-generated as "sata{index}" at load time.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub id: String,

    /// Controller type (currently ahci)
    #[serde(default = "default_sata_controller_type")]
    pub r#type: String,

    /// PCI/PCIe bus placement for the controller
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
}

fn default_sata_controller_type() -> String {
    "ahci".to_string()
}

impl SataControllerConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = format!("sata{}", candidate_index);
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator: Option<String>,

    /// Username for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Password for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// SCSI controller to attach to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,
}
