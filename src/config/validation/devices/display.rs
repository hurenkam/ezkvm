use anyhow::{Result, anyhow};

use crate::config::DisplayConfig;

pub(super) fn validate_display_config(display: &DisplayConfig) -> Result<()> {
    let valid_types = ["virtio-gpu", "qxl", "cirrus", "vga", "vmware-svga", "none"];
    if !valid_types.contains(&display.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported display type: {}. Supported: {:?}",
            display.r#type,
            valid_types
        ));
    }

    if let Some(vram) = display.vram {
        if vram == 0 {
            return Err(anyhow!("VRAM must be greater than 0 when specified"));
        }
        if vram > 1024 {
            return Err(anyhow!("VRAM cannot exceed 1024 MiB"));
        }

        let vram_supported_types = ["virtio-gpu", "qxl", "vmware-svga"];
        if !vram_supported_types.contains(&display.r#type.as_str()) {
            return Err(anyhow!(
                "Display type '{}' does not support configurable VRAM. Supported: {:?}",
                display.r#type,
                vram_supported_types
            ));
        }
    }

    Ok(())
}
