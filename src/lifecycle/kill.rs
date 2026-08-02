use std::time::Duration;

use super::{
    host_config::HostConfig,
    qmp::QmpClient,
    stop::{
        LifecycleError, cleanup_after_exit, load_live_handle, terminate_process, wait_for_pid_exit,
    },
};

const QUIT_GRACE_PERIOD: Duration = Duration::from_secs(1);
const TERM_GRACE_PERIOD: Duration = Duration::from_secs(2);
const FORCE_KILL_GRACE_PERIOD: Duration = Duration::from_secs(2);

pub fn kill(host_config: &HostConfig, vm_name: &str) -> Result<(), LifecycleError> {
    let (vm_name, handle) = load_live_handle(host_config, vm_name)?;
    let qemu_pid = *handle.qemu_pid();
    let ui_client_pid = *handle.ui_client_pid();
    let swtpm_pid = *handle.swtpm_pid();
    let qmp_socket_path = handle.qmp_socket_path().to_string();
    let tpm_socket_path = handle.tpm_socket_path().clone();

    // VMGR-04: prefer QMP `quit`; fall back to SIGTERM (not SIGKILL) when the QMP
    // connection itself fails, or when `quit` did not result in exit in time.
    // A SIGKILL is only used as a last-resort safety net after SIGTERM also fails.
    let quit_acknowledged = match QmpClient::connect(&qmp_socket_path) {
        Ok(mut client) => client.quit().is_ok(),
        Err(_) => false,
    };

    let exited = quit_acknowledged && wait_for_pid_exit(qemu_pid, Some(QUIT_GRACE_PERIOD));

    if !exited {
        terminate_process(qemu_pid, false)?; // SIGTERM fallback (VMGR-04)
        if !wait_for_pid_exit(qemu_pid, Some(TERM_GRACE_PERIOD)) {
            terminate_process(qemu_pid, true)?; // SIGKILL: last-resort safety net
            if !wait_for_pid_exit(qemu_pid, Some(FORCE_KILL_GRACE_PERIOD)) {
                return Err(LifecycleError::TimedOut { pid: qemu_pid });
            }
        }
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
