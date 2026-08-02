use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

use thiserror::Error;

use super::{
    host_config::{HostConfig, InvalidVmNameError},
    process,
    qmp::{QmpClient, QmpError},
    vm_handle::{VmHandle, VmHandleError},
};

const POLL_INTERVAL: Duration = Duration::from_millis(200);
const PROCESS_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error(transparent)]
    InvalidVmName(#[from] InvalidVmNameError),
    #[error(transparent)]
    VmHandle(#[from] VmHandleError),
    #[error(transparent)]
    Qmp(#[from] QmpError),
    #[error("VM '{vm_name}' is not running")]
    NotRunning { vm_name: String },
    #[error("failed to terminate process {pid}: {source}")]
    TerminateProcess {
        pid: u32,
        #[source]
        source: std::io::Error,
    },
    #[error("timed out waiting for process {pid} to exit")]
    TimedOut { pid: u32 },
}

pub fn stop(host_config: &HostConfig, vm_name: &str) -> Result<(), LifecycleError> {
    let (vm_name, handle) = load_live_handle(host_config, vm_name)?;
    let qemu_pid = *handle.qemu_pid();
    let ui_client_pid = *handle.ui_client_pid();
    let swtpm_pid = *handle.swtpm_pid();
    let qmp_socket_path = handle.qmp_socket_path().to_string();
    let tpm_socket_path = handle.tpm_socket_path().clone();

    let mut client = QmpClient::connect(&qmp_socket_path)?;
    client.system_powerdown()?;

    let escalation_deadline = host_config
        .stop_escalation_timeout_secs()
        .map(|secs| Instant::now() + Duration::from_secs(secs));
    let mut escalated = false;

    loop {
        if !process::is_pid_alive(qemu_pid) {
            break;
        }

        if !escalated && escalation_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            if process::is_pid_alive(qemu_pid) {
                match client.quit() {
                    Ok(()) => {}
                    Err(_error) if !process::is_pid_alive(qemu_pid) => break,
                    Err(error) => return Err(error.into()),
                }
            }
            escalated = true;
        }

        thread::sleep(POLL_INTERVAL);
    }

    cleanup_after_exit(
        host_config.state_dir(),
        &vm_name,
        ui_client_pid,
        swtpm_pid,
        &qmp_socket_path,
        tpm_socket_path.as_deref(),
    )
}

pub(crate) fn load_live_handle(
    host_config: &HostConfig,
    vm_name: &str,
) -> Result<(String, VmHandle), LifecycleError> {
    let vm_name = HostConfig::resolve_vm_name(vm_name)?.to_string();
    let Some(handle) = VmHandle::read(host_config.state_dir(), &vm_name)? else {
        return Err(LifecycleError::NotRunning { vm_name });
    };

    if handle.is_stale() {
        VmHandle::remove(host_config.state_dir(), &vm_name)?;
        return Err(LifecycleError::NotRunning { vm_name });
    }

    Ok((vm_name, handle))
}

pub(crate) fn cleanup_after_exit(
    state_dir: &Path,
    vm_name: &str,
    ui_client_pid: Option<u32>,
    swtpm_pid: Option<u32>,
    qmp_socket_path: &str,
    tpm_socket_path: Option<&str>,
) -> Result<(), LifecycleError> {
    terminate_tracked_process(ui_client_pid)?;
    terminate_tracked_process(swtpm_pid)?;
    remove_socket_path(qmp_socket_path)?;
    if let Some(path) = tpm_socket_path {
        remove_socket_path(path)?;
    }
    VmHandle::remove(state_dir, vm_name)?;
    Ok(())
}

fn terminate_tracked_process(pid: Option<u32>) -> Result<(), LifecycleError> {
    if let Some(pid) = pid.filter(|pid| process::is_pid_alive(*pid)) {
        terminate_process(pid, true)?;
        if !wait_for_pid_exit(pid, Some(PROCESS_TERMINATION_TIMEOUT)) {
            return Err(LifecycleError::TimedOut { pid });
        }
    }
    Ok(())
}

fn remove_socket_path(path: &str) -> Result<(), LifecycleError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LifecycleError::TerminateProcess { pid: 0, source }),
    }
}

pub(crate) fn terminate_process(pid: u32, force: bool) -> Result<(), LifecycleError> {
    match process::terminate_pid(pid, force) {
        Ok(()) => Ok(()),
        Err(source) if source.raw_os_error() == Some(libc::ESRCH) => Ok(()),
        Err(source) => Err(LifecycleError::TerminateProcess { pid, source }),
    }
}

pub(crate) fn wait_for_pid_exit(pid: u32, timeout: Option<Duration>) -> bool {
    let deadline = timeout.map(|duration| Instant::now() + duration);
    loop {
        if !process::is_pid_alive(pid) {
            return true;
        }

        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            return false;
        }

        thread::sleep(POLL_INTERVAL);
    }
}
