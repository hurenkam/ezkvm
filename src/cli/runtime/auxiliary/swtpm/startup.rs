use anyhow::{Result, anyhow};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::helpers::{
    build_tpmstate_arg, ensure_run_dir, ensure_socket_parent_dir, resolve_tpm_socket_path,
    wait_for_unix_socket,
};

pub(crate) fn ensure_runtime_socket_dirs(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    if let Some(tpm) = config.system_tpm()
        && tpm.backend == "emulator"
    {
        ensure_socket_parent_dir(
            &resolve_tpm_socket_path(config, central_config),
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
) -> Result<()> {
    let Some(tpm) = config.system_tpm().filter(|tpm| tpm.backend == "emulator") else {
        return Ok(());
    };

    let swtpm_path = swtpm_path(central_config)?;
    let startup = prepare_swtpm_startup(config, central_config, tpm)?;
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

fn swtpm_path(central_config: &crate::config::CentralConfig) -> Result<String> {
    central_config.tools.swtpm.clone().ok_or_else(|| {
        anyhow!("TPM emulator backend requires tools.swtpm to be configured in the central config")
    })
}

fn prepare_swtpm_startup(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    tpm: &crate::config::TpmConfig,
) -> Result<SwtpmStartup> {
    let run_dir = ensure_run_dir(central_config)?;
    let socket_path = resolve_tpm_socket_path(config, central_config);
    ensure_socket_parent_dir(&socket_path, "TPM socket")?;

    let pid_path = run_dir.join(format!("{}.swtpm.pid", config.name));
    let log_path = run_dir.join(format!("{}-swtpm.log", config.name));
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
        println!(
            "Warning: failed to start swtpm at '{}': {}",
            swtpm_path, err
        );
        return Ok(());
    }

    wait_for_unix_socket(&startup.socket_path, Duration::from_secs(3), "swtpm socket")?;
    println!(
        "✓ Started swtpm emulator using socket {}",
        startup.socket_path
    );
    Ok(())
}
