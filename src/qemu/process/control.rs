use super::find_qemu_processes;
use anyhow::{Result, anyhow};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

/// Send signal to a process
pub(super) fn signal_process(pid: i32, signal: Signal) -> Result<()> {
    kill(Pid::from_raw(pid), signal).map_err(|e| {
        anyhow!(
            "Failed to send signal {:?} to process {}: {}",
            signal,
            pid,
            e
        )
    })
}

/// Gracefully stop a VM by sending SIGTERM
pub fn stop_vm(vm_name: &str) -> Result<()> {
    let pids = find_qemu_processes(vm_name)?;

    if pids.is_empty() {
        return Err(anyhow!("No running VM found with name: {}", vm_name));
    }

    if pids.len() > 1 {
        eprintln!(
            "Warning: Found {} processes for VM '{}', stopping all",
            pids.len(),
            vm_name
        );
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
        eprintln!(
            "Warning: Found {} processes for VM '{}', killing all",
            pids.len(),
            vm_name
        );
    }

    for pid in pids {
        println!("Sending SIGKILL to process {}", pid);
        signal_process(pid, Signal::SIGKILL)?;
    }

    Ok(())
}

/// Check if a VM is running
pub fn is_vm_running(vm_name: &str) -> Result<bool> {
    let pids = find_qemu_processes(vm_name)?;
    Ok(!pids.is_empty())
}
