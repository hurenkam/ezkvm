use anyhow::{Result, anyhow};

use crate::config::DriveConfig;

pub(super) fn validate_drive_config(drive: &DriveConfig) -> Result<()> {
    validate_drive_enums(drive)?;
    validate_drive_optional_modes(drive)?;
    validate_drive_bus_and_unit(drive)?;
    validate_drive_path_rules(drive)?;
    Ok(())
}

fn validate_drive_enums(drive: &DriveConfig) -> Result<()> {
    let valid_interfaces = ["virtio", "scsi", "ide", "sata", "nvme"];
    if !valid_interfaces.contains(&drive.interface.as_str()) {
        return Err(anyhow!(
            "Unsupported drive interface: {}. Supported: {:?}",
            drive.interface,
            valid_interfaces
        ));
    }

    let valid_types = ["disk", "cdrom"];
    if !valid_types.contains(&drive.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported drive type: {}. Supported: {:?}",
            drive.r#type,
            valid_types
        ));
    }

    let valid_formats = ["qcow2", "raw", "vmdk", "vdi"];
    if !valid_formats.contains(&drive.format.as_str()) {
        return Err(anyhow!(
            "Unsupported drive format: {}. Supported: {:?}",
            drive.format,
            valid_formats
        ));
    }

    Ok(())
}

fn validate_drive_optional_modes(drive: &DriveConfig) -> Result<()> {
    if let Some(cache) = &drive.cache {
        let valid_cache = ["none", "writeback", "writethrough", "unsafe", "directsync"];
        if !valid_cache.contains(&cache.as_str()) {
            return Err(anyhow!(
                "Unsupported drive cache mode: {}. Supported: {:?}",
                cache,
                valid_cache
            ));
        }
    }

    if let Some(aio) = &drive.aio {
        let valid_aio = ["threads", "native", "io_uring"];
        if !valid_aio.contains(&aio.as_str()) {
            return Err(anyhow!(
                "Unsupported drive aio mode: {}. Supported: {:?}",
                aio,
                valid_aio
            ));
        }
    }

    if let Some(detect_zeroes) = &drive.detect_zeroes {
        let valid_detect_zeroes = ["off", "on", "unmap"];
        if !valid_detect_zeroes.contains(&detect_zeroes.as_str()) {
            return Err(anyhow!(
                "Unsupported detect-zeroes mode: {}. Supported: {:?}",
                detect_zeroes,
                valid_detect_zeroes
            ));
        }
    }

    if let Some(boot_index) = drive.boot_index
        && boot_index == 0
    {
        return Err(anyhow!("Drive boot_index must be greater than 0"));
    }

    if let Some(scsi_id) = drive.scsi_id {
        if drive.interface != "scsi" {
            return Err(anyhow!("Drive scsi_id is only valid for scsi interfaces"));
        }
        if scsi_id > 255 {
            return Err(anyhow!("Drive scsi_id cannot exceed 255"));
        }
    }

    Ok(())
}

fn validate_drive_bus_and_unit(drive: &DriveConfig) -> Result<()> {
    if let Some(bus) = &drive.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Drive bus cannot be empty"));
    }

    if let Some(unit) = drive.unit {
        if drive.interface != "ide" && drive.interface != "sata" {
            return Err(anyhow!(
                "Drive unit is currently only supported for ide and sata interfaces"
            ));
        }
        if drive.interface == "ide" && unit > 3 {
            return Err(anyhow!("Drive unit cannot exceed 3 for ide interfaces"));
        }
        if drive.interface == "sata" && unit > 5 {
            return Err(anyhow!("Drive unit cannot exceed 5 for sata interfaces"));
        }
    }

    Ok(())
}

fn validate_drive_path_rules(drive: &DriveConfig) -> Result<()> {
    let has_path = !drive.path.trim().is_empty();

    if !has_path && drive.r#type != "cdrom" {
        return Err(anyhow!(
            "Drive path cannot be empty unless drive type is cdrom"
        ));
    }

    if has_path && !std::path::Path::new(&drive.path).exists() {
        eprintln!("Warning: Drive path '{}' does not exist", drive.path);
    }

    Ok(())
}
