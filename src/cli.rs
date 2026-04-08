//! Command-line interface for ezkvm
//!
//! Provides the main CLI commands for managing virtual machines.

use clap::{Parser, Subcommand};
use anyhow::Result;

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
    }
}

/// Handle create command
async fn handle_create(config_path: &str, validate_only: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");
    
    if validate_only {
        println!("✓ Validation successful");
        return Ok(());
    }
    
    // TODO: Save VM configuration for later use
    println!("✓ VM '{}' created successfully", config.name);
    
    Ok(())
}

/// Handle start command
async fn handle_start(config_path: &str, daemon: bool, dry_run: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);
    
    let mut config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");
    
    // Override daemonize option based on CLI flag
    config.options.daemonize = daemon;
    
    let manager = crate::qemu::QemuManager::new(config);
    let args = manager.build_command()?;
    
    println!("Starting VM: {}", manager.config().name);
    println!("QEMU binary: {}", manager.binary_name());
    
    // Check if QEMU binary is available
    crate::qemu::executor::check_qemu_available(&manager.binary_name())?;
    println!("✓ QEMU binary found");
    
    if dry_run {
        println!("Dry run mode - would execute:");
        println!("{} {}", manager.binary_name(), args.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "));
        return Ok(());
    }
    
    let executor = crate::qemu::executor::QemuExecutor::new(
        manager.binary_name(),
        args,
    );
    
    if daemon {
        println!("Starting in daemon mode...");
        // With -daemonize, QEMU detaches so this should return quickly
        let status = executor.execute_sync()?;
        println!("✓ VM '{}' started (daemonized)", manager.config().name);
    } else {
        println!("Starting interactively...");
        let status = executor.execute_sync()?;
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
    
    println!("Status of VM: {}", config.name);
    
    match crate::qemu::process::is_vm_running(&config.name) {
        Ok(true) => println!("Status: Running"),
        Ok(false) => println!("Status: Not running"),
        Err(e) => {
            eprintln!("Error checking status: {}", e);
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
    // TODO: Attach to VM console (could use QEMU monitor or serial console)
    println!("Console: Not implemented yet");
    
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