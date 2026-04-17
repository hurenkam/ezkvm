use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn is_false(value: &bool) -> bool {
    !*value
}

/// Drive configuration.
/// Kept as one type because deserialization, validation rules, and `From<DriveConfig>`
/// argument emission are tightly coupled and should evolve together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveConfig {
    /// Unique identifier for the drive.
    /// When empty, an ID is auto-generated from interface + index at load time.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub id: String,

    /// Path to the disk image.
    /// Defaults to an empty string so media-less cdrom definitions can omit it.
    #[serde(default)]
    pub path: String,

    /// Interface type (virtio, scsi, ide, nvme)
    pub interface: String,

    /// Drive type (disk, cdrom)
    pub r#type: String,

    /// Image format (qcow2, raw, etc.)
    pub format: String,

    /// Whether the drive is read-only
    #[serde(default, skip_serializing_if = "is_false")]
    pub readonly: bool,

    /// Enable discard (TRIM) support
    #[serde(default, skip_serializing_if = "is_false")]
    pub discard: bool,

    /// Enable SSD emulation
    #[serde(default, skip_serializing_if = "is_false")]
    pub ssd: bool,

    /// QEMU cache mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,

    /// QEMU async I/O backend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aio: Option<String>,

    /// Detect-zeroes behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detect_zeroes: Option<String>,

    /// SCSI controller to attach to (for SCSI drives)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,

    /// Boot index for firmware boot ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_index: Option<u32>,

    /// SCSI target ID for attached SCSI devices
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scsi_id: Option<u32>,

    /// Rotation rate: 1 for SSD (non-rotating), 0 or None for HDD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_rate: Option<u32>,

    /// Explicit attachment bus for device-based drive emission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Unit number for IDE/SATA style drive placement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<u32>,
}

impl DriveConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = self.generated_id(candidate_index);
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }

    fn generated_id(&self, index: usize) -> String {
        let prefix = self.interface.trim();
        if prefix.is_empty() {
            format!("drive{}", index)
        } else {
            format!("{}{}", prefix, index)
        }
    }
}

impl From<DriveConfig> for QemuArgs {
    fn from(drive: DriveConfig) -> Self {
        let mut args = QemuArgs::new();
        let has_path = !drive.path.trim().is_empty();

        let drive_node_id = format!("drive-{}", drive.id);
        let needs_attached_device = drive.controller.is_some()
            || drive.boot_index.is_some()
            || drive.scsi_id.is_some()
            || drive.bus.is_some()
            || drive.unit.is_some()
            || drive.interface == "ide"
            || drive.interface == "sata";

        args.push_str("-drive");

        let mut drive_parts = Vec::new();
        if has_path {
            drive_parts.push(format!("file={}", drive.path));
        }

        if needs_attached_device {
            drive_parts.push("if=none".to_string());
            drive_parts.push(format!("id={}", drive_node_id));
            if drive.r#type == "cdrom" {
                drive_parts.push("media=cdrom".to_string());
            }
        } else {
            drive_parts.push(format!("if={}", drive.interface));
        }

        if has_path {
            drive_parts.push(format!("format={}", drive.format));
        }

        if drive.readonly {
            drive_parts.push("readonly=on".to_string());
        }

        if has_path && drive.discard {
            drive_parts.push("discard=unmap".to_string());
        }

        if has_path {
            if let Some(cache) = drive.cache.as_ref() {
                drive_parts.push(format!("cache={}", cache));
            }

            if let Some(aio) = drive.aio.as_ref() {
                drive_parts.push(format!("aio={}", aio));
            }

            if let Some(detect_zeroes) = drive.detect_zeroes.as_ref() {
                drive_parts.push(format!("detect-zeroes={}", detect_zeroes));
            }
        }

        args.push(drive_parts.join(","));

        if needs_attached_device {
            args.push_str("-device");
            let mut device_spec = match drive.interface.as_str() {
                "scsi" => format!(
                    "{},drive={},id={}",
                    if drive.r#type == "cdrom" {
                        "scsi-cd"
                    } else {
                        "scsi-hd"
                    },
                    drive_node_id,
                    drive.id
                ),
                "ide" => format!(
                    "{},drive={},id={}",
                    if drive.r#type == "cdrom" {
                        "ide-cd"
                    } else {
                        "ide-hd"
                    },
                    drive_node_id,
                    drive.id
                ),
                "sata" => format!(
                    "{},drive={},id={}",
                    if drive.r#type == "cdrom" {
                        "ide-cd"
                    } else {
                        "ide-hd"
                    },
                    drive_node_id,
                    drive.id
                ),
                "virtio" => format!("virtio-blk-pci,drive={},id={}", drive_node_id, drive.id),
                "nvme" => format!("nvme,drive={},id={}", drive_node_id, drive.id),
                _ => format!(
                    "{},drive={},id={}",
                    drive.interface, drive_node_id, drive.id
                ),
            };

            let attachment_bus = drive.bus.clone().or_else(|| {
                drive
                    .controller
                    .as_ref()
                    .map(|controller| format!("{}.0", controller))
            });
            if let Some(bus) = attachment_bus {
                device_spec.push_str(&format!(",bus={}", bus));
            }

            if let Some(unit) = drive.unit {
                device_spec.push_str(&format!(",unit={}", unit));
            }

            if let Some(scsi_id) = drive.scsi_id {
                device_spec.push_str(&format!(",scsi-id={}", scsi_id));
            }

            if let Some(rotation_rate) = drive.rotation_rate {
                device_spec.push_str(&format!(",rotation_rate={}", rotation_rate));
            }

            if let Some(boot_index) = drive.boot_index {
                device_spec.push_str(&format!(",bootindex={}", boot_index));
            }

            args.push(device_spec);
        }

        args
    }
}
