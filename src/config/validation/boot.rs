use anyhow::{Result, anyhow};

use crate::config::BootConfig;

pub(crate) fn validate_boot_config(boot: &BootConfig) -> Result<()> {
    if let Some(firmware) = &boot.firmware {
        let valid_firmware = ["uefi", "bios", "ovmf"];
        if !valid_firmware.contains(&firmware.as_str()) {
            return Err(anyhow!(
                "Unsupported firmware: {}. Supported: {:?}",
                firmware,
                valid_firmware
            ));
        }
    }

    let valid_boot_devices = ["disk", "cdrom", "network", "hd", "cd"];
    for device in &boot.boot_order {
        if !valid_boot_devices.contains(&device.as_str()) {
            return Err(anyhow!(
                "Unsupported boot device: {}. Supported: {:?}",
                device,
                valid_boot_devices
            ));
        }
    }

    if let Some(firmware) = &boot.firmware
        && (firmware == "uefi" || firmware == "ovmf")
    {
        if let Some(code_path) = &boot.uefi_code
            && !std::path::Path::new(code_path).exists()
        {
            eprintln!("Warning: UEFI code path '{}' does not exist", code_path);
        }
        if let Some(vars_path) = &boot.uefi_vars
            && !std::path::Path::new(vars_path).exists()
        {
            eprintln!("Warning: UEFI vars path '{}' does not exist", vars_path);
        }
    }

    if let Some(splash) = &boot.splash
        && splash.trim().is_empty()
    {
        return Err(anyhow!("Boot splash path cannot be empty"));
    }

    Ok(())
}
