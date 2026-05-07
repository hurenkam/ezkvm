use super::find_qemu_processes;
use anyhow::{Result, anyhow};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

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

/// Gracefully stop a VM.
///
/// Preferred order:
/// 1) QMP `system_powerdown`
/// 2) QMP `quit`
/// 3) SIGTERM fallback
pub fn stop_vm(vm_name: &str, qmp_socket_path: Option<&str>) -> Result<()> {
    let pids = find_qemu_processes(vm_name)?;

    if pids.is_empty() {
        return Err(anyhow!("No running VM found with name: {}", vm_name));
    }

    if pids.len() > 1 {
        tracing::warn!(
            target: "ezkvm::process",
            vm = %vm_name,
            count = pids.len(),
            "multiple processes found for VM; stopping all"
        );
    }

    if let Some(socket_path) = qmp_socket_path {
        tracing::info!(target: "ezkvm::process", vm = %vm_name, socket = %socket_path, "requesting guest shutdown via QMP system_powerdown");
        if qmp_execute(socket_path, "system_powerdown").is_ok() {
            if wait_for_vm_exit(vm_name, Duration::from_secs(10))? {
                return Ok(());
            }

            tracing::info!(target: "ezkvm::process", vm = %vm_name, "VM still running after system_powerdown; requesting QMP quit");
            if qmp_execute(socket_path, "quit").is_ok()
                && wait_for_vm_exit(vm_name, Duration::from_secs(5))?
            {
                return Ok(());
            }
        } else {
            tracing::warn!(target: "ezkvm::process", vm = %vm_name, "QMP powerdown failed; falling back to SIGTERM");
        }
    }

    for pid in pids {
        tracing::info!(target: "ezkvm::process", vm = %vm_name, pid, "sending SIGTERM to process");
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
        tracing::warn!(
            target: "ezkvm::process",
            vm = %vm_name,
            count = pids.len(),
            "multiple processes found for VM; killing all"
        );
    }

    for pid in pids {
        tracing::info!(target: "ezkvm::process", vm = %vm_name, pid, "sending SIGKILL to process");
        signal_process(pid, Signal::SIGKILL)?;
    }

    Ok(())
}

/// Check if a VM is running
pub fn is_vm_running(vm_name: &str) -> Result<bool> {
    let pids = find_qemu_processes(vm_name)?;
    Ok(!pids.is_empty())
}

fn wait_for_vm_exit(vm_name: &str, timeout: Duration) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if find_qemu_processes(vm_name)?.is_empty() {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(find_qemu_processes(vm_name)?.is_empty())
}

fn qmp_execute(socket_path: &str, command: &str) -> Result<()> {
    let stream = UnixStream::connect(socket_path)
        .map_err(|e| anyhow!("Failed to connect to QMP socket {}: {}", socket_path, e))?;
    let reader_stream = stream
        .try_clone()
        .map_err(|e| anyhow!("Failed to clone QMP socket stream: {}", e))?;

    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    let greeting = read_qmp_message(&mut reader)?;
    if greeting.get("QMP").is_none() {
        return Err(anyhow!("Invalid QMP greeting: {}", greeting));
    }

    write_qmp_message(&mut writer, json!({ "execute": "qmp_capabilities" }))?;
    read_qmp_response(&mut reader)?;

    write_qmp_message(&mut writer, json!({ "execute": command }))?;
    read_qmp_response(&mut reader)?;

    Ok(())
}

fn write_qmp_message(writer: &mut UnixStream, msg: Value) -> Result<()> {
    writer
        .write_all(format!("{}\n", msg).as_bytes())
        .map_err(|e| anyhow!("Failed to write QMP command: {}", e))?;
    writer
        .flush()
        .map_err(|e| anyhow!("Failed to flush QMP command: {}", e))
}

fn read_qmp_message(reader: &mut BufReader<UnixStream>) -> Result<Value> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| anyhow!("Failed to read QMP message: {}", e))?;
        if bytes == 0 {
            return Err(anyhow!("QMP socket closed unexpectedly"));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return serde_json::from_str(trimmed)
            .map_err(|e| anyhow!("Invalid QMP JSON '{}': {}", trimmed, e));
    }
}

fn read_qmp_response(reader: &mut BufReader<UnixStream>) -> Result<()> {
    loop {
        let msg = read_qmp_message(reader)?;
        if msg.get("return").is_some() {
            return Ok(());
        }
        if let Some(err) = msg.get("error") {
            return Err(anyhow!("QMP command failed: {}", err));
        }
        // Ignore asynchronous events while waiting for the command response.
    }
}
