use super::auxiliary::{
    build_looking_glass_launch, build_remote_viewer_launch, build_swtpm_launch_preview,
    ensure_runtime_socket_dirs, format_auxiliary_launch, spawn_looking_glass, spawn_remote_viewer,
    start_swtpm_if_configured,
};
use super::preflight::run_runtime_preflight;
use anyhow::Result;
use std::path::Path;
use std::process::Stdio;

pub(crate) async fn handle_start(
    config_path: &str,
    daemon: bool,
    dry_run: bool,
    runtime_overrides: crate::config::RuntimeCliOverrides,
) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let (mut config, central_config) = load_start_configs(config_path)?;
    config.options.daemonize = daemon;

    let qemu_binary = config.system.qemu_binary();
    let preflight =
        run_runtime_preflight(&config, &central_config, &runtime_overrides, &qemu_binary)?;
    println!("✓ Runtime preflight checks passed");
    for warning in preflight.optional_warnings() {
        println!("Preflight warning: {}", warning);
    }

    prepare_auxiliary_runtime(&config, &central_config, &runtime_overrides, dry_run)?;

    let central_config_clone = central_config.clone();
    crate::state::cache_config(&config.name, &config)?;

    let pid_file = crate::state::get_pid_file_at(&config.name, config.options.pid_file.as_deref())?;
    let log_file = prepare_log_file(&config, daemon)?;

    let manager = crate::qemu::QemuManager::new_with_overrides(
        config,
        central_config,
        runtime_overrides.clone(),
    );
    let args = manager.build_command()?;

    println!("Starting VM: {}", manager.config().name);
    println!("QEMU binary: {}", manager.binary_name());

    if dry_run {
        print_dry_run(
            &manager,
            &args,
            &pid_file,
            log_file.as_ref(),
            &central_config_clone,
            &runtime_overrides,
        );
        return Ok(());
    }

    let executor = crate::qemu::executor::QemuExecutor::new(manager.binary_name(), args);
    if daemon {
        run_daemon_start(
            &manager,
            &executor,
            log_file.as_ref(),
            &central_config_clone,
            &runtime_overrides,
        )?;
    } else {
        run_interactive_start(
            &manager,
            &executor,
            log_file.as_ref(),
            &central_config_clone,
            &runtime_overrides,
        )?;
    }

    Ok(())
}

fn load_start_configs(
    config_path: &str,
) -> Result<(crate::config::VmConfig, crate::config::CentralConfig)> {
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");

    let central_config = crate::config::CentralConfig::load()?;
    println!("✓ Central configuration loaded");

    Ok((config, central_config))
}

fn prepare_auxiliary_runtime(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    ensure_runtime_socket_dirs(config, central_config, runtime_overrides)?;
    start_swtpm_if_configured(config, central_config, runtime_overrides)?;
    Ok(())
}

fn prepare_log_file(
    config: &crate::config::VmConfig,
    daemon: bool,
) -> Result<Option<std::path::PathBuf>> {
    if !daemon && config.options.log_dir.is_none() {
        return Ok(None);
    }

    crate::state::cleanup_old_logs_at(
        &config.name,
        config.options.log_dir.as_deref(),
        config.options.log_keep,
    )?;
    let log_file =
        crate::state::create_session_log_file(&config.name, config.options.log_dir.as_deref())?;
    Ok(Some(log_file))
}

fn print_dry_run(
    manager: &crate::qemu::QemuManager,
    args: &crate::qemu::types::QemuArgs,
    pid_file: &std::path::Path,
    log_file: Option<&std::path::PathBuf>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) {
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
    if let Some(log_file) = log_file {
        println!("Log file: {}", log_file.display());
    }

    if let Ok(Some(cmd)) =
        build_swtpm_launch_preview(manager.config(), central_config, runtime_overrides)
    {
        println!("Auxiliary launch (swtpm): {}", cmd);
    }

    if let Some(launch) =
        build_remote_viewer_launch(manager.config(), central_config, runtime_overrides)
    {
        println!(
            "Auxiliary launch (SPICE): {}",
            format_auxiliary_launch(&launch)
        );
    }

    if let Ok(Some(launch)) =
        build_looking_glass_launch(manager.config(), central_config, runtime_overrides)
    {
        println!(
            "Auxiliary launch (Looking Glass): {}",
            format_auxiliary_launch(&launch)
        );
        if !Path::new(&manager.config().system_memory_ivshmem().unwrap().mem_path).exists() {
            println!(
                "Looking Glass note: shared memory path '{}' does not exist on this host",
                manager.config().system_memory_ivshmem().unwrap().mem_path
            );
        }
    }

    print_capability_diagnostics(manager, central_config, runtime_overrides);
}

fn print_capability_diagnostics(
    manager: &crate::qemu::QemuManager,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) {
    let mode = crate::state::detect_runtime_capability_mode(manager.config());
    println!("Capability resolution diagnostics:");
    println!("  mode: {:?}", mode);

    if mode == crate::state::RuntimeCapabilityMode::ProxmoxParity {
        println!(
            "  runtime capabilities: source={}, value=proxmox-parity defaults",
            crate::state::CapabilitySource::ParityBypass
        );
        return;
    }

    let runtime_root =
        crate::state::resolve_runtime_root_with_source(None, central_config, runtime_overrides);
    println!(
        "  runtime_root: source={}, value={}",
        runtime_root
            .source
            .map(|source| source.to_string())
            .unwrap_or_else(|| "unresolved".to_string()),
        runtime_root.value.as_deref().unwrap_or("<none>")
    );

    if manager
        .config()
        .system_tpm()
        .is_some_and(|tpm| tpm.backend == "emulator")
    {
        let swtpm =
            crate::state::resolve_swtpm_binary_with_source(central_config, runtime_overrides);
        println!(
            "  swtpm_binary: source={}, value={}",
            swtpm
                .source
                .map(|source| source.to_string())
                .unwrap_or_else(|| "unresolved".to_string()),
            swtpm.value.as_deref().unwrap_or("<none>")
        );
    }

    let firmware = manager.config().system_boot();
    let firmware_kind = firmware.firmware.as_deref().unwrap_or("bios");
    if firmware_kind == "uefi" || firmware_kind == "ovmf" {
        let secure_boot = firmware.secure_boot;
        let uefi_resolution = if let Some(ovmf_dir) = runtime_overrides
            .ovmf_dir
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            crate::state::CapabilityResolution::with_value(
                crate::qemu::resolve_ovmf_code_from_dir(ovmf_dir, secure_boot),
                crate::state::CapabilitySource::CliOverride,
            )
        } else if let Some(explicit) = firmware.uefi_code.as_deref() {
            crate::state::CapabilityResolution::with_value(
                explicit.to_string(),
                crate::state::CapabilitySource::VmOverride,
            )
        } else {
            crate::qemu::CentralFirmwareCapabilityResolver::new(central_config, runtime_overrides)
                .resolve_ovmf_code_with_source(secure_boot)
        };

        println!(
            "  ovmf_code: source={}, value={}",
            uefi_resolution
                .source
                .map(|source| source.to_string())
                .unwrap_or_else(|| "unresolved".to_string()),
            uefi_resolution.value.as_deref().unwrap_or("<none>")
        );
    }

    let looking_glass = crate::state::resolve_looking_glass_program_with_source(
        manager.config().options.looking_glass.as_ref(),
        central_config,
        runtime_overrides,
    );
    if let Ok(resolution) = looking_glass {
        println!(
            "  looking_glass_program: mode={:?}, source={}, value={}",
            resolution.mode,
            resolution
                .resolution
                .source
                .map(|source| source.to_string())
                .unwrap_or_else(|| "unresolved".to_string()),
            resolution.resolution.value.as_deref().unwrap_or("<none>")
        );
    }

    for network in &manager.config().devices.networks {
        let resolved =
            crate::state::resolve_network_outcome(&manager.config().name, network, central_config);
        println!(
            "  network[{}]: mode={:?}, source={}, backend={}",
            network.id,
            resolved.mode,
            resolved
                .source
                .map(|source| source.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            resolved
                .network
                .backend
                .as_ref()
                .map(|backend| backend.backend_type.as_str())
                .unwrap_or("none")
        );
    }
}

fn run_daemon_start(
    manager: &crate::qemu::QemuManager,
    executor: &crate::qemu::executor::QemuExecutor,
    log_file: Option<&std::path::PathBuf>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    println!("Starting in daemon mode...");
    if let Some(log_file) = log_file {
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

    if let Err(err) = spawn_remote_viewer(manager.config(), central_config, runtime_overrides) {
        eprintln!("Warning: {}", err);
    }
    if let Err(err) = spawn_looking_glass(manager.config(), central_config, runtime_overrides) {
        eprintln!("Warning: {}", err);
    }

    Ok(())
}

fn run_interactive_start(
    manager: &crate::qemu::QemuManager,
    executor: &crate::qemu::executor::QemuExecutor,
    log_file: Option<&std::path::PathBuf>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    println!("Starting interactively...");

    if let Err(err) = spawn_remote_viewer(manager.config(), central_config, runtime_overrides) {
        eprintln!("Warning: {}", err);
    }
    if let Err(err) = spawn_looking_glass(manager.config(), central_config, runtime_overrides) {
        eprintln!("Warning: {}", err);
    }

    let status = if let Some(log_file) = log_file {
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

    Ok(())
}
