use anyhow::Result;

pub(crate) async fn handle_stop(config_path: &str, force: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");

    println!("Stopping VM: {}", config.name);

    let mut lifecycle_state = crate::state::VmState::Running { pid: None };

    if force {
        lifecycle_state =
            lifecycle_state.transition(crate::state::VmStateEvent::ForceKillIssued)?;
        println!("Force stopping...");
        crate::qemu::process::kill_vm(&config.name)?;
        println!("✓ VM '{}' force stopped", config.name);
    } else {
        lifecycle_state =
            lifecycle_state.transition(crate::state::VmStateEvent::StopCommandIssued)?;
        println!("Gracefully stopping...");
        let qmp_socket = resolve_qmp_socket_path(&config);
        crate::qemu::process::stop_vm(&config.name, qmp_socket.as_deref())?;
        println!("✓ VM '{}' stopped", config.name);
    }

    lifecycle_state = lifecycle_state.transition(crate::state::VmStateEvent::ProcessExited)?;
    tracing::debug!(target: "ezkvm::lifecycle", vm = %config.name, state = ?lifecycle_state, "vm stop path completed");

    let _ = crate::state::delete_pid_at(&config.name, config.options.pid_file.as_deref());

    Ok(())
}

pub(crate) async fn handle_kill(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded");

    println!("Killing VM: {}", config.name);

    let mut lifecycle_state = crate::state::VmState::Running { pid: None };
    lifecycle_state = lifecycle_state.transition(crate::state::VmStateEvent::ForceKillIssued)?;

    crate::qemu::process::kill_vm(&config.name)?;
    println!("✓ VM '{}' killed", config.name);

    lifecycle_state = lifecycle_state.transition(crate::state::VmStateEvent::ProcessExited)?;
    tracing::debug!(target: "ezkvm::lifecycle", vm = %config.name, state = ?lifecycle_state, "vm kill path completed");

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

fn resolve_qmp_socket_path(config: &crate::config::VmConfig) -> Option<String> {
    if let Some(qmp) = config.options_qmp()
        && qmp.enabled
    {
        return match qmp.socket_type {
            crate::config::QmpSocketType::Unix => Some(
                qmp.socket_path
                    .clone()
                    .unwrap_or_else(|| "/var/run/qemu-monitor.sock".to_string()),
            ),
            crate::config::QmpSocketType::Tcp => None,
        };
    }

    let central = crate::config::CentralConfig::load().unwrap_or_default();
    let overrides = crate::config::RuntimeCliOverrides::default();
    let runtime_root = crate::state::resolve_runtime_root_with_source(None, &central, &overrides)
        .value
        .unwrap_or_else(|| "/tmp/ezkvm".to_string());

    Some(format!("{}/{}.qmp", runtime_root, config.name))
}
