//! Process management for QEMU VMs
//!
//! Handles discovery, monitoring, and control of running QEMU processes.

use anyhow::{anyhow, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Command;
use crate::state;

/// Find QEMU processes by VM name
///
/// Uses PID-file-first strategy:
/// 1. Try reading the saved PID file (most reliable)
/// 2. Verify the PID is running and is a qemu-system process
/// 3. Fall back to exact command-line matching if PID file lookup fails
///
/// This approach avoids regex pattern ambiguities that can match partial VM names.
pub fn find_qemu_processes(vm_name: &str) -> Result<Vec<i32>> {
    // Strategy 1: Try to read saved PID file
    if let Ok(Some(pid)) = state::read_pid(vm_name) {
        if is_process_alive(pid)? && is_qemu_process(pid, Some(vm_name))? {
            return Ok(vec![pid]);
        }
        // PID file exists but process is not running or is not the right one
        // Fall through to process lookup below
    }

    // Strategy 2: Fall back to exact command-line matching
    // Find all qemu-system processes and check for exact -name match
    find_qemu_processes_by_exact_name(vm_name)
}

/// Check if a process is still alive
fn is_process_alive(pid: i32) -> Result<bool> {
    let output = Command::new("ps")
        .args(&["-p", &pid.to_string()])
        .output()
        .map_err(|e| anyhow!("Failed to run ps: {}", e))?;

    Ok(output.status.success())
}

/// Check if a process is a QEMU VM, optionally matching a specific VM name
fn is_qemu_process(pid: i32, vm_name: Option<&str>) -> Result<bool> {
    let cmd_line = get_process_cmdline(pid)?;
    
    if !cmd_line.contains("qemu-system") {
        return Ok(false);
    }

    if let Some(name) = vm_name {
        // Check for exact -name match to avoid partial matches
        return Ok(contains_exact_qemu_name_arg(&cmd_line, name));
    }

    Ok(true)
}

/// Find QEMU processes by exact command-line name matching
fn find_qemu_processes_by_exact_name(vm_name: &str) -> Result<Vec<i32>> {
    // Get all processes
    let output = Command::new("ps")
        .args(&["aux"])
        .output()
        .map_err(|e| anyhow!("Failed to run ps aux: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Invalid UTF-8 in ps output: {}", e))?;

    let mut pids = Vec::new();
    for line in stdout.lines() {
        // Skip header line
        if line.starts_with("USER") {
            continue;
        }

        // Parse process line: USER PID CPU MEM VSZ RSS TT STAT START TIME COMMAND
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 {
            continue;
        }

        // Extract PID (second column)
        if let Ok(pid) = parts[1].parse::<i32>() {
            // Extract command (everything from 10th column onward)
            let cmd = parts[10..].join(" ");
            
            // Check if this is a qemu-system process with exact -name match
            if cmd.contains("qemu-system") && contains_exact_qemu_name_arg(&cmd, vm_name) {
                pids.push(pid);
            }
        }
    }

    Ok(pids)
}

/// Check if a command line contains exact -name argument matching the VM name.
/// This avoids matching partial names (e.g., "test-vm" should not match "test-vm-backup").
fn contains_exact_qemu_name_arg(cmd_line: &str, vm_name: &str) -> bool {
    // Check for exact -name match with different terminators (space, quote, tab, end of string)
    // Pattern: space, then -name, then space, then the exact name, then terminator
    
    if cmd_line.contains(&format!(" -name {} ", vm_name)) {
        return true;
    }
    if cmd_line.contains(&format!(" -name {}\"", vm_name)) {
        return true;
    }
    if cmd_line.contains(&format!(" -name {}'", vm_name)) {
        return true;
    }
    if cmd_line.contains(&format!(" -name {}\t", vm_name)) {
        return true;
    }
    if cmd_line.ends_with(&format!(" -name {}", vm_name)) {
        return true;
    }

    false
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

    #[test]
    fn test_contains_exact_qemu_name_arg_exact_match() {
        // Exact match with space after
        assert!(contains_exact_qemu_name_arg(" -name test-vm ", "test-vm"));
        
        // Exact match at end of command
        assert!(contains_exact_qemu_name_arg(" -name test-vm", "test-vm"));
        
        // Exact match with quote
        assert!(contains_exact_qemu_name_arg(" -name test-vm\"", "test-vm"));
    }

    #[test]
    fn test_contains_exact_qemu_name_arg_rejects_partial_match() {
        // Should not match partial overlaps
        assert!(!contains_exact_qemu_name_arg(" -name test-vm-production ", "test-vm"));
        assert!(!contains_exact_qemu_name_arg(" -name test-vm-backup ", "test-vm"));
        
        // Should not match if name is substring but not exact
        assert!(!contains_exact_qemu_name_arg(" -name prefix-test-vm ", "test-vm"));
    }

    #[test]
    fn test_contains_exact_qemu_name_arg_overlapping_names() {
        // Two similar names should not cross-match
        let cmd_for_prod = " -name production-vm ";
        let cmd_for_test = " -name test-vm ";
        
        assert!(contains_exact_qemu_name_arg(cmd_for_prod, "production-vm"));
        assert!(!contains_exact_qemu_name_arg(cmd_for_prod, "test-vm"));
        
        assert!(contains_exact_qemu_name_arg(cmd_for_test, "test-vm"));
        assert!(!contains_exact_qemu_name_arg(cmd_for_test, "production-vm"));
    }

    #[test]
    fn test_contains_exact_qemu_name_arg_unsafe_characters() {
        // VM names with underscores, numbers, dashes should work
        assert!(contains_exact_qemu_name_arg(" -name my_vm_2024 ", "my_vm_2024"));
        assert!(contains_exact_qemu_name_arg(" -name vm-01 ", "vm-01"));
        
        // Should not match if unsafe chars differ
        assert!(!contains_exact_qemu_name_arg(" -name my_vm_2024 ", "my-vm-2024"));
    }

    #[test]
    fn test_full_command_line_with_multiple_spaces() {
        let cmd = "qemu-system-x86_64 -machine type=q35 -name test-vm -smp 4 -m 4096";
        assert!(contains_exact_qemu_name_arg(cmd, "test-vm"));
        assert!(!contains_exact_qemu_name_arg(cmd, "test"));
        assert!(!contains_exact_qemu_name_arg(cmd, "vm"));
    }
}