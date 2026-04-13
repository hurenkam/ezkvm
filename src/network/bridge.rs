use anyhow::{Result, anyhow};
use std::process::Command;

/// Create virtual bridge interface
pub fn create_bridge(name: &str) -> Result<()> {
    // Check if bridge already exists
    let check = Command::new("ip").args(["link", "show", name]).output();

    if check.is_ok() && check.as_ref().is_ok_and(|output| output.status.success()) {
        return Err(anyhow!("Bridge '{}' already exists", name));
    }

    // Create bridge
    let cmd = Command::new("ip")
        .args(["link", "add", name, "type", "bridge"])
        .output()
        .map_err(|e| anyhow!("Failed to create bridge: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to create bridge: {}", err));
    }

    // Enable bridge
    Command::new("ip")
        .args(["link", "set", name, "up"])
        .output()?;

    println!("✓ Created bridge: {}", name);
    Ok(())
}

/// Delete virtual bridge interface
pub fn delete_bridge(name: &str) -> Result<()> {
    let cmd = Command::new("ip")
        .args(["link", "delete", name])
        .output()
        .map_err(|e| anyhow!("Failed to delete bridge: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to delete bridge: {}", err));
    }

    println!("✓ Deleted bridge: {}", name);
    Ok(())
}

/// List all available network bridges
pub fn list_bridges() -> Result<Vec<String>> {
    let output = Command::new("ip")
        .args(["link", "show", "type", "bridge"])
        .output()
        .map_err(|e| anyhow!("Failed to list bridges: {}", e))?;

    let output_str = String::from_utf8(output.stdout)?;
    let bridges: Vec<String> = output_str
        .lines()
        .filter_map(|line| {
            if line.starts_with(char::is_numeric) {
                line.split_whitespace()
                    .nth(1)
                    .map(|s| s.trim_end_matches(':').to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(bridges)
}

/// Add a device to a bridge
pub fn add_to_bridge(bridge: &str, device: &str) -> Result<()> {
    let cmd = Command::new("ip")
        .args(["link", "set", device, "master", bridge])
        .output()
        .map_err(|e| anyhow!("Failed to add device to bridge: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to add device to bridge: {}", err));
    }

    println!("✓ Added {} to bridge {}", device, bridge);
    Ok(())
}
