use anyhow::{Result, anyhow};

use crate::config::{DeviceConfig, DisplayConfig, DriveConfig, NetworkConfig, SerialConfig};

pub(crate) fn validate_device_config(devices: &DeviceConfig) -> Result<()> {
    for drive in &devices.drives {
        validate_drive_config(drive)?;
    }

    for network in &devices.networks {
        validate_network_config(network)?;
    }

    for display in &devices.displays {
        validate_display_config(display)?;
    }

    for serial in &devices.serials {
        validate_serial_config(serial)?;
    }

    Ok(())
}

fn validate_drive_config(drive: &DriveConfig) -> Result<()> {
    let has_path = !drive.path.trim().is_empty();

    let valid_interfaces = ["virtio", "scsi", "ide", "nvme"];
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

    if let Some(bus) = &drive.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Drive bus cannot be empty"));
    }

    if let Some(unit) = drive.unit {
        if drive.interface != "ide" {
            return Err(anyhow!(
                "Drive unit is currently only supported for ide interfaces"
            ));
        }
        if unit > 3 {
            return Err(anyhow!("Drive unit cannot exceed 3 for ide interfaces"));
        }
    }

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

fn validate_network_config(network: &NetworkConfig) -> Result<()> {
    let valid_models = ["virtio-net", "virtio-net-pci", "e1000", "e1000e", "rtl8139"];
    if !valid_models.contains(&network.model.as_str()) {
        return Err(anyhow!(
            "Unsupported network model: {}. Supported: {:?}",
            network.model,
            valid_models
        ));
    }

    let valid_mode_prefixes = ["user", "bridge", "socket", "tap"];
    let mode_valid = valid_mode_prefixes
        .iter()
        .any(|prefix| network.mode.starts_with(prefix));
    if !mode_valid {
        return Err(anyhow!(
            "Unsupported network mode: {}. Must start with one of: {:?}",
            network.mode,
            valid_mode_prefixes
        ));
    }

    if let Some(mac) = &network.mac
        && !is_valid_mac_address(mac)
    {
        return Err(anyhow!("Invalid MAC address format: {}", mac));
    }

    if let Some(rx_queue_size) = network.rx_queue_size
        && rx_queue_size == 0
    {
        return Err(anyhow!("RX queue size must be greater than 0"));
    }

    if let Some(tx_queue_size) = network.tx_queue_size
        && tx_queue_size == 0
    {
        return Err(anyhow!("TX queue size must be greater than 0"));
    }

    if let Some(bus) = &network.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Network bus cannot be empty"));
    }

    if let Some(addr) = &network.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("Network device address cannot be empty"));
    }

    Ok(())
}

fn validate_display_config(display: &DisplayConfig) -> Result<()> {
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

fn validate_serial_config(serial: &SerialConfig) -> Result<()> {
    let valid_types = ["pty", "stdio", "file", "socket"];
    if !valid_types.contains(&serial.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported serial type: {}. Supported: {:?}",
            serial.r#type,
            valid_types
        ));
    }

    match serial.r#type.as_str() {
        "file" => {
            let path = serial.path.as_deref().unwrap_or("").trim();
            if path.is_empty() {
                return Err(anyhow!("Serial file backend requires a non-empty path"));
            }
        }
        "socket" => {
            let host = serial.host.as_deref().unwrap_or("").trim();
            if host.is_empty() {
                return Err(anyhow!("Serial socket backend requires a non-empty host"));
            }

            match serial.socket_port {
                Some(0) | None => {
                    return Err(anyhow!(
                        "Serial socket backend requires a TCP port between 1 and 65535"
                    ));
                }
                Some(_) => {}
            }
        }
        "pty" | "stdio" => {}
        _ => unreachable!(),
    }

    Ok(())
}

fn is_valid_mac_address(mac: &str) -> bool {
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return false;
    }

    for part in parts {
        if part.len() != 2 {
            return false;
        }
        if u8::from_str_radix(part, 16).is_err() {
            return false;
        }
    }

    true
}
