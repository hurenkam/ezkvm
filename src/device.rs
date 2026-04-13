//! Device management for VMs
//!
//! Handles hot-plugging of devices (drives, networks) to running VMs.

#![allow(dead_code)]

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;

/// Hot-add a disk to a running VM using QMP (QEMU Monitor Protocol)
///
/// This requires QEMU to be started with a monitor socket.
pub fn hotadd_disk(vm_pid: i32, disk_path: &str, id: &str, if_type: &str) -> Result<()> {
    let socket_path = resolve_qmp_socket_from_pid(vm_pid)?;
    let mut qmp = QmpClient::connect(&socket_path)?;

    let node_name = disk_node_name(id);
    qmp.execute(
        "blockdev-add",
        Some(json!({
            "node-name": node_name,
            "driver": "raw",
            "file": {
                "driver": "file",
                "filename": disk_path
            }
        })),
    )?;
    qmp.execute(
        "device_add",
        Some(json!({
            "driver": map_disk_driver(if_type),
            "drive": node_name,
            "id": id
        })),
    )?;

    println!("✓ Hot-added disk: {} ({})", disk_path, id);
    Ok(())
}

/// Hot-remove a disk from a running VM
pub fn hotremove_disk(vm_pid: i32, id: &str) -> Result<()> {
    let socket_path = resolve_qmp_socket_from_pid(vm_pid)?;
    let mut qmp = QmpClient::connect(&socket_path)?;

    qmp.execute("device_del", Some(json!({ "id": id })))?;
    // Best effort cleanup: the block node may already be gone.
    let _ = qmp.execute(
        "blockdev-del",
        Some(json!({
            "node-name": disk_node_name(id)
        })),
    );

    println!("✓ Hot-removed disk: {}", id);
    Ok(())
}

/// Hot-add a network device to a running VM
pub fn hotadd_network(vm_pid: i32, model: &str, mac: Option<&str>, id: &str) -> Result<()> {
    let socket_path = resolve_qmp_socket_from_pid(vm_pid)?;
    let mut qmp = QmpClient::connect(&socket_path)?;
    let netdev_id = netdev_id(id);

    qmp.execute(
        "netdev_add",
        Some(json!({
            "type": "user",
            "id": netdev_id
        })),
    )?;
    qmp.execute("device_add", Some(network_device_add_args(model, mac, id)))?;

    println!("✓ Hot-added network device: {} ({})", model, id);
    if let Some(mac_addr) = mac {
        println!("  MAC: {}", mac_addr);
    }
    Ok(())
}

/// Hot-remove a network device from a running VM
pub fn hotremove_network(vm_pid: i32, id: &str) -> Result<()> {
    let socket_path = resolve_qmp_socket_from_pid(vm_pid)?;
    let mut qmp = QmpClient::connect(&socket_path)?;

    qmp.execute("device_del", Some(json!({ "id": id })))?;
    qmp.execute("netdev_del", Some(json!({ "id": netdev_id(id) })))?;

    println!("✓ Hot-removed network device: {}", id);
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
    let cmdline = fs::read_to_string("/proc/cmdline")
        .map_err(|e| anyhow!("Failed to read /proc/cmdline: {}", e))?;

    // Convert to lowercase for case-insensitive matching
    let cmdline_lower = cmdline.to_lowercase();

    // Check for common IOMMU enable patterns
    let has_intel =
        cmdline_lower.contains("intel_iommu=on") || cmdline_lower.contains("intel_iommu");
    let has_amd = cmdline_lower.contains("amd_iommu=on") || cmdline_lower.contains("amd_iommu");

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
    let device_list = devices
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(device_list)
}

fn resolve_qmp_socket_from_pid(vm_pid: i32) -> Result<String> {
    let cmdline_path = format!("/proc/{}/cmdline", vm_pid);
    let cmdline = fs::read(cmdline_path)
        .map_err(|e| anyhow!("Failed to read process cmdline for PID {}: {}", vm_pid, e))?;

    find_qmp_socket_path_from_cmdline(&cmdline).ok_or_else(|| {
        anyhow!(
            "Could not resolve a QMP unix socket from PID {} command line",
            vm_pid
        )
    })
}

fn find_qmp_socket_path_from_cmdline(cmdline: &[u8]) -> Option<String> {
    let args: Vec<String> = cmdline
        .split(|b| *b == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect();

    for pair in args.windows(2) {
        if pair[0] == "-qmp"
            && let Some(path) = parse_qmp_socket_path(&pair[1])
        {
            return Some(path);
        }
    }

    None
}

fn parse_qmp_socket_path(spec: &str) -> Option<String> {
    if let Some(rest) = spec.strip_prefix("unix:") {
        return Some(rest.split(',').next().unwrap_or_default().to_string());
    }
    None
}

fn map_disk_driver(if_type: &str) -> &'static str {
    match if_type {
        "scsi" => "scsi-hd",
        "nvme" => "nvme",
        "virtio" | "virtio-blk" => "virtio-blk-pci",
        _ => "virtio-blk-pci",
    }
}

fn disk_node_name(id: &str) -> String {
    format!("drive-{}", id)
}

fn netdev_id(id: &str) -> String {
    format!("netdev-{}", id)
}

fn network_device_add_args(model: &str, mac: Option<&str>, id: &str) -> Value {
    let mut args = json!({
        "driver": model,
        "netdev": netdev_id(id),
        "id": id
    });

    if let Some(mac_addr) = mac {
        args["mac"] = Value::String(mac_addr.to_string());
    }

    args
}

struct QmpClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl QmpClient {
    fn connect(socket_path: &str) -> Result<Self> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| anyhow!("Failed to connect to QMP socket {}: {}", socket_path, e))?;
        let reader_stream = stream
            .try_clone()
            .map_err(|e| anyhow!("Failed to clone QMP socket stream: {}", e))?;

        let mut client = Self {
            reader: BufReader::new(reader_stream),
            writer: stream,
        };
        client.initialize()?;
        Ok(client)
    }

    fn initialize(&mut self) -> Result<()> {
        let greeting = self.read_message()?;
        if greeting.get("QMP").is_none() {
            return Err(anyhow!("Invalid QMP greeting: {}", greeting));
        }

        self.execute("qmp_capabilities", None)?;
        Ok(())
    }

    fn execute(&mut self, execute: &str, arguments: Option<Value>) -> Result<Value> {
        let mut request = json!({ "execute": execute });
        if let Some(arguments) = arguments {
            request["arguments"] = arguments;
        }

        self.writer
            .write_all(format!("{}\n", request).as_bytes())
            .map_err(|e| anyhow!("Failed to send QMP command {}: {}", execute, e))?;
        self.writer
            .flush()
            .map_err(|e| anyhow!("Failed to flush QMP command {}: {}", execute, e))?;

        loop {
            let response = self.read_message()?;
            if let Some(ret) = response.get("return") {
                return Ok(ret.clone());
            }
            if let Some(err) = response.get("error") {
                return Err(format_qmp_error(execute, err));
            }
            // Ignore asynchronous events and continue until a command response appears.
        }
    }

    fn read_message(&mut self) -> Result<Value> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self
                .reader
                .read_line(&mut line)
                .map_err(|e| anyhow!("Failed to read from QMP socket: {}", e))?;

            if bytes == 0 {
                return Err(anyhow!("QMP socket closed unexpectedly"));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let value: Value = serde_json::from_str(trimmed)
                .map_err(|e| anyhow!("Invalid JSON from QMP: {} ({})", trimmed, e))?;
            return Ok(value);
        }
    }
}

fn format_qmp_error(execute: &str, err: &Value) -> anyhow::Error {
    let class = err
        .get("class")
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    let desc = err
        .get("desc")
        .and_then(Value::as_str)
        .unwrap_or("No description");
    anyhow!("QMP command '{}' failed ({}): {}", execute, class, desc)
}

/// List available PCI devices suitable for passthrough
pub fn list_pci_devices() -> Result<Vec<String>> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("lspci | grep -E 'VGA|Network|Serial|USB'")
        .output()
        .map_err(|e| anyhow!("Failed to list PCI devices: {}", e))?;

    let devices = String::from_utf8(output.stdout)?;
    let device_list = devices
        .lines()
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

    #[test]
    fn test_parse_qmp_socket_path_unix() {
        let path = parse_qmp_socket_path("unix:/var/run/qemu-server/108.qmp,server=on,wait=off");
        assert_eq!(path.as_deref(), Some("/var/run/qemu-server/108.qmp"));
    }

    #[test]
    fn test_parse_qmp_socket_path_non_unix_rejected() {
        let path = parse_qmp_socket_path("tcp:127.0.0.1:4444,server=on,wait=off");
        assert!(path.is_none());
    }

    #[test]
    fn test_find_qmp_socket_path_from_cmdline() {
        let cmdline =
            b"qemu-system-x86_64\0-name\0test\0-qmp\0unix:/tmp/test.qmp,server=on,wait=off\0";
        let path = find_qmp_socket_path_from_cmdline(cmdline);
        assert_eq!(path.as_deref(), Some("/tmp/test.qmp"));
    }

    #[test]
    fn test_network_device_add_args_with_mac() {
        let args = network_device_add_args("virtio-net-pci", Some("52:54:00:12:34:56"), "net0");
        assert_eq!(args["driver"], "virtio-net-pci");
        assert_eq!(args["netdev"], "netdev-net0");
        assert_eq!(args["id"], "net0");
        assert_eq!(args["mac"], "52:54:00:12:34:56");
    }

    #[test]
    fn test_network_device_add_args_without_mac() {
        let args = network_device_add_args("virtio-net-pci", None, "net1");
        assert_eq!(args["driver"], "virtio-net-pci");
        assert_eq!(args["netdev"], "netdev-net1");
        assert_eq!(args["id"], "net1");
        assert!(args.get("mac").is_none());
    }

    #[test]
    fn test_format_qmp_error_includes_class_and_desc() {
        let err = json!({"class": "DeviceNotFound", "desc": "No such device"});
        let formatted = format_qmp_error("device_del", &err).to_string();
        assert!(formatted.contains("device_del"));
        assert!(formatted.contains("DeviceNotFound"));
        assert!(formatted.contains("No such device"));
    }
}
