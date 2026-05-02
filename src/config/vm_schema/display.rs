use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

fn qemu_display_device_model(display_type: &str) -> &str {
    match display_type {
        "vga" => "VGA",
        _ => display_type,
    }
}

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

        // Skip device emission for "none" display (explicit vga=none from Proxmox)
        if display.r#type == "none" {
            return args;
        }

        args.push_str("-device");
        let mut device_spec = qemu_display_device_model(&display.r#type).to_string();

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

#[cfg(test)]
mod tests {
    use super::DisplayConfig;
    use crate::qemu::types::QemuArgs;

    #[test]
    fn vga_alias_maps_to_qemu_vga_model() {
        let display = DisplayConfig {
            r#type: "vga".to_string(),
            vram: None,
        };

        let args = QemuArgs::from(display).into_inner();
        assert_eq!(args, vec!["-device".to_string(), "VGA".to_string()]);
    }

}
