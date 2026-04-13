use super::auxiliary::{
    build_looking_glass_launch, build_remote_viewer_launch, build_swtpm_launch_preview,
    ensure_runtime_socket_dirs, format_auxiliary_launch, spawn_looking_glass, spawn_remote_viewer,
    start_swtpm_if_configured,
};
use anyhow::Result;
use std::path::Path;
use std::process::Stdio;

pub(crate) async fn handle_start(config_path: &str, daemon: bool, dry_run: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let (mut config, central_config) = load_start_configs(config_path)?;
    prepare_auxiliary_runtime(&config, &central_config, dry_run)?;

    let central_config_clone = central_config.clone();
    config.options.daemonize = daemon;
    crate::state::cache_config(&config.name, &config)?;

    let pid_file = crate::state::get_pid_file_at(&config.name, config.options.pid_file.as_deref())?;
    let log_file = prepare_log_file(&config, daemon)?;

    let manager = crate::qemu::QemuManager::new(config, central_config);
    let args = manager.build_command()?;

    println!("Starting VM: {}", manager.config().name);
    println!("QEMU binary: {}", manager.binary_name());

    crate::qemu::executor::check_qemu_available(&manager.binary_name())?;
    println!("✓ QEMU binary found");

    if dry_run {
        print_dry_run(
            &manager,
            &args,
            &pid_file,
            log_file.as_ref(),
            &central_config_clone,
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
        )?;
    } else {
        run_interactive_start(
            &manager,
            &executor,
            log_file.as_ref(),
            &central_config_clone,
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
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    ensure_runtime_socket_dirs(config, central_config)?;
    start_swtpm_if_configured(config, central_config)?;
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

    match build_swtpm_launch_preview(manager.config(), central_config) {
        Ok(Some(cmd)) => println!("Auxiliary launch (swtpm): {}", cmd),
        Ok(None) => {}
        Err(err) => println!("swtpm configuration error: {}", err),
    }

    if let Some(launch) = build_remote_viewer_launch(manager.config(), central_config) {
        println!(
            "Auxiliary launch (SPICE): {}",
            format_auxiliary_launch(&launch)
        );
    }

    match build_looking_glass_launch(manager.config(), central_config) {
        Ok(Some(launch)) => {
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
        Ok(None) => {}
        Err(err) => println!("Looking Glass configuration error: {}", err),
    }
}

fn run_daemon_start(
    manager: &crate::qemu::QemuManager,
    executor: &crate::qemu::executor::QemuExecutor,
    log_file: Option<&std::path::PathBuf>,
    central_config: &crate::config::CentralConfig,
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

    if let Err(err) = spawn_remote_viewer(manager.config(), central_config) {
        eprintln!("Warning: {}", err);
    }
    if let Err(err) = spawn_looking_glass(manager.config(), central_config) {
        eprintln!("Warning: {}", err);
    }

    Ok(())
}

fn run_interactive_start(
    manager: &crate::qemu::QemuManager,
    executor: &crate::qemu::executor::QemuExecutor,
    log_file: Option<&std::path::PathBuf>,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    println!("Starting interactively...");

    if let Err(err) = spawn_remote_viewer(manager.config(), central_config) {
        eprintln!("Warning: {}", err);
    }
    if let Err(err) = spawn_looking_glass(manager.config(), central_config) {
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
