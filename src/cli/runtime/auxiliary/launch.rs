use anyhow::{Result, anyhow};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct AuxiliaryLaunch {
    pub(crate) label: &'static str,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) inherit_output: bool,
    pub(crate) verify_running: bool,
}

pub(crate) fn resolve_client_host(addr: &str) -> String {
    match addr.trim() {
        "0.0.0.0" | "::" | "[::]" | "" => "127.0.0.1".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn format_auxiliary_launch(launch: &AuxiliaryLaunch) -> String {
    if launch.args.is_empty() {
        launch.program.clone()
    } else {
        format!("{} {}", launch.program, launch.args.join(" "))
    }
}

fn run_auxiliary_launch(launch: &AuxiliaryLaunch) -> Result<()> {
    tracing::debug!(
        target: "ezkvm::runtime::auxiliary",
        label = launch.label,
        program = %launch.program,
        args = ?launch.args,
        "spawning auxiliary process"
    );

    let mut cmd = Command::new(&launch.program);
    cmd.args(&launch.args).stdin(Stdio::null());

    if launch.inherit_output {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let mut child = cmd.spawn().map_err(|err| {
        tracing::warn!(
            target: "ezkvm::runtime::auxiliary",
            label = launch.label,
            program = %launch.program,
            error = %err,
            "failed to spawn auxiliary process"
        );
        anyhow!(
            "failed to start {} at '{}': {}",
            launch.label,
            launch.program,
            err
        )
    })?;

    if launch.verify_running {
        std::thread::sleep(Duration::from_millis(250));
        if let Some(status) = child.try_wait()? {
            tracing::warn!(
                target: "ezkvm::runtime::auxiliary",
                label = launch.label,
                status = %status,
                "auxiliary process exited immediately"
            );
            return Err(anyhow!(
                "{} exited immediately with status {}",
                launch.label,
                status
            ));
        }
    }

    tracing::info!(
        target: "ezkvm::runtime::auxiliary",
        label = launch.label,
        program = %launch.program,
        "auxiliary process started"
    );
    println!("✓ Started {}", launch.label);
    Ok(())
}

fn has_primary_passthrough_gpu(config: &crate::config::VmConfig) -> bool {
    config
        .host_pci()
        .iter()
        .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
}

fn vnc_display_to_tcp_port(display_or_port: u16) -> u16 {
    if display_or_port < 100 {
        5900 + display_or_port
    } else {
        display_or_port
    }
}

fn parse_vnc_host_and_display(endpoint: &str) -> Option<(String, u16)> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() || endpoint.starts_with("unix:") {
        return None;
    }

    if let Some(display) = endpoint.strip_prefix(':') {
        let display = display.parse::<u16>().ok()?;
        return Some(("127.0.0.1".to_string(), display));
    }

    let (host, display) = if endpoint.starts_with('[') {
        let bracket_end = endpoint.find(']')?;
        let host = &endpoint[..=bracket_end];
        let remainder = endpoint.get(bracket_end + 1..)?;
        let display = remainder.strip_prefix(':')?;
        (host, display)
    } else {
        endpoint.rsplit_once(':')?
    };

    let host = if host.trim().is_empty() {
        "127.0.0.1"
    } else {
        host.trim()
    };

    let display = display.parse::<u16>().ok()?;
    Some((resolve_client_host(host), display))
}

fn format_uri_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}

fn vnc_uri_from_display(endpoint: &str) -> Option<String> {
    let (host, display) = parse_vnc_host_and_display(endpoint)?;
    let host = format_uri_host(&host);
    let port = vnc_display_to_tcp_port(display);
    Some(format!("vnc://{}:{}", host, port))
}

pub(crate) fn build_remote_viewer_launch(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Option<AuxiliaryLaunch> {
    if has_primary_passthrough_gpu(config) {
        return None;
    }

    let remote_viewer_path =
        central_config.remote_viewer_program_with_overrides(runtime_overrides)?;

    let uri = if let Some(spice) = &config.spice
        && spice.enabled
    {
        format!(
            "spice://{}:{}",
            resolve_client_host(&spice.addr),
            spice.port
        )
    } else if let Some(vnc) = &config.vnc
        && vnc.enabled
    {
        vnc_uri_from_display(&vnc.display)?
    } else {
        return None;
    };

    Some(AuxiliaryLaunch {
        label: "remote-viewer session",
        program: remote_viewer_path.to_string(),
        args: vec![uri],
        inherit_output: false,
        verify_running: false,
    })
}

pub(crate) fn build_looking_glass_launch(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<Option<AuxiliaryLaunch>> {
    if !has_primary_passthrough_gpu(config) {
        return Ok(None);
    }

    let ivshmem = match config.system_memory_ivshmem() {
        Some(ivshmem) if ivshmem.enabled => ivshmem,
        _ => return Ok(None),
    };

    let looking_glass_options = config
        .options
        .looking_glass
        .as_ref()
        .unwrap_or(&central_config.looking_glass);

    let looking_glass_path = match crate::state::resolve_looking_glass_program(
        config.options.looking_glass.as_ref(),
        central_config,
        runtime_overrides,
    )? {
        Some(path) => path,
        None => return Ok(None),
    };

    let mut args = vec![format!("app:shmFile={}", ivshmem.mem_path)];

    if let Some(full_screen) = looking_glass_options.full_screen {
        args.push(format!("win:fullScreen={}", full_screen));
    }

    if let Some(size) = looking_glass_options.size.as_deref()
        && !size.trim().is_empty()
    {
        args.push(format!("win:size={}", size));
    }

    if let Some(grab_keyboard) = looking_glass_options.grab_keyboard {
        args.push(format!("input:grabKeyboard={}", grab_keyboard));
    }

    if let Some(escape_key) = looking_glass_options.escape_key.as_deref()
        && !escape_key.trim().is_empty()
    {
        args.push(format!("input:escapeKey={}", escape_key));
    }

    if let Some(spice) = &config.spice
        && spice.enabled
    {
        args.push(format!("spice:host={}", resolve_client_host(&spice.addr)));
        args.push(format!("spice:port={}", spice.port));
    }

    Ok(Some(AuxiliaryLaunch {
        label: "Looking Glass client for ivshmem session",
        program: looking_glass_path,
        args,
        inherit_output: true,
        verify_running: true,
    }))
}

fn wait_for_tcp_endpoint(host: &str, port: u16, timeout: Duration, label: &str) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_error = None;

    while Instant::now() < deadline {
        match TcpStream::connect((host, port)) {
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
            "timed out waiting for {} {}:{} to become ready: {}",
            label,
            host,
            port,
            err
        )),
        None => Err(anyhow!(
            "timed out waiting for {} {}:{} to become ready",
            label,
            host,
            port
        )),
    }
}

pub(crate) fn spawn_remote_viewer(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    if let Some(launch) = build_remote_viewer_launch(config, central_config, runtime_overrides) {
        run_auxiliary_launch(&launch)?;
    }

    Ok(())
}

pub(crate) fn spawn_looking_glass(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    let Some(launch) = build_looking_glass_launch(config, central_config, runtime_overrides)?
    else {
        return Ok(());
    };

    if !Path::new(&config.system_memory_ivshmem().unwrap().mem_path).exists() {
        return Err(anyhow!(
            "Looking Glass shared memory path '{}' does not exist",
            config.system_memory_ivshmem().unwrap().mem_path
        ));
    }

    if let Some(spice) = &config.spice
        && spice.enabled
    {
        wait_for_tcp_endpoint(
            &resolve_client_host(&spice.addr),
            spice.port,
            Duration::from_secs(5),
            "SPICE server",
        )?;
    }

    run_auxiliary_launch(&launch)?;

    Ok(())
}
