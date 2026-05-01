use anyhow::{Result, anyhow};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::helpers::{
    build_tpmstate_arg, ensure_run_dir, ensure_socket_parent_dir, resolve_swtpm_log_path,
    resolve_tpm_socket_path, wait_for_unix_socket,
};

pub(crate) fn ensure_runtime_socket_dirs(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    if let Some(tpm) = config.system_tpm()
        && tpm.backend == "emulator"
    {
        ensure_socket_parent_dir(
            &resolve_tpm_socket_path(config, central_config, runtime_overrides),
            "TPM socket",
        )?;
    }

    if let Some(guest_agent) = config.options_guest_agent()
        && guest_agent.enabled
        && let Some(socket_path) = guest_agent.socket_path.as_deref()
    {
        ensure_socket_parent_dir(socket_path, "guest agent socket")?;
    }

    if let Some(qmp) = config.options_qmp()
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
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    let Some(tpm) = config.system_tpm().filter(|tpm| tpm.backend == "emulator") else {
        return Ok(());
    };

    // In ProxmoxParity mode, an explicit state_path means qemu-server manages swtpm externally.
    // In PortableLinux mode, ezkvm must start swtpm regardless of whether state_path is set.
    let runtime_mode = crate::state::detect_runtime_capability_mode(config);
    if runtime_mode == crate::state::RuntimeCapabilityMode::ProxmoxParity
        && tpm.state_path.is_some()
    {
        return Ok(());
    }

    let placement_mode =
        crate::state::resolve_tpm_placement_mode(central_config, runtime_overrides);
    if placement_mode == crate::state::TpmPlacementMode::StateFile {
        return Ok(());
    }

    let swtpm_path = swtpm_path(central_config, runtime_overrides)?;
    let startup = prepare_swtpm_startup(config, central_config, runtime_overrides, tpm)?;
    let rendered_cmd = render_swtpm_command(&swtpm_path, &startup);
    println!("Launching swtpm: {}", rendered_cmd.join(" "));

    spawn_swtpm(&swtpm_path, &startup)?;

    Ok(())
}

struct SwtpmStartup {
    socket_path: String,
    tpm_flag: &'static str,
    tpmstate_arg: String,
    ctrl_arg: String,
    pid_arg: String,
    log_arg: String,
}

fn swtpm_path(
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<String> {
    crate::state::resolve_swtpm_binary(central_config, runtime_overrides).ok_or_else(|| {
        anyhow!(
            "TPM emulator backend in socket mode requires --swtpm-binary, host_capabilities.tpm.swtpm_binary, PATH swtpm, or legacy tools.swtpm"
        )
    })
}

fn prepare_swtpm_startup(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    tpm: &crate::config::TpmConfig,
) -> Result<SwtpmStartup> {
    let run_dir = ensure_run_dir(central_config, runtime_overrides)?;
    let socket_path = resolve_tpm_socket_path(config, central_config, runtime_overrides);
    ensure_socket_parent_dir(&socket_path, "TPM socket")?;

    let pid_path = run_dir.join(format!("{}.swtpm.pid", config.name));
    let log_path = resolve_swtpm_log_path(config, central_config, &run_dir);
    ensure_socket_parent_dir(&log_path.display().to_string(), "swtpm log file")?;
    let tpmstate_arg = build_tpmstate_arg(tpm, &run_dir, true)?;

    Ok(SwtpmStartup {
        socket_path: socket_path.clone(),
        tpm_flag: if tpm.version == "2.0" {
            "--tpm2"
        } else {
            "--tpm"
        },
        tpmstate_arg,
        ctrl_arg: format!("type=unixio,path={},mode=0600", socket_path),
        pid_arg: format!("file={}", pid_path.display()),
        log_arg: format!("file={},level=1", log_path.display()),
    })
}

fn render_swtpm_command(swtpm_path: &str, startup: &SwtpmStartup) -> Vec<String> {
    vec![
        swtpm_path.to_string(),
        "socket".to_string(),
        startup.tpm_flag.to_string(),
        "--tpmstate".to_string(),
        startup.tpmstate_arg.clone(),
        "--ctrl".to_string(),
        startup.ctrl_arg.clone(),
        "--pid".to_string(),
        startup.pid_arg.clone(),
        "--terminate".to_string(),
        "--log".to_string(),
        startup.log_arg.clone(),
        "--daemon".to_string(),
    ]
}

fn spawn_swtpm(swtpm_path: &str, startup: &SwtpmStartup) -> Result<()> {
    let mut cmd = Command::new(swtpm_path);
    cmd.arg("socket")
        .arg(startup.tpm_flag)
        .arg("--tpmstate")
        .arg(&startup.tpmstate_arg)
        .arg("--ctrl")
        .arg(&startup.ctrl_arg)
        .arg("--pid")
        .arg(&startup.pid_arg)
        .arg("--terminate")
        .arg("--log")
        .arg(&startup.log_arg)
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Err(err) = cmd.spawn() {
        return Err(anyhow!(
            "failed to start swtpm at '{}': {}",
            swtpm_path,
            err
        ));
    }

    wait_for_unix_socket(&startup.socket_path, Duration::from_secs(3), "swtpm socket")?;
    println!(
        "✓ Started swtpm emulator using socket {}",
        startup.socket_path
    );
    Ok(())
}
