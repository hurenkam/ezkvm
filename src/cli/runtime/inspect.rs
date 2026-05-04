use anyhow::{Result, anyhow};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuestNetworkInterface {
    name: String,
    addresses: Vec<String>,
}

fn guest_agent_socket_path(config: &crate::config::VmConfig) -> Option<String> {
    let guest_agent = config.options_guest_agent()?;
    if !guest_agent.enabled {
        return None;
    }

    Some(
        guest_agent
            .socket_path
            .clone()
            .unwrap_or_else(|| "/var/run/qemu-server/qga.sock".to_string()),
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
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| anyhow!("failed to connect to guest agent socket '{}': {}", socket_path, e))?;
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

fn print_guest_agent_network_details(config: &crate::config::VmConfig) {
    let Some(socket_path) = guest_agent_socket_path(config) else {
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
    println!("✓ Configuration loaded");

    let vm_name = &config.name;
    println!("Status of VM: {}", vm_name);

    if let Ok(Some(pid)) = crate::state::read_pid_at(vm_name, config.options.pid_file.as_deref()) {
        match crate::qemu::process::find_qemu_processes(vm_name) {
            Ok(pids) if pids.contains(&pid) => {
                println!("Status: Running (PID: {})", pid);
                println!("Memory: {} MiB", config.system.memory.size);
                println!("vCPUs: {}", config.system.cpu.vcpus);
                print_guest_agent_network_details(&config);
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
                println!("Memory: {} MiB", config.system.memory.size);
                println!("vCPUs: {}", config.system.cpu.vcpus);
                print_guest_agent_network_details(&config);
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

#[cfg(test)]
mod tests {
    use super::{guest_agent_socket_path, parse_guest_network_interfaces};

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

        assert_eq!(
            guest_agent_socket_path(&config).as_deref(),
            Some("/var/run/qemu-server/qga.sock")
        );
    }
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
    println!("Memory: {} MiB", config.system.memory.size);
    println!("vCPUs: {}", config.system.cpu.vcpus);

    if show_resolved_config {
        println!("\nResolved configuration:");
        let resolved_yaml = serde_yaml::to_string(&config)?;
        print!("{}", resolved_yaml);
    }

    Ok(())
}
