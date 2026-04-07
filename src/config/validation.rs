//! Configuration validation module
//!
//! Validates VM configurations for correctness and compatibility.

use super::VmConfig;
use anyhow::{anyhow, Result};

/// Validate a complete VM configuration
pub fn validate_config(config: &VmConfig) -> Result<()> {
    // Validate backend
    if config.backend != "qemu" {
        return Err(anyhow!("Unsupported backend: {}. Only 'qemu' is currently supported.", config.backend));
    }
    
    // Validate system configuration
    validate_system_config(&config.system)?;
    
    // Validate boot configuration
    validate_boot_config(&config.boot)?;
    
    // Validate device configuration
    validate_device_config(&config.devices)?;
    
    Ok(())
}

/// Validate system configuration
fn validate_system_config(system: &super::SystemConfig) -> Result<()> {
    // Validate architecture
    let valid_architectures = ["x86_64", "aarch64", "x86", "ppc64", "riscv64"];
    if !valid_architectures.contains(&system.architecture.as_str()) {
        return Err(anyhow!("Unsupported architecture: {}. Supported: {:?}", 
                          system.architecture, valid_architectures));
    }
    
    // Validate memory (reasonable bounds)
    if system.memory < 128 {
        return Err(anyhow!("Memory must be at least 128 MiB"));
    }
    if system.memory > 1024 * 1024 { // 1 TiB
        return Err(anyhow!("Memory cannot exceed 1 TiB"));
    }
    
    // Validate vCPUs
    if system.vcpus == 0 {
        return Err(anyhow!("Must have at least 1 vCPU"));
    }
    if system.vcpus > 1024 {
        return Err(anyhow!("Cannot have more than 1024 vCPUs"));
    }
    
    // Validate CPU features
    for feature in &system.cpu_features {
        if !feature.name.starts_with('+') && !feature.name.starts_with('-') {
            return Err(anyhow!("CPU feature '{}' must start with '+' or '-'", feature.name));
        }
    }
    
    Ok(())
}

/// Validate boot configuration
fn validate_boot_config(boot: &super::BootConfig) -> Result<()> {
    // Validate firmware
    if let Some(firmware) = &boot.firmware {
        let valid_firmware = ["uefi", "bios"];
        if !valid_firmware.contains(&firmware.as_str()) {
            return Err(anyhow!("Unsupported firmware: {}. Supported: {:?}", 
                              firmware, valid_firmware));
        }
    }
    
    // Validate boot order
    let valid_boot_devices = ["disk", "cdrom", "network", "hd", "cd"];
    for device in &boot.boot_order {
        if !valid_boot_devices.contains(&device.as_str()) {
            return Err(anyhow!("Unsupported boot device: {}. Supported: {:?}", 
                              device, valid_boot_devices));
        }
    }
    
    Ok(())
}

/// Validate device configuration
fn validate_device_config(devices: &super::DeviceConfig) -> Result<()> {
    // Validate drives
    for drive in &devices.drives {
        validate_drive_config(drive)?;
    }
    
    // Validate networks
    for network in &devices.networks {
        validate_network_config(network)?;
    }
    
    // Validate displays
    for display in &devices.displays {
        validate_display_config(display)?;
    }
    
    Ok(())
}

/// Validate drive configuration
fn validate_drive_config(drive: &super::DriveConfig) -> Result<()> {
    // Validate interface
    let valid_interfaces = ["virtio", "scsi", "ide", "nvme"];
    if !valid_interfaces.contains(&drive.interface.as_str()) {
        return Err(anyhow!("Unsupported drive interface: {}. Supported: {:?}", 
                          drive.interface, valid_interfaces));
    }
    
    // Validate type
    let valid_types = ["disk", "cdrom"];
    if !valid_types.contains(&drive.r#type.as_str()) {
        return Err(anyhow!("Unsupported drive type: {}. Supported: {:?}", 
                          drive.r#type, valid_types));
    }
    
    // Validate format
    let valid_formats = ["qcow2", "raw", "vmdk", "vdi"];
    if !valid_formats.contains(&drive.format.as_str()) {
        return Err(anyhow!("Unsupported drive format: {}. Supported: {:?}", 
                          drive.format, valid_formats));
    }
    
    // Check if path exists (optional, but warn if not)
    if !std::path::Path::new(&drive.path).exists() {
        eprintln!("Warning: Drive path '{}' does not exist", drive.path);
    }
    
    Ok(())
}

/// Validate network configuration
fn validate_network_config(network: &super::NetworkConfig) -> Result<()> {
    // Validate model
    let valid_models = ["virtio-net", "e1000", "e1000e", "rtl8139"];
    if !valid_models.contains(&network.model.as_str()) {
        return Err(anyhow!("Unsupported network model: {}. Supported: {:?}", 
                          network.model, valid_models));
    }
    
    // Validate mode
    let valid_modes = ["user", "bridge", "socket", "tap"];
    if !valid_modes.contains(&network.mode.as_str()) {
        return Err(anyhow!("Unsupported network mode: {}. Supported: {:?}", 
                          network.mode, valid_modes));
    }
    
    // Validate MAC address format if provided
    if let Some(mac) = &network.mac {
        if !is_valid_mac_address(mac) {
            return Err(anyhow!("Invalid MAC address format: {}", mac));
        }
    }
    
    Ok(())
}

/// Validate display configuration
fn validate_display_config(display: &super::DisplayConfig) -> Result<()> {
    // Validate type
    let valid_types = ["virtio-gpu", "qxl", "cirrus", "vga", "vmware-svga"];
    if !valid_types.contains(&display.r#type.as_str()) {
        return Err(anyhow!("Unsupported display type: {}. Supported: {:?}", 
                          display.r#type, valid_types));
    }
    
    // Validate VRAM
    if let Some(vram) = display.vram {
        if vram > 1024 { // 1 GiB
            return Err(anyhow!("VRAM cannot exceed 1024 MiB"));
        }
    }
    
    Ok(())
}

/// Check if a string is a valid MAC address
fn is_valid_mac_address(mac: &str) -> bool {
    // Basic MAC address validation (XX:XX:XX:XX:XX:XX format)
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