use anyhow::{Result, anyhow};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn ensure_run_dir(central_config: &crate::config::CentralConfig) -> Result<PathBuf> {
    let run_dir = central_config
        .locations
        .run_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/run/ezkvm"));

    std::fs::create_dir_all(&run_dir)?;
    Ok(run_dir)
}

fn ensure_socket_parent_dir(socket_path: &str, label: &str) -> Result<()> {
    let parent = Path::new(socket_path).parent().ok_or_else(|| {
        anyhow!(
            "{} '{}' does not have a parent directory",
            label,
            socket_path
        )
    })?;

    std::fs::create_dir_all(parent).map_err(|err| {
        anyhow!(
            "failed to create parent directory for {} '{}': {}",
            label,
            socket_path,
            err
        )
    })
}

fn wait_for_unix_socket(socket_path: &str, timeout: Duration, label: &str) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_error = None;

    while Instant::now() < deadline {
        match UnixStream::connect(socket_path) {
            Ok(stream) => {
                drop(stream);
                return Ok(());
            }
            Err(err) => {
                last_error = Some(err);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    match last_error {
        Some(err) => Err(anyhow!(
            "timed out waiting for {} '{}' to become ready: {}",
            label,
            socket_path,
            err
        )),
        None => Err(anyhow!(
            "timed out waiting for {} '{}' to become ready",
            label,
            socket_path
        )),
    }
}

fn resolve_tpm_socket_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> String {
    if let Some(tpm) = &config.tpm
        && let Some(state_path) = &tpm.state_path
    {
        return state_path.clone();
    }

    if let Some(run_dir) = &central_config.locations.run_dir {
        return format!("{}/{}.swtpm", run_dir, config.name);
    }

    format!("/var/run/qemu-server/{}.swtpm", config.name)
}

pub(crate) fn ensure_runtime_socket_dirs(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    if let Some(tpm) = &config.tpm
        && tpm.backend == "emulator"
    {
        ensure_socket_parent_dir(
            &resolve_tpm_socket_path(config, central_config),
            "TPM socket",
        )?;
    }

    if let Some(guest_agent) = &config.guest_agent
        && guest_agent.enabled
        && let Some(socket_path) = guest_agent.socket_path.as_deref()
    {
        ensure_socket_parent_dir(socket_path, "guest agent socket")?;
    }

    if let Some(qmp) = &config.qmp
        && qmp.enabled
        && let crate::config::QmpSocketType::Unix = qmp.socket_type
        && let Some(socket_path) = qmp.socket_path.as_deref()
    {
        ensure_socket_parent_dir(socket_path, "QMP socket")?;
    }

    Ok(())
}

pub(crate) fn start_swtpm_if_configured(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    let tpm = match &config.tpm {
        Some(tpm) if tpm.backend == "emulator" => tpm,
        _ => return Ok(()),
    };

    let swtpm_path = match &central_config.tools.swtpm {
        Some(path) => path,
        None => {
            return Err(anyhow!(
                "TPM emulator backend requires tools.swtpm to be configured in the central config"
            ));
        }
    };

    let run_dir = ensure_run_dir(central_config)?;
    let socket_path = resolve_tpm_socket_path(config, central_config);
    ensure_socket_parent_dir(&socket_path, "TPM socket")?;
    let pid_path = run_dir.join(format!("{}.swtpm.pid", config.name));
    let log_path = run_dir.join(format!("{}-swtpm.log", config.name));

    let tpmstate_arg = if let Some(uri) = tpm.state_backend_uri.as_deref() {
        let trimmed = uri.trim();
        let normalized = if let Some(stripped) = trimmed.strip_prefix("file://dev/") {
            format!("file:///dev/{}", stripped)
        } else {
            trimmed.to_string()
        };
        let mut backend = if normalized.starts_with('/') {
            format!("backend-uri=file://{}", normalized)
        } else {
            format!("backend-uri={}", normalized)
        };
        if !backend.contains(",mode=") {
            backend.push_str(",mode=0600");
        }
        backend
    } else {
        let state_dir_path: std::path::PathBuf = if let Some(explicit) = tpm.state_dir.as_deref() {
            explicit.into()
        } else {
            let default = run_dir.join("tpm-state");
            std::fs::create_dir_all(&default)?;
            default
        };
        format!("dir={}", state_dir_path.display())
    };

    let mut cmd = Command::new(swtpm_path);
    let mut rendered_cmd: Vec<String> = vec![swtpm_path.clone(), "socket".to_string()];

    let tpm_flag = if tpm.version == "2.0" {
        "--tpm2"
    } else {
        "--tpm"
    };
    let ctrl_arg = format!("type=unixio,path={},mode=0600", socket_path);
    let pid_arg = format!("file={}", pid_path.display());
    let log_arg = format!("file={},level=1", log_path.display());

    rendered_cmd.push(tpm_flag.to_string());
    rendered_cmd.push("--tpmstate".to_string());
    rendered_cmd.push(tpmstate_arg.clone());
    rendered_cmd.push("--ctrl".to_string());
    rendered_cmd.push(ctrl_arg.clone());
    rendered_cmd.push("--pid".to_string());
    rendered_cmd.push(pid_arg.clone());
    rendered_cmd.push("--terminate".to_string());
    rendered_cmd.push("--log".to_string());
    rendered_cmd.push(log_arg.clone());
    rendered_cmd.push("--daemon".to_string());

    println!("Launching swtpm: {}", rendered_cmd.join(" "));

    cmd.arg("socket")
        .arg(tpm_flag)
        .arg("--tpmstate")
        .arg(tpmstate_arg)
        .arg("--ctrl")
        .arg(ctrl_arg)
        .arg("--pid")
        .arg(pid_arg)
        .arg("--terminate")
        .arg("--log")
        .arg(log_arg)
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Err(err) = cmd.spawn() {
        println!(
            "Warning: failed to start swtpm at '{}': {}",
            swtpm_path, err
        );
    } else {
        wait_for_unix_socket(&socket_path, Duration::from_secs(3), "swtpm socket")?;
        println!("✓ Started swtpm emulator using socket {}", socket_path);
    }

    Ok(())
}

pub(crate) fn build_swtpm_launch_preview(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<Option<String>> {
    let tpm = match &config.tpm {
        Some(tpm) if tpm.backend == "emulator" => tpm,
        _ => return Ok(None),
    };

    let swtpm_path = match &central_config.tools.swtpm {
        Some(path) => path,
        None => {
            return Err(anyhow!(
                "TPM emulator backend requires tools.swtpm to be configured in the central config"
            ));
        }
    };

    let run_dir = central_config
        .locations
        .run_dir
        .as_deref()
        .unwrap_or("/var/run/ezkvm");

    let socket_path = resolve_tpm_socket_path(config, central_config);
    let pid_path = Path::new(run_dir).join(format!("{}.swtpm.pid", config.name));
    let log_path = Path::new(run_dir).join(format!("{}-swtpm.log", config.name));

    let tpmstate_arg = if let Some(uri) = tpm.state_backend_uri.as_deref() {
        let trimmed = uri.trim();
        let normalized = if let Some(stripped) = trimmed.strip_prefix("file://dev/") {
            format!("file:///dev/{}", stripped)
        } else {
            trimmed.to_string()
        };
        let mut backend = if normalized.starts_with('/') {
            format!("backend-uri=file://{}", normalized)
        } else {
            format!("backend-uri={}", normalized)
        };
        if !backend.contains(",mode=") {
            backend.push_str(",mode=0600");
        }
        backend
    } else {
        let state_dir = if let Some(explicit) = tpm.state_dir.as_deref() {
            explicit.to_string()
        } else {
            Path::new(run_dir)
                .join("tpm-state")
                .to_string_lossy()
                .to_string()
        };
        format!("dir={}", state_dir)
    };

    let tpm_flag = if tpm.version == "2.0" {
        "--tpm2"
    } else {
        "--tpm"
    };
    let ctrl_arg = format!("type=unixio,path={},mode=0600", socket_path);
    let pid_arg = format!("file={}", pid_path.display());
    let log_arg = format!("file={},level=1", log_path.display());

    let rendered = vec![
        swtpm_path.clone(),
        "socket".to_string(),
        tpm_flag.to_string(),
        "--tpmstate".to_string(),
        tpmstate_arg,
        "--ctrl".to_string(),
        ctrl_arg,
        "--pid".to_string(),
        pid_arg,
        "--terminate".to_string(),
        "--log".to_string(),
        log_arg,
        "--daemon".to_string(),
    ];

    Ok(Some(rendered.join(" ")))
}
