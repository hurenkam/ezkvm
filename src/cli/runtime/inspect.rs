use anyhow::{Result, anyhow};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestNetworkInterface {
    name: String,
    addresses: Vec<String>,
}

fn guest_agent_socket_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Option<String> {
    let guest_agent = config.options_guest_agent()?;
    if !guest_agent.enabled {
        return None;
    }

    if let Some(socket_path) = guest_agent.socket_path.clone() {
        return Some(socket_path);
    }

    Some(
        crate::state::resolve_runtime_guest_agent_socket(
            &config.name,
            central_config,
            &crate::config::RuntimeCliOverrides::default(),
        )
        .unwrap_or_else(|_| format!("/tmp/ezkvm/{}.qga", config.name)),
    )
}

fn parse_guest_network_interfaces(response: &Value) -> Vec<GuestNetworkInterface> {
    let Some(interfaces) = response.get("return").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut parsed = Vec::new();
    for iface in interfaces {
        let Some(name) = iface.get("name").and_then(Value::as_str) else {
            continue;
        };

        let mut addresses = Vec::new();
        if let Some(ip_entries) = iface.get("ip-addresses").and_then(Value::as_array) {
            for ip in ip_entries {
                let Some(ip_addr) = ip.get("ip-address").and_then(Value::as_str) else {
                    continue;
                };

                let prefix = ip
                    .get("prefix")
                    .and_then(Value::as_u64)
                    .map(|v| format!("/{}", v))
                    .unwrap_or_default();
                let family = ip
                    .get("ip-address-type")
                    .and_then(Value::as_str)
                    .map(|v| format!(" ({})", v))
                    .unwrap_or_default();

                addresses.push(format!("{}{}{}", ip_addr, prefix, family));
            }
        }

        if addresses.is_empty() {
            continue;
        }

        parsed.push(GuestNetworkInterface {
            name: name.to_string(),
            addresses,
        });
    }

    parsed.sort_by(|a, b| a.name.cmp(&b.name));
    parsed
}

fn guest_network_interfaces_from_socket(socket_path: &str) -> Result<Vec<GuestNetworkInterface>> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| {
        anyhow!(
            "failed to connect to guest agent socket '{}': {}",
            socket_path,
            e
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_millis(1200)))
        .map_err(|e| anyhow!("failed to configure guest agent read timeout: {}", e))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1200)))
        .map_err(|e| anyhow!("failed to configure guest agent write timeout: {}", e))?;

    let command = "{\"execute\":\"guest-network-get-interfaces\"}\n";
    stream
        .write_all(command.as_bytes())
        .map_err(|e| anyhow!("failed to send guest agent command: {}", e))?;
    stream
        .flush()
        .map_err(|e| anyhow!("failed to flush guest agent command: {}", e))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| anyhow!("failed reading guest agent response: {}", e))?;
        if bytes == 0 {
            return Ok(Vec::new());
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let payload: Value = serde_json::from_str(trimmed)
            .map_err(|e| anyhow!("invalid guest agent JSON response '{}': {}", trimmed, e))?;

        if let Some(err) = payload.get("error") {
            let class = err
                .get("class")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let desc = err
                .get("desc")
                .and_then(Value::as_str)
                .unwrap_or("unknown guest-agent error");
            return Err(anyhow!(
                "guest agent command guest-network-get-interfaces failed ({}): {}",
                class,
                desc
            ));
        }

        if payload.get("return").is_some() {
            return Ok(parse_guest_network_interfaces(&payload));
        }
    }
}

fn qmp_socket_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Option<String> {
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

    let overrides = crate::config::RuntimeCliOverrides::default();
    let runtime_root =
        crate::state::resolve_runtime_root_with_source(None, central_config, &overrides)
            .value
            .unwrap_or_else(|| "/tmp/ezkvm".to_string());

    Some(format!("{}/{}.qmp", runtime_root, config.name))
}

fn qmp_socket_path_from_pid(pid: i32) -> Option<String> {
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let cmdline = std::fs::read(cmdline_path).ok()?;
    find_qmp_socket_path_from_cmdline(&cmdline)
}

fn find_qmp_socket_path_from_cmdline(cmdline: &[u8]) -> Option<String> {
    let args: Vec<String> = cmdline
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect();

    for pair in args.windows(2) {
        if pair[0] == "-qmp"
            && let Some(path) = parse_qmp_socket_path(&pair[1])
        {
            return Some(path);
        }
    }

    None
}

fn parse_qmp_socket_path(spec: &str) -> Option<String> {
    if let Some(rest) = spec.strip_prefix("unix:") {
        return Some(rest.split(',').next().unwrap_or_default().to_string());
    }

    None
}

fn qmp_status_from_socket(socket_path: &str) -> Result<String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| anyhow!("failed to connect to QMP socket '{}': {}", socket_path, e))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(1200)))
        .map_err(|e| anyhow!("failed to configure QMP read timeout: {}", e))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1200)))
        .map_err(|e| anyhow!("failed to configure QMP write timeout: {}", e))?;

    let reader_stream = stream
        .try_clone()
        .map_err(|e| anyhow!("failed to clone QMP socket stream: {}", e))?;
    let mut reader = BufReader::new(reader_stream);

    let greeting = read_qmp_json_with_retry(&mut reader, "QMP greeting")?;
    if greeting.get("QMP").is_none() {
        return Err(anyhow!("invalid QMP greeting payload: {}", greeting));
    }

    stream
        .write_all(b"{\"execute\":\"qmp_capabilities\"}\n")
        .map_err(|e| anyhow!("failed to send qmp_capabilities: {}", e))?;
    stream
        .flush()
        .map_err(|e| anyhow!("failed to flush qmp_capabilities: {}", e))?;

    loop {
        let msg = read_qmp_json_with_retry(&mut reader, "qmp_capabilities response")?;
        if msg.get("return").is_some() {
            break;
        }
        if let Some(err) = msg.get("error") {
            return Err(anyhow!("qmp_capabilities failed: {}", err));
        }
    }

    stream
        .write_all(b"{\"execute\":\"query-status\"}\n")
        .map_err(|e| anyhow!("failed to send query-status: {}", e))?;
    stream
        .flush()
        .map_err(|e| anyhow!("failed to flush query-status: {}", e))?;

    loop {
        let msg = read_qmp_json_with_retry(&mut reader, "query-status response")?;
        if let Some(ret) = msg.get("return") {
            let status = ret
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let running = ret
                .get("running")
                .and_then(Value::as_bool)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Ok(format!("status={}, running={}", status, running));
        }
        if let Some(err) = msg.get("error") {
            return Err(anyhow!("query-status failed: {}", err));
        }
    }
}

fn qmp_status_from_socket_with_timeout(socket_path: &str, timeout: Duration) -> Result<String> {
    let (tx, rx) = mpsc::channel();
    let path = socket_path.to_string();
    std::thread::spawn(move || {
        let _ = tx.send(qmp_status_from_socket(&path));
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(anyhow!("timed out waiting for QMP status query"))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(anyhow!("QMP status worker exited unexpectedly"))
        }
    }
}

fn read_qmp_json_with_retry(reader: &mut BufReader<UnixStream>, context: &str) -> Result<Value> {
    const MAX_TIMEOUT_RETRIES: usize = 5;

    let mut retries = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                return Err(anyhow!("QMP socket closed while waiting for {}", context));
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                return serde_json::from_str(trimmed).map_err(|e| {
                    anyhow!(
                        "invalid QMP JSON while waiting for {} '{}': {}",
                        context,
                        trimmed,
                        e
                    )
                });
            }
            Err(err) if is_retryable_socket_timeout(&err) => {
                retries += 1;
                if retries > MAX_TIMEOUT_RETRIES {
                    return Err(anyhow!("timed out waiting for {}: {}", context, err));
                }
            }
            Err(err) => {
                return Err(anyhow!("failed reading {}: {}", context, err));
            }
        }
    }
}

fn is_retryable_socket_timeout(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

fn print_qmp_status_details(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    pid: Option<i32>,
) {
    let socket_path = pid
        .and_then(qmp_socket_path_from_pid)
        .or_else(|| qmp_socket_path(config, central_config));

    let Some(socket_path) = socket_path else {
        println!("QMP: unavailable (tcp socket mode)");
        return;
    };

    match qmp_status_from_socket_with_timeout(&socket_path, Duration::from_secs(2)) {
        Ok(status) => println!("QMP: {}", status),
        Err(err) => println!("QMP: unavailable at {} ({})", socket_path, err),
    }
}

fn print_guest_agent_network_details(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) {
    let Some(socket_path) = guest_agent_socket_path(config, central_config) else {
        return;
    };

    match guest_network_interfaces_from_socket(&socket_path) {
        Ok(interfaces) if interfaces.is_empty() => {
            println!("Guest Agent: running (no network addresses reported)");
        }
        Ok(interfaces) => {
            println!("Guest Agent: running");
            println!("Guest Network:");
            for iface in interfaces {
                println!("  {}", iface.name);
                for address in iface.addresses {
                    println!("    {}", address);
                }
            }
        }
        Err(err) => {
            println!("Guest Agent: unavailable ({})", err);
        }
    }
}

pub(crate) async fn handle_status(config_path: &str) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    let central_config = crate::config::CentralConfig::load().unwrap_or_default();
    println!("✓ Configuration loaded");

    let vm_name = &config.name;
    println!("Status of VM: {}", vm_name);
    let shutdown_marker =
        crate::state::get_shutdown_marker_file_at(vm_name, config.options.pid_file.as_deref())?;
    let shutdown_observed = crate::state::shutdown_marker_exists(&shutdown_marker);

    let mut lifecycle_state = crate::state::VmState::Stopped;

    if let Ok(Some(pid)) = crate::state::read_pid_at(vm_name, config.options.pid_file.as_deref()) {
        match crate::qemu::process::find_qemu_processes(vm_name) {
            Ok(pids) if pids.contains(&pid) => {
                lifecycle_state = lifecycle_state
                    .transition(crate::state::VmStateEvent::StartCommandIssued)?
                    .transition(crate::state::VmStateEvent::ProcessObserved { pid: Some(pid) })?;
                if shutdown_observed {
                    lifecycle_state = lifecycle_state
                        .transition(crate::state::VmStateEvent::GuestShutdownObserved)?;
                }
                println!("Status: {} (PID: {})", lifecycle_state.status_label(), pid);
                println!("Memory: {} MiB", config.system.memory.size);
                println!("vCPUs: {}", config.system.cpu.vcpus);
                if shutdown_observed {
                    println!("Shutdown: guest shutdown observed; waiting for QEMU to exit");
                }
                print_qmp_status_details(&config, &central_config, Some(pid));
                print_guest_agent_network_details(&config, &central_config);
                return Ok(());
            }
            _ => {
                let _ = crate::state::delete_pid_at(vm_name, config.options.pid_file.as_deref());
                let _ = crate::state::delete_shutdown_marker(&shutdown_marker);
            }
        }
    }

    match crate::qemu::process::is_vm_running(vm_name) {
        Ok(is_running) => {
            if is_running {
                lifecycle_state = lifecycle_state
                    .transition(crate::state::VmStateEvent::StartCommandIssued)?
                    .transition(crate::state::VmStateEvent::ProcessObserved { pid: None })?;
                if shutdown_observed {
                    lifecycle_state = lifecycle_state
                        .transition(crate::state::VmStateEvent::GuestShutdownObserved)?;
                }
                println!("Status: {}", lifecycle_state.status_label());
                println!("Memory: {} MiB", config.system.memory.size);
                println!("vCPUs: {}", config.system.cpu.vcpus);
                if shutdown_observed {
                    println!("Shutdown: guest shutdown observed; waiting for QEMU to exit");
                }
                let pid = crate::qemu::process::find_qemu_processes(vm_name)
                    .ok()
                    .and_then(|pids| pids.into_iter().next());
                print_qmp_status_details(&config, &central_config, pid);
                print_guest_agent_network_details(&config, &central_config);
            } else {
                let _ = crate::state::delete_shutdown_marker(&shutdown_marker);
                println!("Status: {}", lifecycle_state.status_label());
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

pub(crate) async fn handle_validate(
    config_path: &str,
    show_resolved_config: bool,
    show_machine_layout: bool,
) -> Result<()> {
    println!("Validating configuration: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration is valid");
    println!("VM Name: {}", config.name);
    println!("Architecture: {}", config.system.architecture);
    println!("Memory: {} MiB", config.system.memory.size);
    println!("vCPUs: {}", config.system.cpu.vcpus);

    if show_machine_layout {
        let central_config = crate::config::CentralConfig::load()?;
        let manager = crate::qemu::QemuManager::new_with_overrides(
            config.clone(),
            central_config,
            crate::config::RuntimeCliOverrides::default(),
        );
        let args = manager.build_command()?;
        let layout = crate::qemu::topology::render_machine_layout(
            &config.system.machine,
            &args,
            &config.system.readconfig,
        );
        println!("\nMachine layout:");
        println!("{}", layout);
    }

    if show_resolved_config {
        println!("\nResolved configuration:");
        let resolved_yaml = serde_yaml::to_string(&config)?;
        print!("{}", resolved_yaml);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        find_qmp_socket_path_from_cmdline, guest_agent_socket_path, parse_guest_network_interfaces,
        parse_qmp_socket_path,
    };

    #[test]
    fn parse_guest_network_interfaces_extracts_addresses() {
        let response = serde_json::json!({
            "return": [
                {
                    "name": "eth0",
                    "ip-addresses": [
                        {
                            "ip-address": "192.168.10.12",
                            "ip-address-type": "ipv4",
                            "prefix": 24
                        },
                        {
                            "ip-address": "2001:db8::10",
                            "ip-address-type": "ipv6",
                            "prefix": 64
                        }
                    ]
                },
                {
                    "name": "lo",
                    "ip-addresses": [
                        {
                            "ip-address": "127.0.0.1",
                            "ip-address-type": "ipv4",
                            "prefix": 8
                        }
                    ]
                }
            ]
        });

        let interfaces = parse_guest_network_interfaces(&response);
        assert_eq!(interfaces.len(), 2);
        assert_eq!(interfaces[0].name, "eth0");
        assert_eq!(interfaces[0].addresses[0], "192.168.10.12/24 (ipv4)");
        assert_eq!(interfaces[0].addresses[1], "2001:db8::10/64 (ipv6)");
        assert_eq!(interfaces[1].name, "lo");
        assert_eq!(interfaces[1].addresses[0], "127.0.0.1/8 (ipv4)");
    }

    #[test]
    fn guest_agent_socket_path_uses_default_when_socket_is_unset() {
        let config = crate::config::VmConfig::from_str(
            r#"
            name: "test-vm"
            backend: "qemu"

            system:
              architecture: "x86_64"
              machine: "q35"
              memory:
                size: 1024
              cpu:
                vcpus: 1
                model: "host"

            options:
              guest_agent:
                enabled: true
            "#,
        )
        .unwrap();

        let central_config = crate::config::CentralConfig {
            locations: crate::config::LocationsConfig {
                run_dir: Some("/run/ezkvm".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            guest_agent_socket_path(&config, &central_config).as_deref(),
            Some("/run/ezkvm/test-vm.qga")
        );
    }

    #[test]
    fn parse_qmp_socket_path_extracts_unix_path() {
        let path = parse_qmp_socket_path("unix:/tmp/vm.qmp,server=on,wait=off");
        assert_eq!(path.as_deref(), Some("/tmp/vm.qmp"));
    }

    #[test]
    fn find_qmp_socket_path_from_cmdline_extracts_qmp_argument() {
        let cmdline =
            b"qemu-system-x86_64\0-name\0vm\0-qmp\0unix:/run/ezkvm/vm.qmp,server=on,wait=off\0";
        let path = find_qmp_socket_path_from_cmdline(cmdline);
        assert_eq!(path.as_deref(), Some("/run/ezkvm/vm.qmp"));
    }
}
