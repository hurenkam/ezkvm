use anyhow::{Result, anyhow};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
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

pub(crate) fn resolve_client_host(addr: &str) -> String {
    match addr.trim() {
        "0.0.0.0" | "::" | "[::]" | "" => "127.0.0.1".to_string(),
        other => other.to_string(),
    }
}

fn ensure_runtime_socket_dirs(
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

pub(crate) fn format_auxiliary_launch(launch: &AuxiliaryLaunch) -> String {
    if launch.args.is_empty() {
        launch.program.clone()
    } else {
        format!("{} {}", launch.program, launch.args.join(" "))
    }
}

fn run_auxiliary_launch(launch: &AuxiliaryLaunch) -> Result<()> {
    let mut cmd = Command::new(&launch.program);
    cmd.args(&launch.args).stdin(Stdio::null());

    if launch.inherit_output {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let mut child = cmd.spawn().map_err(|err| {
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
            return Err(anyhow!(
                "{} exited immediately with status {}",
                launch.label,
                status
            ));
        }
    }

    println!("✓ Started {}", launch.label);
    Ok(())
}

fn has_primary_passthrough_gpu(config: &crate::config::VmConfig) -> bool {
    config
        .hostpci
        .iter()
        .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
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

pub(crate) fn build_remote_viewer_launch(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Option<AuxiliaryLaunch> {
    if has_primary_passthrough_gpu(config) {
        return None;
    }

    let spice = match &config.spice {
        Some(spice) if spice.enabled => spice,
        _ => return None,
    };

    let remote_viewer_path = central_config.tools.remote_viewer.as_ref()?;
    let uri = format!(
        "spice://{}:{}",
        resolve_client_host(&spice.addr),
        spice.port
    );

    Some(AuxiliaryLaunch {
        label: "remote-viewer for SPICE session",
        program: remote_viewer_path.clone(),
        args: vec![uri],
        inherit_output: false,
        verify_running: false,
    })
}

pub(crate) fn build_looking_glass_launch(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<Option<AuxiliaryLaunch>> {
    if !has_primary_passthrough_gpu(config) {
        return Ok(None);
    }

    let ivshmem = match &config.ivshmem {
        Some(ivshmem) if ivshmem.enabled => ivshmem,
        _ => return Ok(None),
    };

    let looking_glass_path = match &central_config.tools.looking_glass {
        Some(path) if !path.trim().is_empty() => path,
        Some(_) => return Err(anyhow!("Looking Glass client path is empty")),
        None => return Ok(None),
    };

    let mut args = vec![format!("app:shmFile={}", ivshmem.mem_path)];

    if let Some(full_screen) = central_config.looking_glass.full_screen {
        args.push(format!("win:fullScreen={}", full_screen));
    }

    if let Some(size) = central_config.looking_glass.size.as_deref()
        && !size.trim().is_empty()
    {
        args.push(format!("win:size={}", size));
    }

    if let Some(grab_keyboard) = central_config.looking_glass.grab_keyboard {
        args.push(format!("input:grabKeyboard={}", grab_keyboard));
    }

    if let Some(escape_key) = central_config.looking_glass.escape_key.as_deref()
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
        program: looking_glass_path.clone(),
        args,
        inherit_output: true,
        verify_running: true,
    }))
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

fn build_swtpm_launch_preview(
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

fn spawn_remote_viewer(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    if let Some(launch) = build_remote_viewer_launch(config, central_config) {
        run_auxiliary_launch(&launch)?;
    }

    Ok(())
}

fn spawn_looking_glass(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    let Some(launch) = build_looking_glass_launch(config, central_config)? else {
        return Ok(());
    };

    if !std::path::Path::new(&config.ivshmem.as_ref().unwrap().mem_path).exists() {
        return Err(anyhow!(
            "Looking Glass shared memory path '{}' does not exist",
            config.ivshmem.as_ref().unwrap().mem_path
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

pub(crate) async fn handle_start(config_path: &str, daemon: bool, dry_run: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let mut config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");

    let central_config = crate::config::CentralConfig::load()?;
    println!("✓ Central configuration loaded");

    if !dry_run {
        ensure_runtime_socket_dirs(&config, &central_config)?;
        start_swtpm_if_configured(&config, &central_config)?;
    }

    let central_config_clone = central_config.clone();
    config.options.daemonize = daemon;

    crate::state::cache_config(&config.name, &config)?;

    let pid_file = crate::state::get_pid_file_at(&config.name, config.options.pid_file.as_deref())?;
    let log_file = if daemon || config.options.log_dir.is_some() {
        crate::state::cleanup_old_logs_at(
            &config.name,
            config.options.log_dir.as_deref(),
            config.options.log_keep,
        )?;
        Some(crate::state::create_session_log_file(
            &config.name,
            config.options.log_dir.as_deref(),
        )?)
    } else {
        None
    };

    let manager = crate::qemu::QemuManager::new(config, central_config);
    let args = manager.build_command()?;

    println!("Starting VM: {}", manager.config().name);
    println!("QEMU binary: {}", manager.binary_name());

    crate::qemu::executor::check_qemu_available(&manager.binary_name())?;
    println!("✓ QEMU binary found");

    if dry_run {
        println!("Dry run mode - would execute:");
        println!(
            "{} {}",
            manager.binary_name(),
            args.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        println!("PID file: {}", pid_file.display());
        if let Some(log_file) = &log_file {
            println!("Log file: {}", log_file.display());
        }

        match build_swtpm_launch_preview(manager.config(), &central_config_clone) {
            Ok(Some(cmd)) => println!("Auxiliary launch (swtpm): {}", cmd),
            Ok(None) => {}
            Err(err) => println!("swtpm configuration error: {}", err),
        }

        if let Some(launch) = build_remote_viewer_launch(manager.config(), &central_config_clone) {
            println!(
                "Auxiliary launch (SPICE): {}",
                format_auxiliary_launch(&launch)
            );
        }

        match build_looking_glass_launch(manager.config(), &central_config_clone) {
            Ok(Some(launch)) => {
                println!(
                    "Auxiliary launch (Looking Glass): {}",
                    format_auxiliary_launch(&launch)
                );
                if !std::path::Path::new(&manager.config().ivshmem.as_ref().unwrap().mem_path)
                    .exists()
                {
                    println!(
                        "Looking Glass note: shared memory path '{}' does not exist on this host",
                        manager.config().ivshmem.as_ref().unwrap().mem_path
                    );
                }
            }
            Ok(None) => {}
            Err(err) => println!("Looking Glass configuration error: {}", err),
        }
        return Ok(());
    }

    let executor = crate::qemu::executor::QemuExecutor::new(manager.binary_name(), args);

    if daemon {
        println!("Starting in daemon mode...");
        if let Some(log_file) = &log_file {
            println!("Logging QEMU output to {}", log_file.display());
            let _status = executor.execute_sync_logged(log_file, Stdio::null())?;
        } else {
            let _status = executor.execute_sync()?;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Ok(Some(pid)) = crate::state::read_pid_at(
            &manager.config().name,
            manager.config().options.pid_file.as_deref(),
        ) {
            println!(
                "✓ VM '{}' started (daemonized) - PID {}",
                manager.config().name,
                pid
            );
        } else if let Ok(pids) = crate::qemu::process::find_qemu_processes(&manager.config().name) {
            if let Some(pid) = pids.first() {
                crate::state::save_pid_at(
                    &manager.config().name,
                    *pid,
                    manager.config().options.pid_file.as_deref(),
                )?;
                println!(
                    "✓ VM '{}' started (daemonized) - PID {}",
                    manager.config().name,
                    pid
                );
            } else {
                println!("✓ VM '{}' started (daemonized)", manager.config().name);
            }
        } else {
            println!("✓ VM '{}' started (daemonized)", manager.config().name);
        }

        if let Err(err) = spawn_remote_viewer(manager.config(), &central_config_clone) {
            eprintln!("Warning: {}", err);
        }
        if let Err(err) = spawn_looking_glass(manager.config(), &central_config_clone) {
            eprintln!("Warning: {}", err);
        }
    } else {
        println!("Starting interactively...");

        if let Err(err) = spawn_remote_viewer(manager.config(), &central_config_clone) {
            eprintln!("Warning: {}", err);
        }
        if let Err(err) = spawn_looking_glass(manager.config(), &central_config_clone) {
            eprintln!("Warning: {}", err);
        }

        let status = if let Some(log_file) = &log_file {
            println!("Logging QEMU output to {}", log_file.display());
            executor.execute_sync_logged(log_file, Stdio::inherit())?
        } else {
            executor.execute_sync()?
        };
        let _ = crate::state::delete_pid_at(
            &manager.config().name,
            manager.config().options.pid_file.as_deref(),
        );
        println!(
            "✓ VM '{}' finished with exit code {}",
            manager.config().name,
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

pub(crate) async fn handle_stop(config_path: &str, force: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");

    println!("Stopping VM: {}", config.name);

    if force {
        println!("Force stopping...");
        crate::qemu::process::kill_vm(&config.name)?;
        println!("✓ VM '{}' force stopped", config.name);
    } else {
        println!("Gracefully stopping...");
        crate::qemu::process::stop_vm(&config.name)?;
        println!("✓ VM '{}' stopped", config.name);
    }

    let _ = crate::state::delete_pid_at(&config.name, config.options.pid_file.as_deref());

    Ok(())
}

pub(crate) async fn handle_kill(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");

    println!("Killing VM: {}", config.name);

    crate::qemu::process::kill_vm(&config.name)?;
    println!("✓ VM '{}' killed", config.name);

    let _ = crate::state::delete_pid_at(&config.name, config.options.pid_file.as_deref());

    Ok(())
}

pub(crate) async fn handle_list() -> Result<()> {
    println!("Running VMs:");

    match crate::qemu::process::list_running_vms() {
        Ok(vms) => {
            if vms.is_empty() {
                println!("No VMs currently running");
            } else {
                for (name, pid) in vms {
                    println!("  {} (PID: {})", name, pid);
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing VMs: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

pub(crate) async fn handle_status(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");

    let vm_name = &config.name;
    println!("Status of VM: {}", vm_name);

    if let Ok(Some(pid)) = crate::state::read_pid_at(vm_name, config.options.pid_file.as_deref()) {
        match crate::qemu::process::find_qemu_processes(vm_name) {
            Ok(pids) if pids.contains(&pid) => {
                println!("Status: Running (PID: {})", pid);
                println!("Memory: {} MiB", config.system.memory);
                println!("vCPUs: {}", config.system.vcpus);
                return Ok(());
            }
            _ => {
                let _ = crate::state::delete_pid_at(vm_name, config.options.pid_file.as_deref());
            }
        }
    }

    match crate::qemu::process::is_vm_running(vm_name) {
        Ok(is_running) => {
            if is_running {
                println!("Status: Running");
                println!("Memory: {} MiB", config.system.memory);
                println!("vCPUs: {}", config.system.vcpus);
            } else {
                println!("Status: Not running");
            }
        }
        Err(e) => {
            eprintln!("Error checking VM status: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

pub(crate) async fn handle_console(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");

    println!("Attaching to console of VM: {}", config.name);

    match crate::qemu::process::is_vm_running(&config.name) {
        Ok(is_running) if is_running => {
            println!("\nVM is running. Attempting VNC connection...");
            println!("VNC Server: localhost:5900");
            println!("\nYou can connect using:");
            println!("  vncviewer localhost:5900");
            println!("  or any other VNC client\n");

            if std::process::Command::new("which")
                .arg("vncviewer")
                .output()
                .is_ok()
            {
                println!("Attempting to launch vncviewer...");
                let _ = std::process::Command::new("vncviewer")
                    .arg("localhost:5900")
                    .spawn();
            }
        }
        Ok(_) => {
            println!("\nError: VM '{}' is not running", config.name);
            println!("Start the VM first with: ezkvm start {}", config_path);
            return Err(anyhow!("VM is not running"));
        }
        Err(e) => {
            eprintln!("Error checking VM status: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

pub(crate) async fn handle_validate(config_path: &str, show_resolved_config: bool) -> Result<()> {
    println!("Validating configuration: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration is valid");
    println!("VM Name: {}", config.name);
    println!("Architecture: {}", config.system.architecture);
    println!("Memory: {} MiB", config.system.memory);
    println!("vCPUs: {}", config.system.vcpus);

    if show_resolved_config {
        println!("\nResolved configuration:");
        let resolved_yaml = serde_yaml::to_string(&config)?;
        print!("{}", resolved_yaml);
    }

    Ok(())
}
