//! Storage management for VMs
//!
//! Handles QCOW2 image creation, snapshots, and disk operations.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Default directory for VM disk images
pub fn get_storage_dir() -> Result<PathBuf> {
    let data_home = if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data)
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow!("HOME environment variable not set"))?;
        PathBuf::from(home).join(".local/share")
    };
    
    let storage_dir = data_home.join("ezkvm/disks");
    fs::create_dir_all(&storage_dir)?;
    Ok(storage_dir)
}

/// Create a QCOW2 disk image
pub fn create_qcow2(name: &str, size_gb: u32, backing_file: Option<&str>) -> Result<PathBuf> {
    let storage_dir = get_storage_dir()?;
    let disk_path = storage_dir.join(format!("{}.qcow2", name));
    
    // Check if disk already exists
    if disk_path.exists() {
        return Err(anyhow!("Disk image '{}' already exists at {}", name, disk_path.display()));
    }
    
    let size = format!("{}G", size_gb);
    let mut cmd = Command::new("qemu-img");
    cmd.args(&["create", "-f", "qcow2"]);
    
    // Add backing file if specified
    if let Some(backing) = backing_file {
        cmd.args(&["-b", backing, "-F", "qcow2"]);
    }
    
    cmd.args(&[disk_path.to_str().unwrap(), &size]);
    
    let output = cmd.output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;
    
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to create QCOW2 image: {}", err));
    }
    
    println!("✓ Created QCOW2 image: {} ({} GB)", name, size_gb);
    Ok(disk_path)
}

/// Get information about a disk image
pub fn get_disk_info(disk_path: &Path) -> Result<DiskInfo> {
    if !disk_path.exists() {
        return Err(anyhow!("Disk image not found: {}", disk_path.display()));
    }
    
    let cmd = Command::new("qemu-img")
        .args(&["info", "--output=json", disk_path.to_str().unwrap()])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to get disk info: {}", err));
    }
    
    let output = String::from_utf8(cmd.stdout)?;
    let info: serde_json::Value = serde_json::from_str(&output)?;
    
    let size = info["virtual-size"]
        .as_u64()
        .unwrap_or(0) / (1024 * 1024 * 1024); // Convert to GB
    
    let actual_size = info["actual-size"]
        .as_u64()
        .unwrap_or(0) / (1024 * 1024); // Convert to MB
    
    let format = info["format"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    
    Ok(DiskInfo {
        format,
        size_gb: size as u32,
        actual_size_mb: actual_size as u32,
        path: disk_path.to_path_buf(),
    })
}

/// Disk information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub format: String,
    pub size_gb: u32,
    pub actual_size_mb: u32,
    pub path: PathBuf,
}

/// Create a snapshot of a disk image
pub fn create_snapshot(disk_path: &Path, snapshot_name: &str) -> Result<PathBuf> {
    if !disk_path.exists() {
        return Err(anyhow!("Disk image not found: {}", disk_path.display()));
    }
    
    let snapshot_path = disk_path.with_file_name(
        format!("{}-{}.qcow2", 
            disk_path.file_stem().unwrap().to_str().unwrap(),
            snapshot_name)
    );
    
    let cmd = Command::new("qemu-img")
        .args(&["create", "-f", "qcow2", "-b", 
                disk_path.to_str().unwrap(), "-F", "qcow2",
                snapshot_path.to_str().unwrap()])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to create snapshot: {}", err));
    }
    
    println!("✓ Created snapshot: {} -> {}", snapshot_name, snapshot_path.display());
    Ok(snapshot_path)
}

/// List all available disk images
pub fn list_disks() -> Result<Vec<DiskInfo>> {
    let storage_dir = get_storage_dir()?;
    
    if !storage_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut disks = Vec::new();
    
    for entry in fs::read_dir(&storage_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map(|e| e == "qcow2").unwrap_or(false) {
            if let Ok(info) = get_disk_info(&path) {
                disks.push(info);
            }
        }
    }
    
    // Sort by path for consistent output
    disks.sort_by(|a, b| a.path.cmp(&b.path));
    
    Ok(disks)
}

/// Resize a disk image
pub fn resize_disk(disk_path: &Path, new_size_gb: u32) -> Result<()> {
    if !disk_path.exists() {
        return Err(anyhow!("Disk image not found: {}", disk_path.display()));
    }
    
    let size = format!("{}G", new_size_gb);
    let cmd = Command::new("qemu-img")
        .args(&["resize", disk_path.to_str().unwrap(), &size])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to resize disk: {}", err));
    }
    
    println!("✓ Resized disk to {} GB", new_size_gb);
    Ok(())
}

/// Convert disk image format
pub fn convert_disk(source: &Path, dest: &Path, format: &str) -> Result<()> {
    if !source.exists() {
        return Err(anyhow!("Source disk not found: {}", source.display()));
    }
    
    if dest.exists() {
        return Err(anyhow!("Destination already exists: {}", dest.display()));
    }
    
    let cmd = Command::new("qemu-img")
        .args(&["convert", "-f", "qcow2", "-O", format, 
                source.to_str().unwrap(), dest.to_str().unwrap()])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to convert disk: {}", err));
    }
    
    println!("✓ Converted disk from qcow2 to {} format", format);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_dir_creation() {
        let storage_dir = get_storage_dir();
        assert!(storage_dir.is_ok());
        assert!(storage_dir.unwrap().exists());
    }

    #[test]
    fn test_list_disks() {
        let disks = list_disks();
        assert!(disks.is_ok());
    }
}
