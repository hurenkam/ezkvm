use anyhow::{Result, anyhow};
use std::process::Command;

use super::PortForwardRule;

/// Configure port forwarding rule
pub fn add_port_forward(rule: &PortForwardRule, external_ip: &str) -> Result<()> {
    let protocol = rule.protocol.as_str();
    let host_port = rule.host_port;
    let guest_port = rule.guest_port;
    let guest_ip = &rule.guest_ip;

    let cmd = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-A",
            "PREROUTING",
            "-p",
            protocol,
            "-d",
            external_ip,
            "--dport",
            &host_port.to_string(),
            "-j",
            "DNAT",
            "--to-destination",
            &format!("{}:{}", guest_ip, guest_port),
        ])
        .output()
        .map_err(|e| anyhow!("Failed to setup port forward: {}", e))?;

    if !cmd.status.success() {
        let err = String::from_utf8_lossy(&cmd.stderr);
        return Err(anyhow!("Failed to setup port forward: {}", err));
    }

    println!(
        "✓ Added port forward: {}:{} -> {}:{}",
        external_ip, host_port, guest_ip, guest_port
    );
    Ok(())
}

/// Remove port forwarding rule
pub fn remove_port_forward(rule: &PortForwardRule, external_ip: &str) -> Result<()> {
    let protocol = rule.protocol.as_str();
    let host_port = rule.host_port;
    let guest_port = rule.guest_port;
    let guest_ip = &rule.guest_ip;

    let cmd = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-D",
            "PREROUTING",
            "-p",
            protocol,
            "-d",
            external_ip,
            "--dport",
            &host_port.to_string(),
            "-j",
            "DNAT",
            "--to-destination",
            &format!("{}:{}", guest_ip, guest_port),
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
        .args([
            "link",
            "add",
            "link",
            "eth0",
            "name",
            &vlan_name,
            "type",
            "vlan",
            "id",
            &vlan_id.to_string(),
        ])
        .output()
        .map_err(|e| anyhow!("Failed to execute ip link add command: {}", e))?;

    // Check for specific "already exists" error vs. actual failures
    if !cmd.status.success() {
        let stderr = String::from_utf8_lossy(&cmd.stderr);

        // Only ignore the specific "File exists" error case
        if !stderr.contains("File exists") && !stderr.contains("already exists") {
            return Err(anyhow!(
                "Failed to create VLAN interface '{}': {}",
                vlan_name,
                stderr
            ));
        }

        // If it's just "already exists", log and continue
        eprintln!(
            "ℹ VLAN interface '{}' already exists, proceeding with setup",
            vlan_name
        );
    }

    // Enable VLAN interface
    let enable_cmd = Command::new("ip")
        .args(["link", "set", &vlan_name, "up"])
        .output()
        .map_err(|e| anyhow!("Failed to enable VLAN interface '{}': {}", vlan_name, e))?;

    if !enable_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&enable_cmd.stderr);
        return Err(anyhow!(
            "Failed to bring up VLAN interface '{}': {}",
            vlan_name,
            stderr
        ));
    }

    println!(
        "✓ Setup network isolation for VM '{}' on VLAN {}",
        vm_name, vlan_id
    );
    Ok(())
}
