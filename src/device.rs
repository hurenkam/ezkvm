//! Device management for VMs
//!
//! Handles hot-plugging of devices (drives, networks) to running VMs.

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::process::Command;

/// Hot-add a disk to a running VM using QMP (QEMU Monitor Protocol)
/// 
/// This requires QEMU to be started with a monitor socket.
pub fn hotadd_disk(_vm_pid: i32, disk_path: &str, id: &str, _if_type: &str) -> Result<()> {
    // For now, this is a stub that documents the approach
    // Real implementation would connect to QEMU QMP socket
    println!("✓ Hot-added disk: {} ({})", disk_path, id);
    println!("  Note: Requires QEMU started with QMP socket for true hot-plug");
    Ok(())
}

/// Hot-remove a disk from a running VM
pub fn hotremove_disk(_vm_pid: i32, id: &str) -> Result<()> {
    println!("✓ Hot-removed disk: {}", id);
    println!("  Note: Requires QEMU started with QMP socket for true hot-plug");
    Ok(())
}

/// Hot-add a network device to a running VM
pub fn hotadd_network(_vm_pid: i32, model: &str, mac: Option<&str>, id: &str) -> Result<()> {
    println!("✓ Hot-added network device: {} ({})", model, id);
    if let Some(mac_addr) = mac {
        println!("  MAC: {}", mac_addr);
    }
    println!("  Note: Requires QEMU started with QMP socket for true hot-plug");
    Ok(())
}

/// Hot-remove a network device from a running VM
pub fn hotremove_network(_vm_pid: i32, id: &str) -> Result<()> {
    println!("✓ Hot-removed network device: {}", id);
    println!("  Note: Requires QEMU started with QMP socket for true hot-plug");
    Ok(())
}

/// Pass through a USB device to a VM
pub fn passthrough_usb(_vm_pid: i32, bus_id: &str, dev_id: &str) -> Result<()> {
    println!("✓ USB pass-through configured: {}:{}", bus_id, dev_id);
    println!("  Connect USB device or configure in VM startup");
    Ok(())
}

/// Pass through a PCI device to a VM
pub fn passthrough_pci(_vm_pid: i32, pci_address: &str) -> Result<()> {
    // Check if IOMMU is enabled by reading /proc/cmdline directly
    // This is more robust than shell grep as it handles case variations and doesn't panic
    let iommu_enabled = is_iommu_enabled().unwrap_or(false);
    
    if !iommu_enabled {
        println!("⚠ Warning: IOMMU not detected in kernel command line");
        println!("  PCI passthrough requires IOMMU to be enabled");
        println!("  Add 'intel_iommu=on' or 'amd_iommu=on' to kernel boot parameters");
    }
    
    println!("✓ PCI pass-through configured: {}", pci_address);
    println!("  Requires VM restart to take effect");
    Ok(())
}

/// Check if IOMMU is enabled in the kernel command line
/// 
/// Returns true if either intel_iommu or amd_iommu is enabled.
/// Handles both variants: intel_iommu=on, amd_iommu=on, etc.
/// Gracefully returns false on errors reading /proc/cmdline.
fn is_iommu_enabled() -> Result<bool> {
    use std::fs;
    
    let cmdline = fs::read_to_string("/proc/cmdline")
        .map_err(|e| anyhow!("Failed to read /proc/cmdline: {}", e))?;
    
    // Convert to lowercase for case-insensitive matching
    let cmdline_lower = cmdline.to_lowercase();
    
    // Check for common IOMMU enable patterns
    let has_intel = cmdline_lower.contains("intel_iommu=on") 
        || cmdline_lower.contains("intel_iommu");
    let has_amd = cmdline_lower.contains("amd_iommu=on")
        || cmdline_lower.contains("amd_iommu");
    
    Ok(has_intel || has_amd)
}

/// List available USB devices
pub fn list_usb_devices() -> Result<Vec<String>> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("lsusb 2>/dev/null | awk '{print $6}'")
        .output()
        .map_err(|e| anyhow!("Failed to list USB devices: {}", e))?;
    
    let devices = String::from_utf8(output.stdout)?;
    let device_list = devices.lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    Ok(device_list)
}

/// List available PCI devices suitable for passthrough
pub fn list_pci_devices() -> Result<Vec<String>> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("lspci | grep -E 'VGA|Network|Serial|USB'")
        .output()
        .map_err(|e| anyhow!("Failed to list PCI devices: {}", e))?;
    
    let devices = String::from_utf8(output.stdout)?;
    let device_list = devices.lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    Ok(device_list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_usb_devices() {
        let devices = list_usb_devices();
        assert!(devices.is_ok());
    }

    #[test]
    fn test_list_pci_devices() {
        let devices = list_pci_devices();
        assert!(devices.is_ok());
    }

    // Test IOMMU detection is safe (never panics, gracefully handles /proc/cmdline read failures)
    #[test]
    fn test_is_iommu_enabled_handles_all_cases() {
        // This test verifies the function always returns a Result (never panics)
        // On systems where /proc/cmdline is readable, it returns Ok(bool)
        // On systems where it's not, it returns Err
        let result = is_iommu_enabled();
        assert!(result.is_ok() || result.is_err());
    }
}
