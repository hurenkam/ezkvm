use anyhow::{Result, anyhow};

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
