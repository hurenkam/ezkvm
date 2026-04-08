//! Process management for QEMU VMs
//!
//! Handles discovery, monitoring, and control of running QEMU processes.

use anyhow::{anyhow, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Command;

/// Find QEMU processes by VM name
pub fn find_qemu_processes(vm_name: &str) -> Result<Vec<i32>> {
    // Use pgrep to find processes with the specific name argument
    let output = Command::new("pgrep")
        .args(&["-f", &format!("qemu-system.*-name {}", vm_name)])
        .output()
        .map_err(|e| anyhow!("Failed to run pgrep: {}", e))?;

    if !output.status.success() {
        // pgrep returns non-zero when no processes found
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Invalid UTF-8 in pgrep output: {}", e))?;

    let mut pids = Vec::new();
    for line in stdout.lines() {
        if let Ok(pid) = line.trim().parse::<i32>() {
            pids.push(pid);
        }
    }

    Ok(pids)
}

/// Send signal to a process
pub fn signal_process(pid: i32, signal: Signal) -> Result<()> {
    kill(Pid::from_raw(pid), signal)
        .map_err(|e| anyhow!("Failed to send signal {:?} to process {}: {}", signal, pid, e))
}

/// Gracefully stop a VM by sending SIGTERM
pub fn stop_vm(vm_name: &str) -> Result<()> {
    let pids = find_qemu_processes(vm_name)?;

    if pids.is_empty() {
        return Err(anyhow!("No running VM found with name: {}", vm_name));
    }

    if pids.len() > 1 {
        eprintln!("Warning: Found {} processes for VM '{}', stopping all", pids.len(), vm_name);
    }

    for pid in pids {
        println!("Sending SIGTERM to process {}", pid);
        signal_process(pid, Signal::SIGTERM)?;
    }

    Ok(())
}

/// Force kill a VM by sending SIGKILL
pub fn kill_vm(vm_name: &str) -> Result<()> {
    let pids = find_qemu_processes(vm_name)?;

    if pids.is_empty() {
        return Err(anyhow!("No running VM found with name: {}", vm_name));
    }

    if pids.len() > 1 {
        eprintln!("Warning: Found {} processes for VM '{}', killing all", pids.len(), vm_name);
    }

    for pid in pids {
        println!("Sending SIGKILL to process {}", pid);
        signal_process(pid, Signal::SIGKILL)?;
    }

    Ok(())
}

/// List all running QEMU VMs
pub fn list_running_vms() -> Result<Vec<(String, i32)>> {
    // Use pgrep to find all qemu-system processes
    let output = Command::new("pgrep")
        .args(&["-f", "qemu-system"])
        .output()
        .map_err(|e| anyhow!("Failed to run pgrep: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Invalid UTF-8 in pgrep output: {}", e))?;

    let mut vms = Vec::new();
    for line in stdout.lines() {
        if let Ok(pid) = line.trim().parse::<i32>() {
            // Get the command line for this PID
            if let Ok(cmd_line) = get_process_cmdline(pid) {
                let vm_name = extract_vm_name_from_cmd(&cmd_line)
                    .unwrap_or_else(|| format!("qemu-{}", pid));
                vms.push((vm_name, pid));
            }
        }
    }

    Ok(vms)
}

/// Get command line for a process
fn get_process_cmdline(pid: i32) -> Result<String> {
    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "cmd="])
        .output()
        .map_err(|e| anyhow!("Failed to run ps: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!("ps command failed"));
    }

    let cmd = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Invalid UTF-8 in ps output: {}", e))?
        .trim()
        .to_string();

    Ok(cmd)
}

/// Check if a VM is running
pub fn is_vm_running(vm_name: &str) -> Result<bool> {
    let pids = find_qemu_processes(vm_name)?;
    Ok(!pids.is_empty())
}

/// Extract VM name from QEMU command line (heuristic)
fn extract_vm_name_from_cmd(cmd_line: &str) -> Option<String> {
    // Look for -name argument first (most reliable)
    if let Some(name_start) = cmd_line.find("-name ") {
        let name_part = &cmd_line[name_start + 6..]; // Skip "-name "
        if let Some(end) = name_part.find(' ') {
            return Some(name_part[..end].to_string());
        } else {
            // Name is at the end of the command line
            return Some(name_part.to_string());
        }
    }

    // Fallback: look for drive paths that might contain VM names
    if let Some(start) = cmd_line.find("-drive file=") {
        let drive_part = &cmd_line[start..];
        if let Some(end) = drive_part.find(",if=") {
            let path = &drive_part[12..end]; // Skip "-drive file="
            // Extract filename without extension
            if let Some(filename) = path.split('/').last() {
                if let Some(name) = filename.split('.').next() {
                    return Some(name.to_string());
                }
            }
        }
    }

    // Fallback: look for any recognizable name pattern
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vm_name_from_cmd() {
        let cmd = "qemu-system-x86_64 -name test-vm -drive file=/var/lib/ezkvm/ubuntu-22.04.qcow2,if=virtio,format=qcow2";
        assert_eq!(extract_vm_name_from_cmd(cmd), Some("test-vm".to_string()));

        let cmd2 = "qemu-system-x86_64 -machine type=q35";
        assert_eq!(extract_vm_name_from_cmd(cmd2), None);
    }
}