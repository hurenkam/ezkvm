use std::{path::PathBuf, str::FromStr, time::Duration};

use thiserror::Error;

use crate::{
    config::{
        qemu::{QemuCommandLine, QemuContext, QemuConversionError},
        EzkvmConfigSchema,
    },
    lifecycle::{
        host_config::{HostConfig, InvalidVmNameError},
        process, readiness,
        ui_client::resolve_ui_client,
        vm_handle::{VmHandle, VmHandleError},
    },
    runtime::{RootDeviceKind, Runtime},
};

#[derive(Debug, Error)]
pub enum StartError {
    #[error(transparent)]
    InvalidVmName(#[from] InvalidVmNameError),
    #[error(transparent)]
    VmHandle(#[from] VmHandleError),
    #[error("failed to create state directory {path}: {source}")]
    StateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read VM config {path}: {source}")]
    ReadVmConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse VM config {path}: {message}")]
    ParseVmConfig { path: PathBuf, message: String },
    #[error("failed to convert VM config {path}: {source}")]
    RuntimeConversion {
        path: PathBuf,
        source: crate::config::ezkvm::runtime::YamlRuntimeError,
    },
    #[error("failed to render qemu commandline: {0}")]
    Qemu(#[from] QemuConversionError),
    #[error("VM '{vm_name}' is already running with qemu PID {pid}; stop it first")]
    AlreadyRunning { vm_name: String, pid: u32 },
    #[error("failed to spawn swtpm at {program}: {source}")]
    SwtpmSpawnFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("swtpm socket at {path} did not become ready within {timeout_secs}s")]
    SwtpmNotReady { path: String, timeout_secs: u64 },
    #[error("failed to spawn qemu at {program}: {source}")]
    QemuSpawnFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

pub fn start(host_config: &HostConfig, vm_name: &str) -> Result<(), StartError> {
    let vm_name = HostConfig::resolve_vm_name(vm_name)?;
    process::ensure_dir_secure(host_config.state_dir()).map_err(|source| StartError::StateDir {
        path: host_config.state_dir().to_path_buf(),
        source,
    })?;

    if let Some(handle) = VmHandle::read(host_config.state_dir(), vm_name)? {
        if handle.is_stale() {
            VmHandle::remove(host_config.state_dir(), vm_name)?;
        } else {
            return Err(StartError::AlreadyRunning {
                vm_name: vm_name.to_string(),
                pid: *handle.qemu_pid(),
            });
        }
    }

    let vm_path = host_config.vm_yaml_path(vm_name)?;
    let vm_yaml = std::fs::read_to_string(&vm_path).map_err(|source| StartError::ReadVmConfig {
        path: vm_path.clone(),
        source,
    })?;
    let config = EzkvmConfigSchema::from_str(&vm_yaml).map_err(|source| StartError::ParseVmConfig {
        path: vm_path.clone(),
        message: source,
    })?;
    let display = config.host().display().clone();
    let runtime = Runtime::try_from(config).map_err(|source| StartError::RuntimeConversion {
        path: vm_path.clone(),
        source,
    })?;

    let qmp_socket_path = host_config
        .state_dir()
        .join(format!("{vm_name}.qmp.sock"))
        .to_string_lossy()
        .into_owned();

    let tpm_socket_path = has_tpm(&runtime).then(|| {
        host_config
            .state_dir()
            .join(format!("{vm_name}.tpm.sock"))
            .to_string_lossy()
            .into_owned()
    });

    let swtpm_pid = match tpm_socket_path.as_ref() {
        Some(socket_path) => {
            let _ = std::fs::remove_file(socket_path);
            let swtpm_args = vec![
                "socket".to_string(),
                "--ctrl".to_string(),
                format!("type=unixio,path={socket_path}"),
            ];
            let child = process::spawn_detached(host_config.swtpm_path(), &swtpm_args).map_err(
                |source| StartError::SwtpmSpawnFailed {
                    program: host_config.swtpm_path().clone(),
                    source,
                },
            )?;

            if let Err(error) = readiness::wait_for_socket(socket_path, Duration::from_secs(8)) {
                let _ = process::terminate_pid(child.id(), true);
                return Err(match error {
                    readiness::ReadinessError::Timeout { path, timeout_secs } => {
                        StartError::SwtpmNotReady { path, timeout_secs }
                    }
                });
            }

            Some(child.id())
        }
        None => None,
    };

    let ui_client = resolve_ui_client(display.as_ref(), &runtime, host_config);
    let ctx = QemuContext::new(vm_name.to_string(), String::new(), tpm_socket_path.clone());
    let qemu_commandline = QemuCommandLine::try_from((runtime, ctx))?;
    let rendered = format!(
        "{} -qmp unix:{},server=on,wait=off",
        qemu_commandline.to_string().trim(),
        qmp_socket_path
    );

    let mut args = host_config.qemu_default_args().clone();
    args.extend(rendered.split_whitespace().map(ToString::to_string));

    let child = process::spawn_detached(host_config.qemu_path(), &args).map_err(|source| {
        if let Some(pid) = swtpm_pid {
            let _ = process::terminate_pid(pid, true);
        }
        let _ = VmHandle::remove(host_config.state_dir(), vm_name);
        StartError::QemuSpawnFailed {
            program: host_config.qemu_path().clone(),
            source,
        }
    })?;

    let ui_client_pid = if let Some((binary, ui_args)) = ui_client {
        std::thread::sleep(Duration::from_secs(2));
        match process::spawn_detached(&binary, &ui_args) {
            Ok(child) => Some(child.id()),
            Err(source) => {
                eprintln!("warning: failed to launch UI client {binary}: {source}");
                None
            }
        }
    } else {
        None
    };

    let handle = VmHandle::new(
        vm_name.to_string(),
        child.id(),
        swtpm_pid,
        ui_client_pid,
        qmp_socket_path,
        tpm_socket_path,
    );
    handle.write(host_config.state_dir())?;
    Ok(())
}

fn has_tpm(runtime: &Runtime) -> bool {
    runtime
        .root_devices()
        .iter()
        .any(|device| device.device_kind() == RootDeviceKind::TpmState)
}
