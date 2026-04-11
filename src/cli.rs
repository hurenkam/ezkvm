//! Command-line interface for ezkvm
//!
//! Provides the main CLI commands for managing virtual machines.

use clap::{Parser, Subcommand};
use anyhow::{anyhow, Result};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// ezkvm - Easy KVM virtual machine manager
#[derive(Parser)]
#[command(name = "ezkvm")]
#[command(about = "A simple KVM virtual machine manager using YAML configuration")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create and validate a VM configuration
    Create {
        /// Path to the YAML configuration file
        config: String,
        
        /// Validate only, don't create
        #[arg(long)]
        validate_only: bool,
    },
    
    /// Start a virtual machine
    Start {
        /// Path to the YAML configuration file
        config: String,
        
        /// Run in background (daemon mode)
        #[arg(short, long)]
        daemon: bool,
        
        /// Dry run - show command without executing
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Stop a virtual machine
    Stop {
        /// Path to the YAML configuration file
        config: String,
        
        /// Force stop (SIGKILL)
        #[arg(short, long)]
        force: bool,
    },
    
    /// Kill a virtual machine forcefully
    Kill {
        /// Path to the YAML configuration file
        config: String,
    },
    
    /// List running virtual machines
    List,
    
    /// Show status of a virtual machine
    Status {
        /// Path to the YAML configuration file
        config: String,
    },
    
    /// Attach to VM console
    Console {
        /// Path to the YAML configuration file
        config: String,
    },
    
    /// Validate a configuration file
    Validate {
        /// Path to the YAML configuration file
        config: String,
    },
    
    /// Storage management commands
    #[command(subcommand)]
    Storage(StorageCommands),
    
    /// Device management commands
    #[command(subcommand)]
    Device(DeviceCommands),
    
    /// Network management commands
    #[command(subcommand)]
    Network(NetworkCommands),
}

#[derive(Subcommand)]
pub enum StorageCommands {
    /// Create a QCOW2 disk image
    Create {
        /// Name of the disk image
        name: String,
        /// Size in GB
        #[arg(short, long)]
        size: u32,
    },
    
    /// List all disk images
    List,
    
    /// Show disk image information
    Info {
        /// Name or path of the disk image
        disk: String,
    },
    
    /// Resize a disk image
    Resize {
        /// Name of the disk image
        disk: String,
        /// New size in GB
        #[arg(short, long)]
        size: u32,
    },
    
    /// Create a snapshot of a disk
    Snapshot {
        /// Name of the disk image
        disk: String,
        /// Snapshot name
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum DeviceCommands {
    /// List available USB devices
    Usb {
        #[command(subcommand)]
        cmd: Option<UsbCommands>,
    },
    
    /// List available PCI devices
    Pci {
        #[command(subcommand)]
        cmd: Option<PciCommands>,
    },
}

#[derive(Subcommand)]
pub enum UsbCommands {
    /// List USB devices
    List,
}

#[derive(Subcommand)]
pub enum PciCommands {
    /// List PCI devices suitable for passthrough
    List,
}

#[derive(Subcommand)]
pub enum NetworkCommands {
    /// Create a network bridge
    Bridge {
        /// Name of the bridge
        name: String,
    },
}

/// Execute the CLI command
pub async fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Create { config, validate_only } => {
            handle_create(&config, validate_only).await
        }
        Commands::Start { config, daemon, dry_run } => {
            handle_start(&config, daemon, dry_run).await
        }
        Commands::Stop { config, force } => {
            handle_stop(&config, force).await
        }
        Commands::Kill { config } => {
            handle_kill(&config).await
        }
        Commands::List => {
            handle_list().await
        }
        Commands::Status { config } => {
            handle_status(&config).await
        }
        Commands::Console { config } => {
            handle_console(&config).await
        }
        Commands::Validate { config } => {
            handle_validate(&config).await
        }
        Commands::Storage(cmd) => {
            handle_storage(cmd).await
        }
        Commands::Device(cmd) => {
            handle_device(cmd).await
        }
        Commands::Network(cmd) => {
            handle_network(cmd).await
        }
    }
}

/// Handle create command
async fn handle_create(config_path: &str, validate_only: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");
    println!("\nVM Details:");
    println!("  Name: {}", config.name);
    println!("  Architecture: {}", config.system.architecture);
    println!("  Machine: {}", config.system.machine);
    println!("  Memory: {} MiB", config.system.memory);
    println!("  vCPUs: {}", config.system.vcpus);
    println!("  Devices:");
    println!("    Drives: {}", config.devices.drives.len());
    println!("    Networks: {}", config.devices.networks.len());
    println!("    Displays: {}", config.devices.displays.len());
    
    if validate_only {
        println!("\n✓ Validation successful");
        return Ok(());
    }
    
    // Cache the VM configuration for later use
    crate::state::cache_config(&config.name, &config)?;
    println!("\n✓ VM '{}' configuration cached", config.name);
    let state_dir = crate::state::get_state_dir()?;
    println!("Configuration saved to: {}", state_dir.display());
    
    Ok(())
}

fn ensure_run_dir(central_config: &crate::config::CentralConfig) -> Result<PathBuf> {
    let run_dir = central_config.locations.run_dir.as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/run/ezkvm"));

    std::fs::create_dir_all(&run_dir)?;
    Ok(run_dir)
}

fn ensure_socket_parent_dir(socket_path: &str, label: &str) -> Result<()> {
    let parent = Path::new(socket_path)
        .parent()
        .ok_or_else(|| anyhow!("{} '{}' does not have a parent directory", label, socket_path))?;

    std::fs::create_dir_all(parent)
        .map_err(|err| anyhow!("failed to create parent directory for {} '{}': {}", label, socket_path, err))
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
        Some(err) => Err(anyhow!("timed out waiting for {} '{}' to become ready: {}", label, socket_path, err)),
        None => Err(anyhow!("timed out waiting for {} '{}' to become ready", label, socket_path)),
    }
}

fn ensure_runtime_socket_dirs(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> Result<()> {
    if let Some(tpm) = &config.tpm {
        if tpm.backend == "emulator" {
            ensure_socket_parent_dir(&resolve_tpm_socket_path(config, central_config), "TPM socket")?;
        }
    }

    if let Some(guest_agent) = &config.guest_agent {
        if guest_agent.enabled {
            if let Some(socket_path) = guest_agent.socket_path.as_deref() {
                ensure_socket_parent_dir(socket_path, "guest agent socket")?;
            }
        }
    }

    if let Some(qmp) = &config.qmp {
        if qmp.enabled {
            if let crate::config::QmpSocketType::Unix = qmp.socket_type {
                if let Some(socket_path) = qmp.socket_path.as_deref() {
                    ensure_socket_parent_dir(socket_path, "QMP socket")?;
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct AuxiliaryLaunch {
    label: &'static str,
    program: String,
    args: Vec<String>,
}

fn format_auxiliary_launch(launch: &AuxiliaryLaunch) -> String {
    if launch.args.is_empty() {
        launch.program.clone()
    } else {
        format!("{} {}", launch.program, launch.args.join(" "))
    }
}

fn run_auxiliary_launch(launch: &AuxiliaryLaunch) -> Result<()> {
    let mut cmd = Command::new(&launch.program);
    cmd.args(&launch.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    cmd.spawn()
        .map_err(|err| anyhow!("failed to start {} at '{}': {}", launch.label, launch.program, err))?;

    println!("✓ Started {}", launch.label);
    Ok(())
}

fn resolve_tpm_socket_path(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> String {
    if let Some(tpm) = &config.tpm {
        if let Some(state_path) = &tpm.state_path {
            return state_path.clone();
        }
    }

    if let Some(run_dir) = &central_config.locations.run_dir {
        return format!("{}/tpm", run_dir);
    }

    "/var/run/qemu-server/tpm".to_string()
}

fn build_remote_viewer_launch(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> Option<AuxiliaryLaunch> {
    let spice = match &config.spice {
        Some(spice) if spice.enabled => spice,
        _ => return None,
    };

    let remote_viewer_path = central_config.tools.remote_viewer.as_ref()?;
    let uri = format!("spice://{}:{}", spice.addr, spice.port);

    Some(AuxiliaryLaunch {
        label: "remote-viewer for SPICE session",
        program: remote_viewer_path.clone(),
        args: vec![uri],
    })
}

fn build_looking_glass_launch(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> Result<Option<AuxiliaryLaunch>> {
    let ivshmem = match &config.ivshmem {
        Some(ivshmem) if ivshmem.enabled => ivshmem,
        _ => return Ok(None),
    };

    let looking_glass_path = match &central_config.tools.looking_glass {
        Some(path) if !path.trim().is_empty() => path,
        Some(_) => return Err(anyhow!("Looking Glass client path is empty")),
        None => return Ok(None),
    };

    Ok(Some(AuxiliaryLaunch {
        label: "Looking Glass client for ivshmem session",
        program: looking_glass_path.clone(),
        args: vec![format!("app:shmFile={}", ivshmem.mem_path)],
    }))
}

fn start_swtpm_if_configured(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> Result<()> {
    let tpm = match &config.tpm {
        Some(tpm) if tpm.backend == "emulator" => tpm,
        _ => return Ok(()),
    };

    let swtpm_path = match &central_config.tools.swtpm {
        Some(path) => path,
        None => return Err(anyhow!("TPM emulator backend requires tools.swtpm to be configured in the central config")),
    };

    let run_dir = ensure_run_dir(central_config)?;
    let socket_path = resolve_tpm_socket_path(config, central_config);
    ensure_socket_parent_dir(&socket_path, "TPM socket")?;
    let state_dir = run_dir.join("tpm-state");
    let ctrl_path = run_dir.join("swtpm-ctrl.sock");

    std::fs::create_dir_all(&state_dir)?;

    let mut cmd = Command::new(swtpm_path);
    cmd.arg("socket")
        .arg(if tpm.version == "2.0" { "--tpm2" } else { "--tpm" })
        .arg("--tpmstate")
        .arg(format!("dir={}", state_dir.display()))
        .arg("--ctrl")
        .arg(format!("type=unixio,path={}", ctrl_path.display()))
        .arg("--server")
        .arg(format!("type=unixio,path={}", socket_path))
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Err(err) = cmd.spawn() {
        println!("Warning: failed to start swtpm at '{}': {}", swtpm_path, err);
    } else {
        wait_for_unix_socket(&socket_path, Duration::from_secs(3), "swtpm socket")?;
        println!("✓ Started swtpm emulator using socket {}", socket_path);
    }

    Ok(())
}

fn spawn_remote_viewer(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> Result<()> {
    if let Some(launch) = build_remote_viewer_launch(config, central_config) {
        run_auxiliary_launch(&launch)?;
    }

    Ok(())
}

fn spawn_looking_glass(config: &crate::config::VmConfig, central_config: &crate::config::CentralConfig) -> Result<()> {
    let Some(launch) = build_looking_glass_launch(config, central_config)? else {
        return Ok(());
    };

    if !std::path::Path::new(&config.ivshmem.as_ref().unwrap().mem_path).exists() {
        return Err(anyhow!(
            "Looking Glass shared memory path '{}' does not exist",
            config.ivshmem.as_ref().unwrap().mem_path
        ));
    }

    run_auxiliary_launch(&launch)?;

    Ok(())
}

/// Handle start command
async fn handle_start(config_path: &str, daemon: bool, dry_run: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let mut config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");
    
    // Load central configuration
    let central_config = crate::config::CentralConfig::load()?;
    println!("✓ Central configuration loaded");

    if !dry_run {
        ensure_runtime_socket_dirs(&config, &central_config)?;

        // Optional service startup for TPM and viewer tools
        start_swtpm_if_configured(&config, &central_config)?;
    }

    let central_config_clone = central_config.clone();

    // Override daemonize option based on CLI flag
    config.options.daemonize = daemon;
    
    // Cache the configuration for quick restarts
    crate::state::cache_config(&config.name, &config)?;

    let pid_file = crate::state::get_pid_file_at(&config.name, config.options.pid_file.as_deref())?;
    let log_file = if daemon || config.options.log_dir.is_some() {
        crate::state::cleanup_old_logs_at(&config.name, config.options.log_dir.as_deref(), config.options.log_keep)?;
        Some(crate::state::create_session_log_file(&config.name, config.options.log_dir.as_deref())?)
    } else {
        None
    };
    
    let manager = crate::qemu::QemuManager::new(config, central_config);
    let args = manager.build_command()?;
    
    println!("Starting VM: {}", manager.config().name);
    println!("QEMU binary: {}", manager.binary_name());
    
    // Check if QEMU binary is available
    crate::qemu::executor::check_qemu_available(&manager.binary_name())?;
    println!("✓ QEMU binary found");
    
    if dry_run {
        println!("Dry run mode - would execute:");
        println!("{} {}", manager.binary_name(), args.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "));
        println!("PID file: {}", pid_file.display());
        if let Some(log_file) = &log_file {
            println!("Log file: {}", log_file.display());
        }

        if let Some(launch) = build_remote_viewer_launch(manager.config(), &central_config_clone) {
            println!("Auxiliary launch (SPICE): {}", format_auxiliary_launch(&launch));
        }

        match build_looking_glass_launch(manager.config(), &central_config_clone) {
            Ok(Some(launch)) => {
                println!("Auxiliary launch (Looking Glass): {}", format_auxiliary_launch(&launch));
                if !std::path::Path::new(&manager.config().ivshmem.as_ref().unwrap().mem_path).exists() {
                    println!("Looking Glass note: shared memory path '{}' does not exist on this host", manager.config().ivshmem.as_ref().unwrap().mem_path);
                }
            }
            Ok(None) => {}
            Err(err) => println!("Looking Glass configuration error: {}", err),
        }
        return Ok(());
    }
    
    let executor = crate::qemu::executor::QemuExecutor::new(
        manager.binary_name(),
        args,
    );
    
    if daemon {
        println!("Starting in daemon mode...");
        if let Some(log_file) = &log_file {
            println!("Logging QEMU output to {}", log_file.display());
            let _status = executor.execute_sync_logged(log_file, Stdio::null())?;
        } else {
            let _status = executor.execute_sync()?;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Ok(Some(pid)) = crate::state::read_pid_at(&manager.config().name, manager.config().options.pid_file.as_deref()) {
            println!("✓ VM '{}' started (daemonized) - PID {}", manager.config().name, pid);
        } else if let Ok(pids) = crate::qemu::process::find_qemu_processes(&manager.config().name) {
            if let Some(pid) = pids.first() {
                crate::state::save_pid_at(&manager.config().name, *pid, manager.config().options.pid_file.as_deref())?;
                println!("✓ VM '{}' started (daemonized) - PID {}", manager.config().name, pid);
            } else {
                println!("✓ VM '{}' started (daemonized)", manager.config().name);
            }
        } else {
            println!("✓ VM '{}' started (daemonized)", manager.config().name);
        }

        // Launch post-start viewer clients when available
        if let Err(err) = spawn_remote_viewer(manager.config(), &central_config_clone) {
            eprintln!("Warning: {}", err);
        }
        if let Err(err) = spawn_looking_glass(manager.config(), &central_config_clone) {
            eprintln!("Warning: {}", err);
        }
    } else {
        println!("Starting interactively...");

        // Launch viewer clients before interactive start so they can connect as soon as QEMU is ready
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
        // Clean up PID file for interactive mode
        let _ = crate::state::delete_pid_at(&manager.config().name, manager.config().options.pid_file.as_deref());
        println!("✓ VM '{}' finished with exit code {}", manager.config().name, status.code().unwrap_or(-1));
    }
    
    Ok(())
}

/// Handle stop command
async fn handle_stop(config_path: &str, force: bool) -> Result<()> {
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
    
    // Clean up PID file
    let _ = crate::state::delete_pid_at(&config.name, config.options.pid_file.as_deref());
    
    Ok(())
}

/// Handle kill command
async fn handle_kill(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");
    
    println!("Killing VM: {}", config.name);
    
    crate::qemu::process::kill_vm(&config.name)?;
    println!("✓ VM '{}' killed", config.name);
    
    // Clean up PID file
    let _ = crate::state::delete_pid_at(&config.name, config.options.pid_file.as_deref());
    
    Ok(())
}

/// Handle list command
async fn handle_list() -> Result<()> {
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

/// Handle status command
async fn handle_status(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");
    
    let vm_name = &config.name;
    println!("Status of VM: {}", vm_name);
    
    // First check if we have a PID file
    if let Ok(Some(pid)) = crate::state::read_pid_at(vm_name, config.options.pid_file.as_deref()) {
        // Verify the process still exists
        match crate::qemu::process::find_qemu_processes(vm_name) {
            Ok(pids) if pids.contains(&pid) => {
                println!("Status: Running (PID: {})", pid);
                println!("Memory: {} MiB", config.system.memory);
                println!("vCPUs: {}", config.system.vcpus);
                return Ok(());
            }
            _ => {
                // Process not found, clean up stale PID file
                let _ = crate::state::delete_pid_at(vm_name, config.options.pid_file.as_deref());
            }
        }
    }
    
    // Check if process is running (even if no PID file)
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

/// Handle console command
async fn handle_console(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");
    
    println!("Attaching to console of VM: {}", config.name);
    
    // Check if VM is running
    match crate::qemu::process::is_vm_running(&config.name) {
        Ok(is_running) if is_running => {
            // Try to connect via VNC (default QEMU VNC port)
            // QEMU provides VNC on port 5900 + display number
            println!("\nVM is running. Attempting VNC connection...");
            println!("VNC Server: localhost:5900");
            println!("\nYou can connect using:");
            println!("  vncviewer localhost:5900");
            println!("  or any other VNC client\n");
            
            // Try to open VNC client if available
            if let Ok(_) = std::process::Command::new("which")
                .arg("vncviewer")
                .output() {
                println!("Attempting to launch vncviewer...");
                let _ = std::process::Command::new("vncviewer")
                    .arg("localhost:5900")
                    .spawn();
            }
        }
        Ok(_) => {
            println!("\nError: VM '{}' is not running", config.name);
            println!("Start the VM first with: ezkvm start {}", config_path);
            return Err(anyhow::anyhow!("VM is not running"));
        }
        Err(e) => {
            eprintln!("Error checking VM status: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Handle validate command
async fn handle_validate(config_path: &str) -> Result<()> {
    println!("Validating configuration: {}", config_path);
    
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration is valid");
    println!("VM Name: {}", config.name);
    println!("Architecture: {}", config.system.architecture);
    println!("Memory: {} MiB", config.system.memory);
    println!("vCPUs: {}", config.system.vcpus);
    
    Ok(())
}

/// Handle storage commands
async fn handle_storage(cmd: StorageCommands) -> Result<()> {
    match cmd {
        StorageCommands::Create { name, size } => {
            println!("Creating storage image: {}", name);
            println!("Size: {} GB", size);
            
            // Create a QEMU disk image
            // qemu-img create -f qcow2 <path> <size>G
            let output = std::process::Command::new("qemu-img")
                .args(&["create", "-f", "qcow2", &name])
                .arg(format!("{}G", size))
                .output()?;
            
            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to create storage image: {}", err_msg));
            }
            
            println!("✓ Storage image created at: {}", name);
            Ok(())
        }
        StorageCommands::List => {
            println!("Available storage images:");
            
            // Try to list qcow2 images in common directories
            let home = std::env::var("HOME").unwrap_or_default();
            let home_storage = format!("{}/.ezkvm/storage", home);
            let common_paths = vec![
                ".",
                "./storage",
                "./images",
                &home_storage,
            ];
            
            for path in common_paths {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".qcow2") || name.ends_with(".img") {
                                println!("  {}", entry.path().display());
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        StorageCommands::Info { disk } => {
            println!("Getting information about disk: {}", disk);
            
            // Use qemu-img info to get disk information
            let output = std::process::Command::new("qemu-img")
                .args(&["info", &disk])
                .output()?;
            
            if output.status.success() {
                let info = String::from_utf8_lossy(&output.stdout);
                println!("{}", info);
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                println!("Error getting disk info: {}", err_msg);
            }
            Ok(())
        }
        StorageCommands::Resize { disk, size } => {
            println!("Resizing disk '{}' to {} GB", disk, size);
            
            // Use qemu-img resize to resize the disk
            let output = std::process::Command::new("qemu-img")
                .args(&["resize", &disk])
                .arg(format!("{}G", size))
                .output()?;
            
            if output.status.success() {
                println!("✓ Disk resized successfully");
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to resize disk: {}", err_msg));
            }
            Ok(())
        }
        StorageCommands::Snapshot { disk, name } => {
            println!("Creating snapshot '{}' of disk '{}'", name, disk);
            
            // Use qemu-img snapshot to create a snapshot
            let output = std::process::Command::new("qemu-img")
                .args(&["snapshot", "-c", &name, &disk])
                .output()?;
            
            if output.status.success() {
                println!("✓ Snapshot created successfully");
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to create snapshot: {}", err_msg));
            }
            Ok(())
        }
    }
}

/// Handle device commands
async fn handle_device(cmd: DeviceCommands) -> Result<()> {
    match cmd {
        DeviceCommands::Usb { cmd } => {
            match cmd {
                Some(UsbCommands::List) | None => {
                    println!("Available USB devices:");
                    
                    // Try to list USB devices using lsusb if available
                    if let Ok(output) = std::process::Command::new("lsusb").output() {
                        if output.status.success() {
                            let devices = String::from_utf8_lossy(&output.stdout);
                            print!("{}", devices);
                        } else {
                            println!("  (lsusb not available)");
                        }
                    } else {
                        println!("  (USB listing requires lsusb tool)");
                    }
                    Ok(())
                }
            }
        }
        DeviceCommands::Pci { cmd } => {
            match cmd {
                Some(PciCommands::List) | None => {
                    println!("Available PCI devices:");
                    
                    // Try to list PCI devices using lspci if available
                    if let Ok(output) = std::process::Command::new("lspci").output() {
                        if output.status.success() {
                            let devices = String::from_utf8_lossy(&output.stdout);
                            print!("{}", devices);
                        } else {
                            println!("  (lspci not available)");
                        }
                    } else {
                        println!("  (PCI listing requires lspci tool)");
                    }
                    Ok(())
                }
            }
        }
    }
}

/// Handle network commands
async fn handle_network(cmd: NetworkCommands) -> Result<()> {
    match cmd {
        NetworkCommands::Bridge { name } => {
            println!("Creating bridge: {}", name);
            println!("Note: This requires root/sudo privileges");
            
            // This would typically use `ip` or `brctl` commands
            println!("\nYou can create a bridge manually with:");
            println!("  sudo brctl addbr {}", name);
            println!("  sudo brctl addif {} <interface>", name);
            println!("  sudo ip addr add <ip>/<mask> dev {}", name);
            println!("  sudo ip link set {} up", name);
            
            Ok(())
        }
    }
}

            #[cfg(test)]
            mod tests {
                use super::*;

                fn base_config() -> crate::config::VmConfig {
                    crate::config::VmConfig::from_str(r#"
            name: "test-vm"
            backend: "qemu"

            system:
              architecture: "x86_64"
              machine: "q35"
              memory: 1024
              vcpus: 1
              cpu_model: "host"

            ivshmem:
              enabled: true
              size: 128
              id: "ivshmem0"
              mem_path: "/dev/kvmfr0"
            "#).unwrap()
                }

                #[test]
                fn test_build_looking_glass_launch_uses_ivshmem_mem_path() {
                    let config = base_config();
                    let central_config = crate::config::CentralConfig {
                        tools: crate::config::ToolsConfig {
                            swtpm: None,
                            remote_viewer: None,
                            looking_glass: Some("looking-glass-client".to_string()),
                        },
                        locations: crate::config::LocationsConfig::default(),
                    };

                    let launch = build_looking_glass_launch(&config, &central_config)
                        .unwrap()
                        .unwrap();

                    assert_eq!(launch.program, "looking-glass-client");
                    assert_eq!(launch.args, vec!["app:shmFile=/dev/kvmfr0"]);
                }

                #[test]
                fn test_build_looking_glass_launch_returns_none_without_tool() {
                    let config = base_config();
                    let central_config = crate::config::CentralConfig::default();

                    let launch = build_looking_glass_launch(&config, &central_config).unwrap();
                    assert!(launch.is_none());
                }

                #[test]
                fn test_build_looking_glass_launch_rejects_empty_tool_path() {
                    let config = base_config();
                    let central_config = crate::config::CentralConfig {
                        tools: crate::config::ToolsConfig {
                            swtpm: None,
                            remote_viewer: None,
                            looking_glass: Some("   ".to_string()),
                        },
                        locations: crate::config::LocationsConfig::default(),
                    };

                    let err = build_looking_glass_launch(&config, &central_config).unwrap_err();
                    assert!(err.to_string().contains("Looking Glass client path is empty"));
                }

                                #[test]
                                fn test_tpm_emulator_requires_swtpm_tool() {
                                        let config = crate::config::VmConfig::from_str(r#"
                        name: "test-vm"
                        backend: "qemu"

                        system:
                            architecture: "x86_64"
                            machine: "q35"
                            memory: 1024
                            vcpus: 1
                            cpu_model: "host"

                        tpm:
                            version: "2.0"
                            backend: "emulator"
                            model: "tpm-tis"
                        "#).unwrap();

                                        let err = start_swtpm_if_configured(&config, &crate::config::CentralConfig::default()).unwrap_err();
                                        assert!(err.to_string().contains("tools.swtpm"));
                                }

                #[test]
                fn test_format_auxiliary_launch() {
                    let launch = AuxiliaryLaunch {
                        label: "Looking Glass client for ivshmem session",
                        program: "looking-glass-client".to_string(),
                        args: vec!["app:shmFile=/dev/kvmfr0".to_string()],
                    };

                    assert_eq!(
                        format_auxiliary_launch(&launch),
                        "looking-glass-client app:shmFile=/dev/kvmfr0"
                    );
                }
            }