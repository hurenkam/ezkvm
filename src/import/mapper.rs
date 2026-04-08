use super::proxmox_model::{ProxmoxDiskEntry, ProxmoxVmConfig};
use super::report::EzkvmImportResult;
use super::ImportError;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;

pub fn map_to_ezkvm_yaml(
    proxmox: &ProxmoxVmConfig,
    name_override: Option<&str>,
) -> Result<EzkvmImportResult, ImportError> {
    let mut mapped_keys = vec![];
    let mut skipped_keys = vec![];
    let mut warnings = vec![];

    let mut root = Mapping::new();

    let name = name_override
        .map(|x| x.to_string())
        .or_else(|| proxmox.scalars.get("name").cloned())
        .unwrap_or_else(|| "imported-vm".to_string());

    let mut general = Mapping::new();
    general.insert(s("name"), s(&name));
    mapped_keys.push("name".to_string());

    if let Some(uuid) = proxmox.scalars.get("uuid") {
        general.insert(s("uuid"), s(uuid));
        mapped_keys.push("uuid".to_string());
    }
    root.insert(s("general"), Value::Mapping(general));

    let mut system = Mapping::new();

    let mut cpu = Mapping::new();
    if let Some(cores) = proxmox.scalars.get("cores") {
        if let Ok(parsed) = cores.parse::<u64>() {
            cpu.insert(s("cores"), Value::Number(parsed.into()));
            mapped_keys.push("cores".to_string());
        } else {
            warnings.push(format!("unable to parse cores '{}'", cores));
            skipped_keys.push("cores".to_string());
        }
    }
    if let Some(sockets) = proxmox.scalars.get("sockets") {
        if let Ok(parsed) = sockets.parse::<u64>() {
            cpu.insert(s("sockets"), Value::Number(parsed.into()));
            mapped_keys.push("sockets".to_string());
        } else {
            warnings.push(format!("unable to parse sockets '{}'", sockets));
            skipped_keys.push("sockets".to_string());
        }
    }
    if let Some(model) = proxmox.scalars.get("cpu") {
        cpu.insert(s("model"), s(model));
        mapped_keys.push("cpu".to_string());
    }
    if !cpu.is_empty() {
        system.insert(s("cpu"), Value::Mapping(cpu));
    }

    if let Some(memory) = proxmox.scalars.get("memory") {
        if let Ok(parsed) = memory.parse::<u64>() {
            let mut memory_map = Mapping::new();
            memory_map.insert(s("max"), Value::Number(parsed.into()));
            system.insert(s("memory"), Value::Mapping(memory_map));
            mapped_keys.push("memory".to_string());
        } else {
            warnings.push(format!("unable to parse memory '{}'", memory));
            skipped_keys.push("memory".to_string());
        }
    }

    if let Some(machine) = proxmox.scalars.get("machine") {
        if machine.contains("q35") {
            let mut chipset = Mapping::new();
            chipset.insert(s("type"), s("q35"));
            system.insert(s("chipset"), Value::Mapping(chipset));
            mapped_keys.push("machine".to_string());
        } else {
            warnings.push(format!(
                "unsupported machine '{}'; keeping ezkvm default chipset",
                machine
            ));
            skipped_keys.push("machine".to_string());
        }
    }

    if let Some(bios) = proxmox.scalars.get("bios") {
        let mut bios_map = Mapping::new();
        let bios_type = if bios.eq_ignore_ascii_case("ovmf") {
            "ovmf"
        } else {
            "seabios"
        };
        bios_map.insert(s("type"), s(bios_type));
        system.insert(s("bios"), Value::Mapping(bios_map));
        mapped_keys.push("bios".to_string());
    }

    if !system.is_empty() {
        root.insert(s("system"), Value::Mapping(system));
    }

    let storage = map_storage(&proxmox.disks, &mut mapped_keys, &mut skipped_keys, &mut warnings);
    if !storage.is_empty() {
        root.insert(s("storage"), Value::Sequence(storage));
    }

    let network = map_network(proxmox, &mut mapped_keys, &mut skipped_keys, &mut warnings);
    if !network.is_empty() {
        root.insert(s("network"), Value::Sequence(network));
    }

    let host = map_host(proxmox, &mut mapped_keys, &mut skipped_keys, &mut warnings);
    if !host.is_empty() {
        root.insert(s("host"), Value::Mapping(host));
    }

    let yaml = serde_yaml::to_string(&Value::Mapping(root))
        .map_err(|e| ImportError::ValidationError(format!("yaml serialization failed: {}", e)))?;

    Ok(EzkvmImportResult {
        yaml,
        mapped_keys,
        skipped_keys,
        warnings,
    })
}

fn map_storage(
    disks: &[ProxmoxDiskEntry],
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Vec<Value> {
    let mut grouped: BTreeMap<String, Vec<&ProxmoxDiskEntry>> = BTreeMap::new();
    for disk in disks {
        grouped.entry(disk.bus.clone()).or_default().push(disk);
    }

    let mut controllers = vec![];
    for (bus, drives) in grouped {
        if bus == "virtio" {
            warnings.push("virtio disk bus is not yet a typed ezkvm controller; skipping virtio disks in phase 1".to_string());
            skipped_keys.push("virtio*".to_string());
            continue;
        }

        let controller_name = if bus == "scsi" { "pvscsi" } else { &bus };

        let mut controller = Mapping::new();
        controller.insert(s("controller"), s(controller_name));

        let mut drive_items = vec![];
        for drive in drives {
            let mut drive_map = Mapping::new();
            let media = drive.options.get("media").cloned().unwrap_or_default();
            let drive_type = if media == "cdrom" { "cd" } else { "hd" };

            drive_map.insert(s("type"), s(drive_type));
            drive_map.insert(s("file"), s(&drive.source));

            if let Some(discard) = drive.options.get("discard") {
                drive_map.insert(s("discard"), s(discard));
            }
            if let Some(cache) = drive.options.get("cache") {
                drive_map.insert(s("cache"), s(cache));
            }
            if let Some(boot) = drive.options.get("bootindex") {
                if let Ok(parsed) = boot.parse::<u64>() {
                    drive_map.insert(s("boot_index"), Value::Number(parsed.into()));
                }
            }

            if !drive.source.starts_with('/') {
                warnings.push(format!(
                    "disk '{}' source '{}' may require manual path translation",
                    drive.key, drive.source
                ));
            }

            mapped_keys.push(drive.key.clone());
            drive_items.push(Value::Mapping(drive_map));
        }

        controller.insert(s("drives"), Value::Sequence(drive_items));
        controllers.push(Value::Mapping(controller));
    }

    controllers
}

fn map_network(
    proxmox: &ProxmoxVmConfig,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Vec<Value> {
    let mut items = vec![];

    for net in &proxmox.networks {
        let Some(bridge) = net.options.get("bridge") else {
            warnings.push(format!(
                "network '{}' has no bridge option; skipped in phase 1",
                net.key
            ));
            skipped_keys.push(net.key.clone());
            continue;
        };

        let mut item = Mapping::new();
        item.insert(s("type"), s("bridge"));
        item.insert(s("bridge"), s(bridge));

        let driver = normalize_net_driver(&net.model, warnings);
        item.insert(s("driver"), s(&driver));

        if let Some(mac) = &net.mac {
            item.insert(s("mac"), s(mac));
        }

        mapped_keys.push(net.key.clone());
        items.push(Value::Mapping(item));
    }

    items
}

fn normalize_net_driver(model: &str, warnings: &mut Vec<String>) -> String {
    match model.to_ascii_lowercase().as_str() {
        "virtio" | "virtio-net-pci" => "virtio-net-pci".to_string(),
        "e1000" => "e1000".to_string(),
        "e1000e" => "e1000e".to_string(),
        other => {
            warnings.push(format!(
                "network model '{}' is not directly mapped; using virtio-net-pci",
                other
            ));
            "virtio-net-pci".to_string()
        }
    }
}

fn map_host(
    proxmox: &ProxmoxVmConfig,
    mapped_keys: &mut Vec<String>,
    _skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Mapping {
    let mut host = Mapping::new();

    if !proxmox.host_pci.is_empty() {
        let mut pci_items = vec![];
        for item in &proxmox.host_pci {
            let mut map = Mapping::new();
            map.insert(s("vm_id"), s(&format!("{:x}", item.index + 1)));
            map.insert(s("host_id"), s(&ensure_pci_function(&item.host)));

            if let Some(mf) = item.options.get("multifunction") {
                let parsed = mf == "1" || mf.eq_ignore_ascii_case("on") || mf == "true";
                map.insert(s("multi_function"), Value::Bool(parsed));
            }

            mapped_keys.push(item.key.clone());
            pci_items.push(Value::Mapping(map));
        }
        host.insert(s("pci"), Value::Sequence(pci_items));
    }

    if !proxmox.usb.is_empty() {
        let mut usb_items = vec![];
        for item in &proxmox.usb {
            if let Some(host) = item.options.get("host") {
                if let Some((bus, port)) = host.split_once('-') {
                    let mut map = Mapping::new();
                    map.insert(s("vm_port"), s(&(item.index + 1).to_string()));
                    map.insert(s("host_bus"), s(bus));
                    map.insert(s("host_port"), s(port));

                    mapped_keys.push(item.key.clone());
                    usb_items.push(Value::Mapping(map));
                } else {
                    warnings.push(format!(
                        "usb '{}' host format '{}' not recognized, expected <bus>-<port>",
                        item.key, host
                    ));
                }
            } else {
                warnings.push(format!("usb '{}' missing host option", item.key));
            }
        }

        if !usb_items.is_empty() {
            host.insert(s("usb"), Value::Sequence(usb_items));
        }
    }

    host
}

fn ensure_pci_function(host: &str) -> String {
    if host.matches('.').count() == 1 {
        host.to_string()
    } else {
        format!("{}.0", host)
    }
}

fn s(input: &str) -> Value {
    Value::String(input.to_string())
}

#[cfg(test)]
mod tests {
    use super::map_to_ezkvm_yaml;
    use crate::import::proxmox_parser::parse_proxmox_config;
    use crate::vm::config::Config;

    #[test]
    fn test_map_and_validate_yaml() {
        let proxmox = parse_proxmox_config(
            r#"
            name: imported-ubuntu
            memory: 8192
            sockets: 1
            cores: 4
            cpu: host
            bios: ovmf
            machine: q35
            scsi0: /dev/vm1/vm-100-disk-0,discard=on,cache=none
            net0: virtio=BC:24:11:FF:76:89,bridge=vmbr0
            hostpci0: 0000:03:10.4,multifunction=on
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();
        assert_eq!(parsed.general().name(), "imported-ubuntu");
        assert!(!result.mapped_keys.is_empty());
    }
}
