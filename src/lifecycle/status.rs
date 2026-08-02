use std::path::Path;

use super::vm_handle::{VmHandle, VmHandleError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusReport {
    Running {
        qemu_pid: u32,
        swtpm_pid: Option<u32>,
        ui_client_pid: Option<u32>,
    },
    NotRunning,
}

pub fn status(state_dir: &Path, vm_name: &str) -> Result<StatusReport, VmHandleError> {
    match VmHandle::read(state_dir, vm_name)? {
        Some(handle) if handle.is_stale() => {
            VmHandle::remove(state_dir, vm_name)?;
            Ok(StatusReport::NotRunning)
        }
        Some(handle) => Ok(StatusReport::Running {
            qemu_pid: *handle.qemu_pid(),
            swtpm_pid: *handle.swtpm_pid(),
            ui_client_pid: *handle.ui_client_pid(),
        }),
        None => Ok(StatusReport::NotRunning),
    }
}
