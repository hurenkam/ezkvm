use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Display type (virtio-gpu, qxl, cirrus)
    pub r#type: String,

    /// Video RAM in MiB
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram: Option<u32>,
}

impl From<DisplayConfig> for QemuArgs {
    fn from(display: DisplayConfig) -> Self {
        let mut args = QemuArgs::new();

        args.push_str("-device");
        let mut device_spec = display.r#type.clone();

        if let Some(vram) = display.vram {
            match display.r#type.as_str() {
                "qxl" => device_spec.push_str(&format!(",vram_size_mb={}", vram)),
                "vmware-svga" => device_spec.push_str(&format!(",vgamem_mb={}", vram)),
                _ => device_spec.push_str(&format!(",vram={}", vram * 1024 * 1024)),
            }
        }

        args.push(device_spec);
        args
    }
}
