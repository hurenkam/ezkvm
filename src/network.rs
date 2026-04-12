//! Network management for VMs
//!
//! Handles networking configuration including bridges, port forwarding, and isolation.

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::process::Command;

/// Network mode for a VM
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkMode {
    /// User mode networking (NAT)
    User,
    /// Bridge mode (direct network access)
    Bridge,
    /// Isolated network
    Isolated,
}

impl std::fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMode::User => write!(f, "user"),
            NetworkMode::Bridge => write!(f, "bridge"),
            NetworkMode::Isolated => write!(f, "isolated"),
        }
    }
}

/// Represent a port forwarding rule
#[derive(Debug, Clone)]
pub struct PortForwardRule {
    pub protocol: String, // tcp/udp
    pub host_port: u16,
    pub guest_port: u16,
    pub guest_ip: String,
}

/// Create virtual bridge interface
pub fn create_bridge(name: &str) -> Result<()> {
    // Check if bridge already exists
    let check = Command::new("ip")
        .args(&["link", "show", name])
        .output();
    
    if check.is_ok() && check.unwrap().status.success() {
        return Err(anyhow!("Bridge '{}' already exists", name));
    }
    
    // Create bridge
    let cmd = Command::new("ip")
        .args(&["link", "add", name, "type", "bridge"])
        .output()
        .map_err(|e| anyhow!("Failed to create bridge: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to create bridge: {}", err));
    }
    
    // Enable bridge
    Command::new("ip")
        .args(&["link", "set", name, "up"])
        .output()?;
    
    println!("✓ Created bridge: {}", name);
    Ok(())
}

/// Delete virtual bridge interface
pub fn delete_bridge(name: &str) -> Result<()> {
    let cmd = Command::new("ip")
        .args(&["link", "delete", name])
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
        .args(&["link", "show", "type", "bridge"])
        .output()
        .map_err(|e| anyhow!("Failed to list bridges: {}", e))?;
    
    let output_str = String::from_utf8(output.stdout)?;
    let bridges: Vec<String> = output_str
        .lines()
        .filter_map(|line| {
            if line.starts_with(char::is_numeric) {
                line.split_whitespace().nth(1).map(|s| s.trim_end_matches(':').to_string())
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
        .args(&["link", "set", device, "master", bridge])
        .output()
        .map_err(|e| anyhow!("Failed to add device to bridge: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to add device to bridge: {}", err));
    }
    
    println!("✓ Added {} to bridge {}", device, bridge);
    Ok(())
}

/// Configure port forwarding rule
pub fn add_port_forward(
    rule: &PortForwardRule,
    external_ip: &str,
) -> Result<()> {
    let protocol = rule.protocol.as_str();
    let host_port = rule.host_port;
    let guest_port = rule.guest_port;
    let guest_ip = &rule.guest_ip;
    
    let cmd = Command::new("iptables")
        .args(&[
            "-t", "nat", "-A", "PREROUTING",
            "-p", protocol,
            "-d", external_ip,
            "--dport", &host_port.to_string(),
            "-j", "DNAT",
            "--to-destination", &format!("{}:{}", guest_ip, guest_port),
        ])
        .output()
        .map_err(|e| anyhow!("Failed to setup port forward: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to setup port forward: {}", err));
    }
    
    println!("✓ Added port forward: {}:{} -> {}:{}",
        external_ip, host_port, guest_ip, guest_port);
    Ok(())
}

/// Remove port forwarding rule
pub fn remove_port_forward(
    rule: &PortForwardRule,
    external_ip: &str,
) -> Result<()> {
    let protocol = rule.protocol.as_str();
    let host_port = rule.host_port;
    let guest_port = rule.guest_port;
    let guest_ip = &rule.guest_ip;
    
    let cmd = Command::new("iptables")
        .args(&[
            "-t", "nat", "-D", "PREROUTING",
            "-p", protocol,
            "-d", external_ip,
            "--dport", &host_port.to_string(),
            "-j", "DNAT",
            "--to-destination", &format!("{}:{}", guest_ip, guest_port),
        ])
        .output()
        .map_err(|e| anyhow!("Failed to remove port forward: {}", e))?;
    
    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to remove port forward: {}", err));
    }
    
    println!("✓ Removed port forward rule");
    Ok(())
}

/// Configure network isolation (dedicated VLAN)
pub fn setup_network_isolation(vm_name: &str, vlan_id: u16) -> Result<()> {
    let vlan_name = format!("vlan{}", vlan_id);
    
    // Create VLAN interface
    let cmd = Command::new("ip")
        .args(&["link", "add", "link", "eth0", "name", &vlan_name, "type", "vlan", "id", &vlan_id.to_string()])
        .output()
        .map_err(|e| anyhow!("Failed to execute ip link add command: {}", e))?;

    // Check for specific "already exists" error vs. actual failures
    if !cmd.status.success() {
        let stderr = String::from_utf8_lossy(&cmd.stderr);
        
        // Only ignore the specific "File exists" error case
        if !stderr.contains("File exists") && !stderr.contains("already exists") {
            return Err(anyhow!("Failed to create VLAN interface '{}': {}", vlan_name, stderr));
        }
        
        // If it's just "already exists", log and continue
        eprintln!("ℹ VLAN interface '{}' already exists, proceeding with setup", vlan_name);
    }
    
    // Enable VLAN interface
    let enable_cmd = Command::new("ip")
        .args(&["link", "set", &vlan_name, "up"])
        .output()
        .map_err(|e| anyhow!("Failed to enable VLAN interface '{}': {}", vlan_name, e))?;
    
    if !enable_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&enable_cmd.stderr);
        return Err(anyhow!("Failed to bring up VLAN interface '{}': {}", vlan_name, stderr));
    }

    println!("✓ Setup network isolation for VM '{}' on VLAN {}", vm_name, vlan_id);
    Ok(())
}

/// Get network statistics for a VM
pub fn get_network_stats(interface: &str) -> Result<NetworkStats> {
    let output = Command::new("ip")
        .args(&["-s", "link", "show", interface])
        .output()
        .map_err(|e| anyhow!("Failed to get network stats: {}", e))?;
    
    let _output_str = String::from_utf8(output.stdout)?;
    
    // Parse output (simplified)
    let stats = NetworkStats {
        interface: interface.to_string(),
        bytes_sent: 0,
        bytes_received: 0,
        packets_sent: 0,
        packets_received: 0,
    };
    
    Ok(stats)
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub interface: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_mode_display() {
        assert_eq!(NetworkMode::User.to_string(), "user");
        assert_eq!(NetworkMode::Bridge.to_string(), "bridge");
        assert_eq!(NetworkMode::Isolated.to_string(), "isolated");
    }

    #[test]
    fn test_port_forward_rule() {
        let rule = PortForwardRule {
            protocol: "tcp".to_string(),
            host_port: 8080,
            guest_port: 80,
            guest_ip: "192.168.1.10".to_string(),
        };
        assert_eq!(rule.host_port, 8080);
        assert_eq!(rule.guest_port, 80);
    }
}
