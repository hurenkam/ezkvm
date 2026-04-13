use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{DiskInfo, get_storage_dir};

/// Create a QCOW2 disk image
pub fn create_qcow2(name: &str, size_gb: u32, backing_file: Option<&str>) -> Result<PathBuf> {
    let storage_dir = get_storage_dir()?;
    let disk_path = storage_dir.join(format!("{}.qcow2", name));

    // Check if disk already exists
    if disk_path.exists() {
        return Err(anyhow!(
            "Disk image '{}' already exists at {}",
            name,
            disk_path.display()
        ));
    }

    let size = format!("{}G", size_gb);
    let mut cmd = Command::new("qemu-img");
    cmd.args(["create", "-f", "qcow2"]);

    // Add backing file if specified
    if let Some(backing) = backing_file {
        cmd.args(["-b", backing, "-F", "qcow2"]);
    }

    let disk_path_str = disk_path.to_str().ok_or_else(|| {
        anyhow!(
            "Disk path contains invalid Unicode: {}",
            disk_path.display()
        )
    })?;
    cmd.args([disk_path_str, &size]);

    let output = cmd
        .output()
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

    let disk_path_str = disk_path.to_str().ok_or_else(|| {
        anyhow!(
            "Disk path contains invalid Unicode: {}",
            disk_path.display()
        )
    })?;

    let cmd = Command::new("qemu-img")
        .args(["info", "--output=json", disk_path_str])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to get disk info: {}", err));
    }

    let output = String::from_utf8(cmd.stdout)?;
    let info: serde_json::Value = serde_json::from_str(&output)?;

    let size = info["virtual-size"].as_u64().unwrap_or(0) / (1024 * 1024 * 1024); // Convert to GB

    let actual_size = info["actual-size"].as_u64().unwrap_or(0) / (1024 * 1024); // Convert to MB

    let format = info["format"].as_str().unwrap_or("unknown").to_string();

    Ok(DiskInfo {
        format,
        size_gb: size as u32,
        actual_size_mb: actual_size as u32,
        path: disk_path.to_path_buf(),
    })
}

/// Create a snapshot of a disk image
pub fn create_snapshot(disk_path: &Path, snapshot_name: &str) -> Result<PathBuf> {
    if !disk_path.exists() {
        return Err(anyhow!("Disk image not found: {}", disk_path.display()));
    }

    let file_stem = disk_path
        .file_stem()
        .ok_or_else(|| {
            anyhow!(
                "Invalid disk path, cannot extract filename: {}",
                disk_path.display()
            )
        })?
        .to_str()
        .ok_or_else(|| {
            anyhow!(
                "Disk filename contains invalid Unicode: {}",
                disk_path.display()
            )
        })?;

    let snapshot_path = disk_path.with_file_name(format!("{}-{}.qcow2", file_stem, snapshot_name));

    let disk_path_str = disk_path.to_str().ok_or_else(|| {
        anyhow!(
            "Disk path contains invalid Unicode: {}",
            disk_path.display()
        )
    })?;
    let snapshot_path_str = snapshot_path.to_str().ok_or_else(|| {
        anyhow!(
            "Snapshot path contains invalid Unicode: {}",
            snapshot_path.display()
        )
    })?;

    let cmd = Command::new("qemu-img")
        .args([
            "create",
            "-f",
            "qcow2",
            "-b",
            disk_path_str,
            "-F",
            "qcow2",
            snapshot_path_str,
        ])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to create snapshot: {}", err));
    }

    println!(
        "✓ Created snapshot: {} -> {}",
        snapshot_name,
        snapshot_path.display()
    );
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

        if path.is_file()
            && path.extension().map(|e| e == "qcow2").unwrap_or(false)
            && let Ok(info) = get_disk_info(&path)
        {
            disks.push(info);
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

    let disk_path_str = disk_path.to_str().ok_or_else(|| {
        anyhow!(
            "Disk path contains invalid Unicode: {}",
            disk_path.display()
        )
    })?;

    let size = format!("{}G", new_size_gb);
    let cmd = Command::new("qemu-img")
        .args(["resize", disk_path_str, &size])
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

    let source_str = source
        .to_str()
        .ok_or_else(|| anyhow!("Source path contains invalid Unicode: {}", source.display()))?;
    let dest_str = dest.to_str().ok_or_else(|| {
        anyhow!(
            "Destination path contains invalid Unicode: {}",
            dest.display()
        )
    })?;

    let cmd = Command::new("qemu-img")
        .args(["convert", "-f", "qcow2", "-O", format, source_str, dest_str])
        .output()
        .map_err(|e| anyhow!("Failed to run qemu-img: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to convert disk: {}", err));
    }

    println!("✓ Converted disk from qcow2 to {} format", format);
    Ok(())
}
