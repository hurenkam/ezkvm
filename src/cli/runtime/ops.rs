use anyhow::Result;

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
