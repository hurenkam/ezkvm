use super::proxmox_model::{
    ProxmoxDiskEntry, ProxmoxHostPciEntry, ProxmoxStorageConfig, ProxmoxVmConfig,
};
use super::report::EzkvmImportResult;
use super::ImportError;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[allow(dead_code)]
pub fn map_to_ezkvm_yaml(
    proxmox: &ProxmoxVmConfig,
    name_override: Option<&str>,
) -> Result<EzkvmImportResult, ImportError> {
    map_to_ezkvm_yaml_with_storage(proxmox, None, name_override)
}

pub fn map_to_ezkvm_yaml_with_storage(
    proxmox: &ProxmoxVmConfig,
    storage_config: Option<&ProxmoxStorageConfig>,
    name_override: Option<&str>,
) -> Result<EzkvmImportResult, ImportError> {
    let mut mapped_keys = vec![];
    let mut skipped_keys = vec![];
    let mut warnings = vec![];
    let mut extras = vec![];

    let mut root = Mapping::new();

    let name = name_override
        .map(|x| x.to_string())
        .or_else(|| proxmox.scalars.get("name").cloned())
        .unwrap_or_else(|| "imported-vm".to_string());
    let guest_uuid = extract_uuid(proxmox);

    let mut general = Mapping::new();
    general.insert(s("name"), s(&name));
    mapped_keys.push("name".to_string());

    if let Some(uuid) = guest_uuid.as_deref() {
        general.insert(s("uuid"), s(uuid));
        mapped_keys.push("uuid".to_string());
    }

    map_general_flag(
        &mut general,
        proxmox,
        "agent",
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    );
    map_general_flag(
        &mut general,
        proxmox,
        "monitor",
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    );

    root.insert(s("general"), Value::Mapping(general));

    let mut system = Mapping::new();

    if let Some(cores) = proxmox.scalars.get("cores") {
        let cpu = system
            .entry(s("cpu"))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if let Ok(parsed) = cores.parse::<u64>() {
            cpu.as_mapping_mut()
                .expect("cpu section must be a mapping")
                .insert(s("cores"), Value::Number(parsed.into()));
            mapped_keys.push("cores".to_string());
        } else {
            warnings.push(format!("unable to parse cores '{}'", cores));
            skipped_keys.push("cores".to_string());
        }
    }
    if let Some(sockets) = proxmox.scalars.get("sockets") {
        let cpu = system
            .entry(s("cpu"))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        if let Ok(parsed) = sockets.parse::<u64>() {
            cpu.as_mapping_mut()
                .expect("cpu section must be a mapping")
                .insert(s("sockets"), Value::Number(parsed.into()));
            mapped_keys.push("sockets".to_string());
        } else {
            warnings.push(format!("unable to parse sockets '{}'", sockets));
            skipped_keys.push("sockets".to_string());
        }
    }
    if let Some(model) = proxmox.scalars.get("cpu") {
        let cpu = system
            .entry(s("cpu"))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
        map_cpu(
            cpu.as_mapping_mut().expect("cpu section must be a mapping"),
            model,
            &mut mapped_keys,
            &mut skipped_keys,
            &mut warnings,
        );
    }

    if let Some(memory) = proxmox.scalars.get("memory") {
        if let Ok(parsed) = memory.parse::<u64>() {
            let mut memory_map = Mapping::new();
            memory_map.insert(s("max"), Value::Number(parsed.into()));

            if let Some(balloon) = proxmox.scalars.get("balloon").and_then(|value| parse_boolish(value)) {
                memory_map.insert(s("balloon"), Value::Bool(balloon));
                mapped_keys.push("balloon".to_string());
            }

            if let Some(hugepages_raw) = proxmox.scalars.get("hugepages") {
                // Proxmox often stores hugepages as 0/2/1024 where non-zero means enabled.
                let hugepages_enabled = parse_boolish(hugepages_raw)
                    .or_else(|| hugepages_raw.parse::<u64>().ok().map(|value| value > 0));

                if let Some(enabled) = hugepages_enabled {
                    memory_map.insert(s("hugepages"), Value::Bool(enabled));
                    mapped_keys.push("hugepages".to_string());

                    if enabled {
                        memory_map.insert(s("mem_path"), s("/run/hugepages/kvm/1048576kB"));
                        memory_map.insert(s("prealloc"), Value::Bool(true));
                    }
                } else {
                    warnings.push(format!("unable to parse hugepages '{}'", hugepages_raw));
                    skipped_keys.push("hugepages".to_string());
                }
            }

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

    if let Some(bios_map) = map_bios(
        proxmox,
        storage_config,
        guest_uuid.as_deref(),
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    ) {
        system.insert(s("bios"), Value::Mapping(bios_map));
    }

    if let Some(tpm_map) = map_tpm(
        proxmox,
        storage_config,
        &name,
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    )
    {
        system.insert(s("tpm"), Value::Mapping(tpm_map));
    }

    if let Some(rng_map) = map_rng(proxmox, &mut mapped_keys, &mut skipped_keys, &mut warnings) {
        system.insert(s("virtio_rng"), Value::Mapping(rng_map));
    }

    if let Some(serial_map) = map_serial(
        proxmox,
        &name,
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    ) {
        system.insert(s("serial"), Value::Mapping(serial_map));
    }

    if let Some(numa_nodes) = map_numa_nodes(
        proxmox,
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    ) {
        system.insert(s("numa_nodes"), Value::Sequence(numa_nodes));
    }

    if let Some(vmgenid) = proxmox.scalars.get("vmgenid") {
        system.insert(s("vmgenid"), s(vmgenid));
        mapped_keys.push("vmgenid".to_string());
    }

    if !system.is_empty() {
        root.insert(s("system"), Value::Mapping(system));
    }

    let (gpu_passthrough, host_passthrough) = split_gpu_passthrough_devices(&proxmox.host_pci);

    // Determine display type first so we can conditionally generate SPICE or VNC
    let display_type = infer_display_type(proxmox, &gpu_passthrough);

    if let Some(spice) = map_spice(
        proxmox,
        &gpu_passthrough,
        &display_type,
        &mut mapped_keys,
        &mut warnings,
    ) {
        root.insert(s("spice"), Value::Mapping(spice));
    }

    if let Some(vnc) = map_vnc(
        proxmox,
        &name,
        &gpu_passthrough,
        &display_type,
        &mut mapped_keys,
        &mut warnings,
    ) {
        root.insert(s("vnc"), Value::Mapping(vnc));
    }

    if let Some(gpu) = map_gpu(
        proxmox,
        &gpu_passthrough,
        &mut extras,
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    ) {
        root.insert(s("gpu"), Value::Mapping(gpu));
    }

    if let Some(display) = map_display(
        proxmox,
        &gpu_passthrough,
        &mut mapped_keys,
        &mut warnings,
    ) {
        root.insert(s("display"), Value::Mapping(display));
    }

    let boot_order = parse_boot_order(proxmox, &mut mapped_keys, &mut skipped_keys, &mut warnings);
    let scsihw = parse_scsihw(proxmox, &mut mapped_keys);

    let storage = map_storage(
        &proxmox.disks,
        storage_config,
        &boot_order,
        &scsihw,
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    );
    if !storage.is_empty() {
        root.insert(s("storage"), Value::Sequence(storage));
    }

    let proxmox_vmid = infer_vmid(proxmox);
    let network = map_network(
        proxmox,
        proxmox_vmid.as_deref(),
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    );
    if !network.is_empty() {
        root.insert(s("network"), Value::Sequence(network));
    }

    let host = map_host(
        &host_passthrough,
        &proxmox.usb,
        &mut mapped_keys,
        &mut warnings,
    );
    if !host.is_empty() {
        root.insert(s("host"), Value::Mapping(host));
    }

    map_extras(proxmox, &mut extras, &mut mapped_keys, &mut skipped_keys, &mut warnings);
    if !extras.is_empty() {
        root.insert(
            s("extras"),
            Value::Sequence(extras.iter().map(|item| s(item)).collect()),
        );
    }

    let yaml = render_import_yaml(&root)
        .map_err(|e| ImportError::ValidationError(format!("yaml serialization failed: {}", e)))?;

    Ok(EzkvmImportResult {
        yaml,
        mapped_keys,
        skipped_keys,
        warnings,
    })
}

fn render_import_yaml(root: &Mapping) -> Result<String, serde_yaml::Error> {
    let mut rendered = String::new();

    for (index, (key, value)) in root.iter().enumerate() {
        if index > 0 {
            rendered.push('\n');
        }

        let key = key.as_str().expect("yaml keys must be strings");
        match value {
            Value::Mapping(mapping) => {
                rendered.push_str(key);
                rendered.push_str(":\n");
                rendered.push_str(&render_mapping_entries(mapping, 1, &[key.to_string()])?);
            }
            Value::Sequence(sequence) => {
                rendered.push_str(key);
                rendered.push_str(":\n");
                rendered.push_str(&render_sequence(sequence, 1, &[key.to_string()])?);
            }
            _ => {
                rendered.push_str(key);
                rendered.push_str(": ");
                rendered.push_str(&render_scalar(value)?);
                rendered.push('\n');
            }
        }
    }

    Ok(rendered)
}

fn render_mapping_entries(
    mapping: &Mapping,
    indent: usize,
    path: &[String],
) -> Result<String, serde_yaml::Error> {
    let mut rendered = String::new();

    for (key, value) in mapping {
        let key = key.as_str().expect("yaml keys must be strings");
        let child_path = append_path(path, key);
        rendered.push_str(&"  ".repeat(indent));
        rendered.push_str(key);

        match value {
            Value::Mapping(child) if should_inline_mapping(&child_path, child) => {
                rendered.push_str(": ");
                rendered.push_str(&render_inline_mapping(child)?);
                rendered.push('\n');
            }
            Value::Mapping(child) => {
                rendered.push_str(":\n");
                rendered.push_str(&render_mapping_entries(child, indent + 1, &child_path)?);
            }
            Value::Sequence(sequence) => {
                rendered.push_str(":\n");
                rendered.push_str(&render_sequence(sequence, indent + 1, &child_path)?);
            }
            _ => {
                rendered.push_str(": ");
                rendered.push_str(&render_scalar(value)?);
                rendered.push('\n');
            }
        }
    }

    Ok(rendered)
}

fn render_sequence(
    sequence: &[Value],
    indent: usize,
    path: &[String],
) -> Result<String, serde_yaml::Error> {
    let mut rendered = String::new();

    for item in sequence {
        rendered.push_str(&"  ".repeat(indent));
        rendered.push_str("- ");

        match item {
            Value::Mapping(mapping) if should_inline_sequence_item(path, mapping) => {
                rendered.push_str(&render_inline_mapping(mapping)?);
                rendered.push('\n');
            }
            Value::Mapping(mapping) => {
                rendered.push_str(&render_block_sequence_mapping(mapping, indent, path)?);
            }
            Value::Sequence(child_sequence) => {
                rendered.push('\n');
                rendered.push_str(&render_sequence(child_sequence, indent + 1, path)?);
            }
            _ => {
                rendered.push_str(&render_scalar(item)?);
                rendered.push('\n');
            }
        }
    }

    Ok(rendered)
}

fn render_block_sequence_mapping(
    mapping: &Mapping,
    indent: usize,
    path: &[String],
) -> Result<String, serde_yaml::Error> {
    let mut rendered = String::new();
    let mut entries = mapping.iter();

    if let Some((first_key, first_value)) = entries.next() {
        let first_key = first_key.as_str().expect("yaml keys must be strings");
        let first_path = append_path(path, first_key);
        rendered.push_str(first_key);

        match first_value {
            Value::Mapping(child) if should_inline_mapping(&first_path, child) => {
                rendered.push_str(": ");
                rendered.push_str(&render_inline_mapping(child)?);
                rendered.push('\n');
            }
            Value::Mapping(child) => {
                rendered.push_str(":\n");
                rendered.push_str(&render_mapping_entries(child, indent + 1, &first_path)?);
            }
            Value::Sequence(sequence) => {
                rendered.push_str(":\n");
                rendered.push_str(&render_sequence(sequence, indent + 1, &first_path)?);
            }
            _ => {
                rendered.push_str(": ");
                rendered.push_str(&render_scalar(first_value)?);
                rendered.push('\n');
            }
        }
    }

    for (key, value) in entries {
        let key = key.as_str().expect("yaml keys must be strings");
        let child_path = append_path(path, key);
        rendered.push_str(&"  ".repeat(indent + 1));
        rendered.push_str(key);

        match value {
            Value::Mapping(child) if should_inline_mapping(&child_path, child) => {
                rendered.push_str(": ");
                rendered.push_str(&render_inline_mapping(child)?);
                rendered.push('\n');
            }
            Value::Mapping(child) => {
                rendered.push_str(":\n");
                rendered.push_str(&render_mapping_entries(child, indent + 2, &child_path)?);
            }
            Value::Sequence(sequence) => {
                rendered.push_str(":\n");
                rendered.push_str(&render_sequence(sequence, indent + 2, &child_path)?);
            }
            _ => {
                rendered.push_str(": ");
                rendered.push_str(&render_scalar(value)?);
                rendered.push('\n');
            }
        }
    }

    Ok(rendered)
}

fn should_inline_mapping(path: &[String], mapping: &Mapping) -> bool {
    if !mapping.keys().all(|key| key.as_str().is_some()) {
        return false;
    }

    if !mapping.values().all(value_is_inline_simple) {
        return false;
    }

    match path {
        [section] if matches!(section.as_str(), "general" | "gpu" | "display" | "spice" | "vnc" | "host") => false,
        [section, _] if section == "system" => true,
        [section, subsection, _] if section == "host" && matches!(subsection.as_str(), "pci" | "usb") => true,
        _ => mapping.is_empty(),
    }
}

fn should_inline_sequence_item(path: &[String], mapping: &Mapping) -> bool {
    if !mapping.keys().all(|key| key.as_str().is_some()) {
        return false;
    }

    if !mapping.values().all(value_is_inline_simple) {
        return false;
    }

    matches!(path, [section] if section == "network")
        || matches!(path, [section, subsection] if section == "storage" && subsection == "drives")
        || matches!(path, [section, subsection] if section == "host" && matches!(subsection.as_str(), "pci" | "usb"))
}

fn value_is_inline_simple(value: &Value) -> bool {
    matches!(value, Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_))
        || matches!(value, Value::Mapping(mapping) if mapping.is_empty())
        || matches!(value, Value::Sequence(sequence) if sequence.is_empty())
}

fn render_inline_mapping(mapping: &Mapping) -> Result<String, serde_yaml::Error> {
    let mut parts = vec![];
    for (key, value) in mapping {
        let key = key.as_str().expect("yaml keys must be strings");
        parts.push(format!("{}: {}", key, render_inline_value(value)?));
    }
    Ok(format!("{{ {} }}", parts.join(", ")))
}

fn render_inline_value(value: &Value) -> Result<String, serde_yaml::Error> {
    match value {
        Value::Mapping(mapping) if mapping.is_empty() => Ok("{ }".to_string()),
        Value::Sequence(sequence) if sequence.is_empty() => Ok("[]".to_string()),
        _ => render_scalar(value),
    }
}

fn render_scalar(value: &Value) -> Result<String, serde_yaml::Error> {
    let rendered = serde_yaml::to_string(value)?;
    Ok(rendered.trim_end().to_string())
}

fn append_path(path: &[String], segment: &str) -> Vec<String> {
    let mut child = path.to_vec();
    child.push(segment.to_string());
    child
}

fn parse_boot_order(
    proxmox: &ProxmoxVmConfig,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let Some(raw) = proxmox.scalars.get("boot") else {
        return vec![];
    };

    // Expected format: "order=scsi2" or "order=scsi2;net0;ide0"
    for token in raw.split(',').map(str::trim) {
        if let Some(order_value) = token.strip_prefix("order=") {
            let devices: Vec<String> = order_value
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if devices.is_empty() {
                warnings.push(format!("boot order '{}' has no devices; skipping", raw));
                skipped_keys.push("boot".to_string());
            } else {
                mapped_keys.push("boot".to_string());
            }
            return devices;
        }
    }
    warnings.push(format!(
        "boot '{}' has no recognized 'order=' clause; skipping",
        raw
    ));
    skipped_keys.push("boot".to_string());
    vec![]
}

fn parse_scsihw(proxmox: &ProxmoxVmConfig, mapped_keys: &mut Vec<String>) -> Option<String> {
    let raw = proxmox.scalars.get("scsihw")?;
    mapped_keys.push("scsihw".to_string());
    Some(raw.to_ascii_lowercase())
}

fn map_storage(
    disks: &[ProxmoxDiskEntry],
    storage_config: Option<&ProxmoxStorageConfig>,
    boot_order: &[String],
    scsihw: &Option<String>,
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

        let use_virtio_scsi_single = bus == "scsi"
            && scsihw.as_deref() == Some("virtio-scsi-single");
        let controller_name = if use_virtio_scsi_single {
            "virtio-scsi-single"
        } else if bus == "scsi" {
            "pvscsi"
        } else {
            &bus
        };

        let mut controller = Mapping::new();
        controller.insert(s("controller"), s(controller_name));

        let mut drive_items = vec![];
        for drive in drives {
            let mut drive_map = Mapping::new();
            let media = drive.options.get("media").cloned().unwrap_or_default();
            let drive_type = if media == "cdrom" { "cd" } else { "hd" };
            let resolved_source = resolve_volume_reference(&drive.source, storage_config)
                .unwrap_or_else(|| drive.source.clone());

            drive_map.insert(s("type"), s(drive_type));
            drive_map.insert(s("file"), s(&resolved_source));

            if let Some(discard) = drive.options.get("discard") {
                drive_map.insert(s("discard"), s(discard));
            }
            if let Some(cache) = drive.options.get("cache") {
                drive_map.insert(s("cache"), s(cache));
            }
            if let Some(aio) = drive.options.get("aio") {
                drive_map.insert(s("aio"), s(aio));
            }
            if let Some(serial) = drive.options.get("serial") {
                drive_map.insert(s("serial"), s(serial));
            }
            if let Some(werror) = drive.options.get("werror") {
                drive_map.insert(s("werror"), s(werror));
            }
            if let Some(rerror) = drive.options.get("rerror") {
                drive_map.insert(s("rerror"), s(rerror));
            }
            // Explicit per-drive bootindex option takes precedence
            if let Some(boot) = drive.options.get("bootindex") {
                if let Ok(parsed) = boot.parse::<u64>() {
                    drive_map.insert(s("boot_index"), Value::Number(parsed.into()));
                }
            } else if let Some(pos) = boot_order.iter().position(|k| k == &drive.key) {
                // 1-based position in the Proxmox boot order sequence
                drive_map.insert(s("boot_index"), Value::Number(((pos + 1) as u64).into()));
            }

            if !resolved_source.starts_with('/') {
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
    proxmox_vmid: Option<&str>,
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
        if let Some(vmid) = proxmox_vmid {
            item.insert(s("type"), s("proxmox_tap"));
            item.insert(s("bridge"), s(bridge));
            item.insert(s("vmid"), s(vmid));
        } else {
            warnings.push(format!(
                "network '{}' could not infer vmid; using bridge backend",
                net.key
            ));
            item.insert(s("type"), s("bridge"));
            item.insert(s("bridge"), s(bridge));
        }

        let driver = normalize_net_driver(&net.model, warnings);
        item.insert(s("driver"), s(&driver));

        if let Some(mac) = &net.mac {
            item.insert(s("mac"), s(mac));
        }

        if let Some(queues) = net.options.get("queues") {
            if let Ok(parsed) = queues.parse::<u64>() {
                item.insert(s("queues"), Value::Number(parsed.into()));
                mapped_keys.push(format!("{}.queues", net.key));
            } else {
                warnings.push(format!(
                    "network '{}' queues '{}' is not numeric; skipping",
                    net.key, queues
                ));
                skipped_keys.push(format!("{}.queues", net.key));
            }
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
    host_pci: &[&ProxmoxHostPciEntry],
    usb_devices: &[super::proxmox_model::ProxmoxUsbEntry],
    mapped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Mapping {
    let mut host = Mapping::new();

    if !host_pci.is_empty() {
        let pci_items = map_pci_entries(host_pci, mapped_keys);
        host.insert(s("pci"), Value::Sequence(pci_items));
    }

    if !usb_devices.is_empty() {
        let mut usb_items = vec![];
        for item in usb_devices {
            if let Some(host) = usb_host_selector(item) {
                if let Some((bus, port)) = host.split_once('-') {
                    let mut map = Mapping::new();
                    map.insert(s("vm_port"), s(&(item.index + 1).to_string()));
                    map.insert(s("host_bus"), s(bus));
                    map.insert(s("host_port"), s(port));

                    mapped_keys.push(item.key.clone());
                    usb_items.push(Value::Mapping(map));
                } else if let Some((vendor_id, product_id)) = host.split_once(':') {
                    let mut map = Mapping::new();
                    map.insert(s("vendor_id"), s(vendor_id));
                    map.insert(s("product_id"), s(product_id));

                    mapped_keys.push(item.key.clone());
                    usb_items.push(Value::Mapping(map));
                } else {
                    warnings.push(format!(
                        "usb '{}' host format '{}' not recognized, expected <bus>-<port> or <vendorid>:<productid>",
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

fn usb_host_selector(item: &super::proxmox_model::ProxmoxUsbEntry) -> Option<&str> {
    item.options
        .get("host")
        .map(String::as_str)
        .or_else(|| item.host.strip_prefix("host="))
}

fn map_general_flag(
    general: &mut Mapping,
    proxmox: &ProxmoxVmConfig,
    key: &str,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if let Some(raw) = proxmox.scalars.get(key) {
        if let Some(value) = parse_boolish(raw) {
            general.insert(s(key), s(if value { "yes" } else { "no" }));
            mapped_keys.push(key.to_string());
        } else {
            warnings.push(format!("unable to parse {} '{}'", key, raw));
            skipped_keys.push(key.to_string());
        }
    }
}

fn map_cpu(
    cpu: &mut Mapping,
    value: &str,
    mapped_keys: &mut Vec<String>,
    _skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let mut tokens = value.split(',').map(|token| token.trim()).filter(|token| !token.is_empty());
    let mut model = tokens.next().unwrap_or("qemu64").to_string();
    let mut explicit_flags: Option<String> = None;

    if let Some((key, parsed_model)) = model.split_once('=') {
        if key == "cputype" || key == "model" {
            model = parsed_model.trim().to_string();
        }
    }

    for token in tokens {
        if let Some((key, raw_value)) = token.split_once('=') {
            match key.trim() {
                "flags" => {
                    // Proxmox uses semicolons to separate flags; ezkvm expects commas
                    explicit_flags = Some(raw_value.trim().replace(';', ","));
                }
                "hidden" | "hv-vendor-id" | "reported-model" => warnings.push(format!(
                    "cpu option '{}' is not represented in typed ezkvm config; consider preserving it in extras manually",
                    key.trim()
                )),
                other => warnings.push(format!(
                    "cpu option '{}' is not mapped from Proxmox; skipped during import",
                    other
                )),
            }
        }
    }

    // KVM passthrough models: Proxmox automatically adds paravirt flags that are not
    // explicit in the VM config. Inject them here so guest behaviour matches.
    let is_kvm_passthrough = matches!(model.as_str(), "host" | "host-passthrough" | "host-model");
    let flags = if is_kvm_passthrough {
        let mut f = explicit_flags.unwrap_or_default();
        if !f.contains("kvm_pv_eoi") {
            let paravirt = "+kvm_pv_eoi,+kvm_pv_unhalt";
            f = if f.is_empty() {
                paravirt.to_string()
            } else {
                format!("{},{}", paravirt, f)
            };
            warnings.push(format!(
                "paravirt flags +kvm_pv_eoi,+kvm_pv_unhalt injected for cpu model '{}' (Proxmox implicit default)",
                model
            ));
        }
        f
    } else {
        explicit_flags.unwrap_or_default()
    };

    cpu.insert(s("model"), s(&model));
    cpu.insert(s("flags"), s(&flags));
    mapped_keys.push("cpu".to_string());
}

fn map_bios(
    proxmox: &ProxmoxVmConfig,
    storage_config: Option<&ProxmoxStorageConfig>,
    guest_uuid: Option<&str>,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let mut bios_type = proxmox.scalars.get("bios").map(|bios| {
        if bios.eq_ignore_ascii_case("ovmf") {
            "ovmf"
        } else {
            "seabios"
        }
    });

    if bios_type.is_none() && proxmox.scalars.contains_key("efidisk0") {
        bios_type = Some("ovmf");
        warnings.push("efidisk0 present without explicit bios setting; assuming ovmf".to_string());
    }

    let bios_type = bios_type?;
    let mut bios = Mapping::new();
    bios.insert(s("type"), s(bios_type));
    mapped_keys.push("bios".to_string());

    if let Some(uuid) = guest_uuid {
        bios.insert(s("uuid"), s(uuid));
    }

    if bios_type == "ovmf" {
        if let Some(efidisk) = proxmox.scalars.get("efidisk0") {
            let (source, options) = parse_inline_options(efidisk);
            let resolved_source =
                resolve_volume_reference(&source, storage_config).unwrap_or_else(|| source.clone());
            bios.insert(s("file"), s(&resolved_source));
            mapped_keys.push("efidisk0".to_string());

            if !resolved_source.starts_with('/') {
                warnings.push(format!(
                    "efidisk0 source '{}' may require manual path translation",
                    source
                ));
            }

            if let Some(efitype) = options.get("efitype") {
                let normalized = efitype.to_ascii_uppercase();
                if normalized == "2M" || normalized == "4M" {
                    bios.insert(s("size"), s(&normalized));
                } else {
                    warnings.push(format!("unsupported efidisk0 efitype '{}'", efitype));
                    skipped_keys.push("efidisk0.efitype".to_string());
                }
            }

            if let Some(keys) = options.get("pre-enrolled-keys") {
                if let Some(enabled) = parse_boolish(keys) {
                    bios.insert(s("secure_boot"), Value::Bool(enabled));
                }
            }
        }
    }

    Some(bios)
}

fn map_tpm(
    proxmox: &ProxmoxVmConfig,
    storage_config: Option<&ProxmoxStorageConfig>,
    name: &str,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let raw = proxmox.scalars.get("tpmstate0")?;
    let (source, options) = parse_inline_options(raw);
    let resolved_source =
        resolve_volume_reference(&source, storage_config).unwrap_or_else(|| source.clone());

    if !resolved_source.starts_with('/') {
        warnings.push(format!(
            "tpmstate0 source '{}' is not an absolute path; skipping typed tpm mapping",
            source
        ));
        skipped_keys.push("tpmstate0".to_string());
        return None;
    }

    let mut tpm = Mapping::new();
    tpm.insert(s("type"), s("swtpm"));
    tpm.insert(s("disk"), s(&resolved_source));
    tpm.insert(s("socket"), s(&format!("/var/ezkvm/{}-tpm.socket", name)));

    if let Some(version) = options.get("version") {
        tpm.insert(s("version"), s(version.trim_start_matches('v')));
    }

    mapped_keys.push("tpmstate0".to_string());
    Some(tpm)
}

fn map_rng(
    proxmox: &ProxmoxVmConfig,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let raw = proxmox.scalars.get("rng0")?;
    let mut rng = Mapping::new();

    // Proxmox uses rng0 as comma-separated key=value pairs.
    let options: HashMap<String, String> = raw
        .split(',')
        .filter_map(|token| token.trim().split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();

    if let Some(source) = options.get("source") {
        rng.insert(s("filename"), s(source));
    } else {
        warnings.push("rng0 present without source option; using ezkvm default /dev/urandom".to_string());
    }

    if let Some(max_bytes) = options.get("max_bytes") {
        warnings.push(format!(
            "rng0 max_bytes={} is not yet represented in typed ezkvm config",
            max_bytes
        ));
        skipped_keys.push("rng0.max_bytes".to_string());
    }
    if let Some(period) = options.get("period") {
        warnings.push(format!(
            "rng0 period={} is not yet represented in typed ezkvm config",
            period
        ));
        skipped_keys.push("rng0.period".to_string());
    }

    mapped_keys.push("rng0".to_string());
    Some(rng)
}

fn split_gpu_passthrough_devices<'a>(
    items: &'a [ProxmoxHostPciEntry],
) -> (Vec<&'a ProxmoxHostPciEntry>, Vec<&'a ProxmoxHostPciEntry>) {
    let mut gpu_groups = BTreeSet::new();

    for item in items {
        if item
            .options
            .get("x-vga")
            .and_then(|value| parse_boolish(value))
            .unwrap_or(false)
        {
            gpu_groups.insert(pci_group_key(&item.host));
        }
    }

    if gpu_groups.is_empty() {
        return (vec![], items.iter().collect());
    }

    let mut gpu = vec![];
    let mut host = vec![];

    for item in items {
        if gpu_groups.contains(&pci_group_key(&item.host)) {
            gpu.push(item);
        } else {
            host.push(item);
        }
    }

    (gpu, host)
}

fn map_pci_entries(items: &[&ProxmoxHostPciEntry], mapped_keys: &mut Vec<String>) -> Vec<Value> {
    let mut pci_items = vec![];

    for item in items {
        let mut map = Mapping::new();
        let normalized_host = ensure_pci_function(&item.host);
        let function = pci_function(&normalized_host).unwrap_or(0);

        // q35 ich9 downstream ports accept slot 0. Map each hostpciN to port N+1 and keep slot 0.
        let vm_id = if function == 0 {
            "0".to_string()
        } else {
            format!("0.{:x}", function)
        };

        map.insert(s("vm_id"), s(&vm_id));
        map.insert(s("port"), s(&(item.index + 1).to_string()));
        map.insert(s("host_id"), s(&normalized_host));

        if let Some(mf) = item.options.get("multifunction").and_then(|value| parse_boolish(value)) {
            map.insert(s("multi_function"), Value::Bool(mf));
        }

        mapped_keys.push(item.key.clone());
        pci_items.push(Value::Mapping(map));
    }

    pci_items
}

fn pci_function(host: &str) -> Option<u8> {
    host.rsplit_once('.')
        .and_then(|(_, function)| u8::from_str_radix(function, 16).ok())
}

fn map_gpu(
    proxmox: &ProxmoxVmConfig,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
    extras: &mut Vec<String>,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    if !gpu_passthrough.is_empty() {
        let mut gpu = Mapping::new();
        gpu.insert(s("type"), s("passthrough"));
        gpu.insert(s("pci"), Value::Sequence(map_pci_entries(gpu_passthrough, mapped_keys)));
        mapped_keys.push("gpu_passthrough".to_string());
        return Some(gpu);
    }

    let vga = proxmox.scalars.get("vga")?;
    let model = vga_model(vga);
    let mut gpu = Mapping::new();

    match model.as_str() {
        "virtio" | "virtio-vga" | "virtio-gl" => {
            gpu.insert(s("type"), s("virtio"));
            if model == "virtio-gl" {
                gpu.insert(s("gl"), s("yes"));
            }
            mapped_keys.push("vga".to_string());
            Some(gpu)
        }
        "vmware" | "vmware-svga" => {
            gpu.insert(s("type"), s("vmware-svga"));
            mapped_keys.push("vga".to_string());
            Some(gpu)
        }
        "none" | "serial0" => {
            gpu.insert(s("type"), s("no_gpu"));
            mapped_keys.push("vga".to_string());
            Some(gpu)
        }
        "std" | "cirrus" | "qxl" | "qxl2" | "qxl3" | "qxl4" => {
            extras.push(format!("-vga {}", model));
            warnings.push(format!(
                "vga model '{}' is not a typed ezkvm gpu; preserved as raw extra",
                model
            ));
            mapped_keys.push("vga".to_string());
            None
        }
        other => {
            warnings.push(format!("unsupported vga model '{}' skipped during import", other));
            skipped_keys.push("vga".to_string());
            None
        }
    }
}

fn infer_display_type(
    proxmox: &ProxmoxVmConfig,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
) -> Option<String> {
    if !gpu_passthrough.is_empty() {
        if proxmox
            .scalars
            .get("vga")
            .map(|value| vga_model(value) == "none")
            .unwrap_or(true)
        {
            return Some("no_display".to_string());
        }
    }

    let Some(vga) = proxmox.scalars.get("vga") else {
        return None;
    };

    match vga_model(vga).as_str() {
        "virtio" | "virtio-vga" | "virtio-gl" | "vmware" | "vmware-svga" | "qxl" | "qxl2"
        | "qxl3" | "qxl4" | "std" | "cirrus" => Some("remote-viewer".to_string()),
        "none" | "serial0" => Some("no_display".to_string()),
        _ => None,
    }
}

fn map_display(
    proxmox: &ProxmoxVmConfig,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
    mapped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let mut display = Mapping::new();

    if !gpu_passthrough.is_empty() {
        if proxmox
            .scalars
            .get("vga")
            .map(|value| vga_model(value) == "none")
            .unwrap_or(true)
        {
            display.insert(s("type"), s("no_display"));
            mapped_keys.push("display".to_string());
            return Some(display);
        }
    }

    let Some(vga) = proxmox.scalars.get("vga") else {
        return None;
    };

    match vga_model(vga).as_str() {
        "virtio" | "virtio-vga" | "virtio-gl" | "vmware" | "vmware-svga" | "qxl" | "qxl2"
        | "qxl3" | "qxl4" | "std" | "cirrus" => {
            display.insert(s("type"), s("remote-viewer"));
            let tablet_enabled = proxmox
                .scalars
                .get("tablet")
                .and_then(|raw| parse_boolish(raw))
                .unwrap_or(true);
            display.insert(s("usb_tablet"), Value::Bool(tablet_enabled));
            warnings.push(format!(
                "display inferred as remote-viewer from Proxmox vga '{}'; importer defaults this path to SPICE",
                vga_model(vga)
            ));
            mapped_keys.push("display".to_string());
            if proxmox.scalars.contains_key("tablet") {
                mapped_keys.push("tablet".to_string());
            }
            Some(display)
        }
        "none" | "serial0" => {
            display.insert(s("type"), s("no_display"));
            mapped_keys.push("display".to_string());
            Some(display)
        }
        _ => None,
    }
}

fn map_serial(
    proxmox: &ProxmoxVmConfig,
    vm_name: &str,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let serial0 = proxmox.scalars.get("serial0")?;

    if serial0 != "socket" {
        warnings.push(format!(
            "serial0 option '{}' is not yet mapped; expected 'socket'",
            serial0
        ));
        skipped_keys.push("serial0".to_string());
        return None;
    }

    let path = infer_vmid(proxmox)
        .map(|vmid| format!("/var/run/qemu-server/{}.serial0", vmid))
        .unwrap_or_else(|| format!("/var/ezkvm/{}.serial0", vm_name));

    let mut serial = Mapping::new();
    serial.insert(s("type"), s("socket"));
    serial.insert(s("path"), s(&path));
    mapped_keys.push("serial0".to_string());
    Some(serial)
}

fn map_numa_nodes(
    proxmox: &ProxmoxVmConfig,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Vec<Value>> {
    // Only generate NUMA nodes when `numa: 1` and hugepages are both active.
    let numa_enabled = proxmox.scalars.get("numa").and_then(|v| parse_boolish(v))
        .or_else(|| proxmox.scalars.get("numa").and_then(|v| v.parse::<u64>().ok().map(|n| n > 0)));
    let numa_enabled = numa_enabled?;
    mapped_keys.push("numa".to_string());
    if !numa_enabled {
        return None;
    }

    let hugepages_raw = match proxmox.scalars.get("hugepages") {
        Some(v) => v,
        None => return None,
    };
    let hugepages_enabled = parse_boolish(hugepages_raw)
        .or_else(|| hugepages_raw.parse::<u64>().ok().map(|v| v > 0))
        .unwrap_or(false);
    if !hugepages_enabled {
        return None;
    }

    let total_memory = proxmox
        .scalars
        .get("memory")
        .and_then(|m| m.parse::<u32>().ok())
        .unwrap_or(0);
    let sockets = proxmox
        .scalars
        .get("sockets")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let cores = proxmox
        .scalars
        .get("cores")
        .and_then(|c| c.parse::<u32>().ok())
        .unwrap_or(1);

    if sockets == 0 || cores == 0 {
        warnings.push("numa: unable to derive node topology (sockets or cores is 0)".to_string());
        skipped_keys.push("numa".to_string());
        return None;
    }

    let cpus_per_socket = cores;
    let mem_per_node = total_memory / sockets;
    let mut nodes = vec![];

    for socket in 0..sockets {
        let cpu_start = socket * cpus_per_socket;
        let cpu_end = cpu_start + cpus_per_socket - 1;
        let cpus_str = if cpu_start == cpu_end {
            cpu_start.to_string()
        } else {
            format!("{}-{}", cpu_start, cpu_end)
        };

        let mut node = Mapping::new();
        node.insert(s("nodeid"), Value::Number((socket as u64).into()));
        node.insert(s("cpus"), s(&cpus_str));
        node.insert(s("mem"), Value::Number((mem_per_node as u64).into()));
        nodes.push(Value::Mapping(node));
    }

    mapped_keys.push("numa".to_string());
    Some(nodes)
}

fn map_spice(
    proxmox: &ProxmoxVmConfig,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
    display_type: &Option<String>,
    mapped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let Some(vga) = proxmox.scalars.get("vga") else {
        return None;
    };

    if !gpu_passthrough.is_empty() && vga_model(vga) == "none" {
        return None;
    }

    match vga_model(vga).as_str() {
        "virtio" | "virtio-vga" | "virtio-gl" | "vmware" | "vmware-svga" | "qxl" | "qxl2"
        | "qxl3" | "qxl4" => {
            let mut spice = Mapping::new();
            spice.insert(s("addr"), s("127.0.0.1"));
            // Proxmox-style TLS-only SPICE endpoint: plain TCP disabled, TLS listener enabled.
            spice.insert(s("port"), Value::Number(0_u64.into()));
            spice.insert(s("tls_port"), Value::Number(61000_u64.into()));
            spice.insert(s("tls_ciphers"), s("HIGH"));
            spice.insert(s("seamless_migration"), Value::Bool(true));
            spice.insert(s("disable_ticketing"), Value::Bool(true));

            if matches!(display_type.as_deref(), Some("remote-viewer")) {
                warnings.push(format!(
                    "spice endpoint inferred from Proxmox vga '{}' (remote-viewer path; using Proxmox-like TCP+TLS defaults)",
                    vga_model(vga)
                ));
            } else {
                warnings.push(format!(
                    "spice endpoint inferred from Proxmox vga '{}' with TCP+TLS defaults",
                    vga_model(vga)
                ));
            }
            mapped_keys.push("spice".to_string());
            Some(spice)
        }
        _ => None,
    }
}

fn map_vnc(
    proxmox: &ProxmoxVmConfig,
    vm_name: &str,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
    display_type: &Option<String>,
    mapped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let Some(vga) = proxmox.scalars.get("vga") else {
        return None;
    };

    let vnc_path = infer_vmid(proxmox)
        .map(|vmid| format!("/var/run/qemu-server/{}.vnc", vmid))
        .unwrap_or_else(|| format!("/var/ezkvm/{}.vnc", vm_name));

    if matches!(display_type.as_deref(), Some("remote-viewer")) {
        let mut vnc = Mapping::new();
        vnc.insert(s("path"), s(&vnc_path));
        vnc.insert(s("password"), Value::Bool(true));
        warnings.push(format!(
            "vnc unix socket inferred from Proxmox vga '{}' on remote-viewer path for management compatibility",
            vga_model(vga)
        ));
        mapped_keys.push("vnc".to_string());
        return Some(vnc);
    }

    // Legacy non-remote-viewer path: generate VNC for std and cirrus (usually local displays)
    if !gpu_passthrough.is_empty() && vga_model(vga) == "none" {
        return None;
    }

    match vga_model(vga).as_str() {
        "std" | "cirrus" => {
            let mut vnc = Mapping::new();
            vnc.insert(s("path"), s(&vnc_path));
            vnc.insert(s("password"), Value::Bool(true));
            warnings.push(format!(
                "vnc unix socket inferred from Proxmox vga '{}'",
                vga_model(vga)
            ));
            mapped_keys.push("vnc".to_string());
            Some(vnc)
        }
        _ => None,
    }
}

fn map_extras(
    proxmox: &ProxmoxVmConfig,
    extras: &mut Vec<String>,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if let Some(args) = proxmox.scalars.get("args") {
        match shell_words::split(args) {
            Ok(tokens) => {
                extras.extend(tokens);
                mapped_keys.push("args".to_string());
            }
            Err(error) => {
                warnings.push(format!("unable to parse raw args '{}': {}", args, error));
                skipped_keys.push("args".to_string());
            }
        }
    }
}

fn parse_inline_options(value: &str) -> (String, HashMap<String, String>) {
    let mut tokens = value.split(',');
    let source = tokens.next().unwrap_or("").trim().to_string();
    let mut options = HashMap::new();

    for token in tokens {
        let token = token.trim();
        if let Some((key, value)) = token.split_once('=') {
            options.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    (source, options)
}

fn resolve_volume_reference(
    source: &str,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> Option<String> {
    if source.starts_with('/') {
        return Some(source.to_string());
    }

    let (store_id, volume) = source.split_once(':')?;
    let storage = storage_config?.storages.get(store_id)?;

    match storage.storage_type.as_str() {
        "dir" => storage
            .options
            .get("path")
            .map(|base_path| resolve_dir_volume(base_path, volume)),
        "lvm" | "lvmthin" => storage.options.get("vgname").and_then(|vgname| {
            if volume.contains('/') {
                None
            } else {
                Some(format!("/dev/{}/{}", vgname, volume))
            }
        }),
        _ => None,
    }
}

fn resolve_dir_volume(base_path: &str, volume: &str) -> String {
    let trimmed_base = base_path.trim_end_matches('/');

    if volume.starts_with('/') {
        return volume.to_string();
    }

    if volume.starts_with("images/")
        || volume.starts_with("iso/")
        || volume.starts_with("vztmpl/")
        || volume.starts_with("backup/")
        || volume.starts_with("snippets/")
        || volume.starts_with("template/")
        || volume.starts_with("rootdir/")
    {
        return format!("{}/{}", trimmed_base, volume);
    }

    if let Some((prefix, _)) = volume.split_once('/') {
        if prefix.chars().all(|ch| ch.is_ascii_digit()) {
            return format!("{}/images/{}", trimmed_base, volume);
        }
    }

    format!("{}/{}", trimmed_base, volume)
}

fn parse_boolish(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "on" | "yes" | "true" | "enabled" => Some(true),
        "0" | "off" | "no" | "false" | "disabled" => Some(false),
        _ => None,
    }
}

fn extract_uuid(proxmox: &ProxmoxVmConfig) -> Option<String> {
    proxmox.scalars.get("uuid").cloned().or_else(|| {
        proxmox.scalars.get("smbios1").and_then(|value| {
            let (_, options) = parse_inline_options(value);
            options.get("uuid").cloned()
        })
    })
}

fn infer_vmid(proxmox: &ProxmoxVmConfig) -> Option<String> {
    if let Some(vmid) = proxmox.scalars.get("vmid") {
        if !vmid.is_empty() && vmid.chars().all(|c| c.is_ascii_digit()) {
            return Some(vmid.clone());
        }
    }

    for value in proxmox.scalars.values() {
        if let Some(vmid) = extract_vmid_from_text(value) {
            return Some(vmid);
        }
    }

    for disk in &proxmox.disks {
        if let Some(vmid) = extract_vmid_from_text(&disk.source) {
            return Some(vmid);
        }
    }

    None
}

fn extract_vmid_from_text(value: &str) -> Option<String> {
    let mut remaining = value;

    while let Some(pos) = remaining.find("vm-") {
        let candidate = &remaining[pos + 3..];
        let digits: String = candidate
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();

        if !digits.is_empty() {
            return Some(digits);
        }

        remaining = &candidate[0..];
    }

    None
}

fn pci_group_key(host: &str) -> String {
    ensure_pci_function(host)
        .rsplit_once('.')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or_else(|| ensure_pci_function(host))
}

fn vga_model(value: &str) -> String {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase()
}

fn s(input: &str) -> Value {
    Value::String(input.to_string())
}

#[cfg(test)]
mod tests {
    use super::{map_to_ezkvm_yaml, map_to_ezkvm_yaml_with_storage};
    use crate::import::proxmox_parser::parse_proxmox_config;
    use crate::import::proxmox_storage_parser::parse_proxmox_storage_config;
    use crate::vm::config::Config;

    #[test]
    fn test_map_and_validate_yaml() {
        let proxmox = parse_proxmox_config(
            r#"
            name: imported-ubuntu
            memory: 8192
            balloon: 1
            hugepages: 1024
            sockets: 1
            cores: 4
            cpu: host
            bios: ovmf
            machine: q35
            scsi0: /dev/vm1/vm-100-disk-0,discard=on,cache=none,aio=io_uring,serial=DISK100,werror=enospc
            net0: virtio=BC:24:11:FF:76:89,bridge=vmbr0,queues=4
            hostpci0: 0000:03:10.4,multifunction=on
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();
        assert_eq!(parsed.general().name(), "imported-ubuntu");
        assert!(result.yaml.contains("type: proxmox_tap"));
        assert!(result.yaml.contains("vmid: '100'") || result.yaml.contains("vmid: \"100\"") || result.yaml.contains("vmid: 100"));
        assert!(result.yaml.contains("balloon: true"));
        assert!(result.yaml.contains("hugepages: true"));
        assert!(result
            .yaml
            .contains("mem_path: /run/hugepages/kvm/1048576kB"));
        assert!(result.yaml.contains("prealloc: true"));
        assert!(result.yaml.contains("aio: io_uring"));
        assert!(result.yaml.contains("serial: DISK100"));
        assert!(result.yaml.contains("werror: enospc"));
        assert!(result.yaml.contains("queues: 4"));
        assert!(!result.mapped_keys.is_empty());
        assert!(result.yaml.contains("cpu: { cores: 4, sockets: 1, model: host"));
        assert!(result.yaml.contains("bios: { type: ovmf"));
        assert!(result.yaml.contains("- { type: hd, file: /dev/vm1/vm-100-disk-0"));
        assert!(result.yaml.contains("- { type: proxmox_tap, bridge: vmbr0"));
    }

    #[test]
    fn test_map_phase3_graphics_and_general_flags() {
        let proxmox = parse_proxmox_config(
            r#"
            name: desktop-vm
            agent: 1
            monitor: 0
            cpu: host
            bios: ovmf
            efidisk0: /dev/vm1/desktop-efidisk,efitype=4m,pre-enrolled-keys=1
            tpmstate0: /dev/vm1/desktop-tpmstate,version=v2.0
            rng0: source=/dev/random,max_bytes=1024,period=1000
            vga: virtio-gl
            args: -S -name "desktop vm"
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "desktop-vm");
        assert!(result.yaml.contains("type: virtio"));
        assert!(result.yaml.contains("type: remote-viewer"));
        assert!(result.yaml.contains("secure_boot: true"));
        assert!(result.yaml.contains("virtio_rng:"));
        assert!(result.yaml.contains("filename: /dev/random"));
        assert!(result.skipped_keys.contains(&"rng0.max_bytes".to_string()));
        assert!(result.skipped_keys.contains(&"rng0.period".to_string()));
        assert!(result.yaml.contains("- -name"));
        assert!(result.yaml.contains("- desktop vm"));
        assert!(result.yaml.contains("usb_tablet: true"));
    }

    #[test]
    fn test_map_serial0_socket_to_system_serial() {
        let proxmox = parse_proxmox_config(
            r#"
            name: serial-vm
            serial0: socket
            scsi0: vm0:vm-301-disk-0
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        assert!(result.yaml.contains("serial: { type: socket"));
        assert!(result
            .yaml
            .contains("path: /var/run/qemu-server/301.serial0"));
        assert!(result.mapped_keys.contains(&"serial0".to_string()));
    }

    #[test]
    fn test_map_tablet_zero_disables_usb_tablet() {
        let proxmox = parse_proxmox_config(
            r#"
            name: tablet-off-vm
            vga: virtio-gl
            tablet: 0
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        assert!(result.yaml.contains("display:"));
        assert!(result.yaml.contains("usb_tablet: false"));
        assert!(result.mapped_keys.contains(&"tablet".to_string()));
    }

    #[test]
    fn test_map_gpu_passthrough_from_x_vga_group() {
        let proxmox = parse_proxmox_config(
            r#"
            name: gaming-vm
            vga: none
            hostpci0: 0000:03:00.0,x-vga=1,multifunction=on
            hostpci1: 0000:03:00.1
            usb0: host=1-2
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "gaming-vm");
        assert!(result.yaml.contains("type: passthrough"));
        assert!(result.yaml.contains("type: no_display"));
        assert!(result.yaml.contains("vm_id: '0'") || result.yaml.contains("vm_id: \"0\"") || result.yaml.contains("vm_id: 0"));
        assert!(result.yaml.contains("vm_id: '0.1'") || result.yaml.contains("vm_id: \"0.1\"") || result.yaml.contains("vm_id: 0.1"));
        assert!(result.yaml.contains("port: '1'") || result.yaml.contains("port: \"1\"") || result.yaml.contains("port: 1"));
        assert!(result.yaml.contains("port: '2'") || result.yaml.contains("port: \"2\"") || result.yaml.contains("port: 2"));
        assert!(result.yaml.contains("usb:"));
        assert!(!result.yaml.contains("host:\n  pci:"));
    }

    #[test]
    fn test_map_usb_vendor_product_format() {
        let proxmox = parse_proxmox_config(
            r#"
            name: usb-vendor-vm
            usb0: host=0451:16a0
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "usb-vendor-vm");
        assert!(result.yaml.contains("vendor_id:"));
        assert!(result.yaml.contains("product_id:"));
        assert!(result.yaml.contains("0451"));
        assert!(result.yaml.contains("16a0"));
        assert!(result.mapped_keys.contains(&"usb0".to_string()));
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_map_host_pci_uses_slot_zero_and_port_per_index() {
        let proxmox = parse_proxmox_config(
            r#"
            name: host-pci-vm
            hostpci0: 0000:6e:00.0
            hostpci1: 0000:6e:00.1
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "host-pci-vm");
        assert!(result.yaml.contains("host:"));
        assert!(result.yaml.contains("pci:"));
        assert!(result.yaml.contains("vm_id: '0'") || result.yaml.contains("vm_id: \"0\"") || result.yaml.contains("vm_id: 0"));
        assert!(result.yaml.contains("vm_id: '0.1'") || result.yaml.contains("vm_id: \"0.1\"") || result.yaml.contains("vm_id: 0.1"));
        assert!(result.yaml.contains("port: '1'") || result.yaml.contains("port: \"1\"") || result.yaml.contains("port: 1"));
        assert!(result.yaml.contains("port: '2'") || result.yaml.contains("port: \"2\"") || result.yaml.contains("port: 2"));
        assert!(!result.yaml.contains("vm_id: '1'"));
        assert!(!result.yaml.contains("vm_id: '2'"));
    }

    #[test]
    fn test_map_scsihw_virtio_scsi_single_uses_correct_controller() {
        let proxmox = parse_proxmox_config(
            r#"
            name: scsi-single-vm
            scsihw: virtio-scsi-single
            scsi0: /dev/vg/disk0,discard=on
            scsi1: /dev/vg/disk1,cache=writeback
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        assert!(result.yaml.contains("controller: virtio-scsi-single"));
        assert!(!result.yaml.contains("controller: pvscsi"));
        assert!(result.mapped_keys.contains(&"scsihw".to_string()));
    }

    #[test]
    fn test_map_scsihw_default_uses_pvscsi() {
        let proxmox = parse_proxmox_config(
            r#"
            name: pvscsi-vm
            scsi0: /dev/vg/disk0,discard=on
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        assert!(result.yaml.contains("controller: pvscsi"));
        assert!(!result.yaml.contains("controller: virtio-scsi-single"));
    }

    #[test]
    fn test_map_boot_order_sets_boot_index() {
        let proxmox = parse_proxmox_config(
            r#"
            name: boot-order-vm
            boot: order=scsi2;net0;ide0
            scsi0: /dev/vg/disk0,discard=on
            scsi2: /dev/vg/disk2,discard=on
            ide0: local:iso/debian.iso,media=cdrom
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        // scsi2 is position 0 → boot_index: 1
        assert!(result.yaml.contains("boot_index: 1"));
        // scsi0 has no boot entry → no boot_index
        // Verify boot key is mapped, not skipped
        assert!(result.mapped_keys.contains(&"boot".to_string()));
        assert!(!result.skipped_keys.contains(&"boot".to_string()));
    }

    #[test]
    fn test_map_boot_order_single_drive() {
        let proxmox = parse_proxmox_config(
            r#"
            name: single-boot-vm
            boot: order=scsi0
            scsi0: /dev/vg/disk0
            scsi1: /dev/vg/disk1
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        // scsi0 is position 0 → boot_index: 1
        assert!(result.yaml.contains("boot_index: 1"));
        // only one boot_index entry
        assert_eq!(result.yaml.matches("boot_index:").count(), 1);
        assert!(result.mapped_keys.contains(&"boot".to_string()));
    }

    #[test]
    fn test_map_cpu_host_injects_paravirt_flags() {
        let proxmox = parse_proxmox_config(
            r#"
            name: paravirt-vm
            cpu: host
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        assert!(result.yaml.contains("+kvm_pv_eoi"));
        assert!(result.yaml.contains("+kvm_pv_unhalt"));
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("paravirt flags") && w.contains("kvm_pv_eoi")));
    }

    #[test]
    fn test_map_cpu_host_preserves_explicit_flags_and_adds_paravirt() {
        let proxmox = parse_proxmox_config(
            r#"
            name: paravirt-vm
            cpu: host,flags=+pdpe1gb;+md-clear
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        // Paravirt flags injected
        assert!(result.yaml.contains("+kvm_pv_eoi"));
        assert!(result.yaml.contains("+kvm_pv_unhalt"));
        // Explicit Proxmox flags preserved (semicolons converted to commas)
        assert!(result.yaml.contains("+pdpe1gb"));
        assert!(result.yaml.contains("+md-clear"));
        // No semicolons in the output
        assert!(!result.yaml.contains(';'));
    }

    #[test]
    fn test_map_cpu_non_host_no_paravirt_injection() {
        let proxmox = parse_proxmox_config(
            r#"
            name: standard-vm
            cpu: qemu64
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();

        assert!(!result.yaml.contains("kvm_pv_eoi"));
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.contains("paravirt flags")));
    }

    #[test]
    fn test_remote_viewer_generates_spice_tls_and_vnc() {
        let proxmox = parse_proxmox_config(
            r#"
            name: remote-viewer-vm
            vga: virtio-gl
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        
        // Verify remote-viewer display type is inferred
        assert!(result.yaml.contains("display:"));
        assert!(result.yaml.contains("type: remote-viewer"));
        
        // Verify SPICE is generated for remote-viewer with TLS defaults.
        assert!(result.yaml.contains("spice:"));
        assert!(result.yaml.contains("addr: 127.0.0.1"));
        assert!(result.yaml.contains("port: 0"));
        assert!(result.yaml.contains("tls_port: 61000"));
        assert!(result.yaml.contains("tls_ciphers: HIGH"));
        assert!(result.yaml.contains("seamless_migration: true"));
        assert!(result.yaml.contains("disable_ticketing: true"));

        // Verify VNC is also generated for management compatibility.
        assert!(result.yaml.contains("vnc:"));
        assert!(result.yaml.contains("password: true"));
    }

    #[test]
    fn test_map_resolves_lvmthin_sources_with_storage_cfg() {
        let proxmox = parse_proxmox_config(
            r#"
            name: imported-ubuntu
            bios: ovmf
            scsi0: vm0:vm-100-disk-0,discard=on
            efidisk0: boot:vm-100-efi,efitype=4m
            tpmstate0: ws0:vm-100-tpm,version=v2.0
            "#,
        )
        .unwrap();
        let storage = parse_proxmox_storage_config(
            r#"
            lvm: boot
                vgname boot
                content images,rootdir

            lvmthin: ws0
                thinpool pool
                vgname ws0
                content rootdir,images

            lvmthin: vm0
                thinpool pool
                vgname vm0
                content images,rootdir
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml_with_storage(&proxmox, Some(&storage), None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "imported-ubuntu");
        assert!(result.yaml.contains("file: /dev/vm0/vm-100-disk-0"));
        assert!(result.yaml.contains("file: /dev/boot/vm-100-efi"));
        assert!(result.yaml.contains("disk: /dev/ws0/vm-100-tpm"));
        assert!(!result
            .warnings
            .iter()
            .any(|warning| warning.contains("manual path translation")));
    }

    #[test]
    fn test_map_resolves_dir_storage_image_paths() {
        let proxmox = parse_proxmox_config(
            r#"
            name: iso-vm
            ide2: local:iso/debian.iso,media=cdrom
            scsi0: local:100/vm-100-disk-0.qcow2
            "#,
        )
        .unwrap();
        let storage = parse_proxmox_storage_config(
            r#"
            dir: local
                path /var/lib/vz
                content iso,vztmpl,images
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml_with_storage(&proxmox, Some(&storage), None).unwrap();

        assert!(result.yaml.contains("file: /var/lib/vz/iso/debian.iso"));
        assert!(result
            .yaml
            .contains("file: /var/lib/vz/images/100/vm-100-disk-0.qcow2"));
    }

    #[test]
    fn test_map_numa_with_hugepages_generates_numa_nodes() {
        let proxmox = parse_proxmox_config(
            r#"
            name: numa-vm
            numa: 1
            hugepages: 1024
            memory: 32768
            sockets: 1
            cores: 12
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "numa-vm");
        assert!(result.yaml.contains("numa_nodes:"), "expected numa_nodes in yaml");
        assert!(result.yaml.contains("nodeid:"), "expected nodeid field");
        assert!(result.yaml.contains("cpus:"), "expected cpus field");
        assert!(result.yaml.contains("mem:"), "expected mem field");
        assert!(result.mapped_keys.contains(&"numa".to_string()));
        assert!(result.warnings.is_empty(), "unexpected warnings: {:?}", result.warnings);
    }

    #[test]
    fn test_map_numa_without_hugepages_does_not_generate_numa_nodes() {
        let proxmox = parse_proxmox_config(
            r#"
            name: no-numa-vm
            numa: 1
            memory: 32768
            sockets: 1
            cores: 12
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        assert!(!result.yaml.contains("numa_nodes:"), "unexpected numa_nodes in yaml");
        assert!(result.mapped_keys.contains(&"numa".to_string()));
    }

    #[test]
    fn test_map_numa_with_two_sockets_splits_cpus() {
        let proxmox = parse_proxmox_config(
            r#"
            name: dual-socket-vm
            numa: 1
            hugepages: 1024
            memory: 32768
            sockets: 2
            cores: 6
            cpu: host
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        assert!(result.yaml.contains("numa_nodes:"));
        // Node 0: cpus 0-5, Node 1: cpus 6-11
        assert!(result.yaml.contains("0-5") || result.yaml.contains("\"0-5\""), "expected node 0 cpu range");
        assert!(result.yaml.contains("6-11") || result.yaml.contains("\"6-11\""), "expected node 1 cpu range");
    }

    #[test]
    fn test_map_vmgenid_from_proxmox() {
        let proxmox = parse_proxmox_config(
            r#"
            name: vmgenid-vm
            vmgenid: b42d5b83-fee2-47dc-98a8-7856b18542ec
            "#,
        )
        .unwrap();

        let result = map_to_ezkvm_yaml(&proxmox, None).unwrap();
        let parsed: Config = serde_yaml::from_str(&result.yaml).unwrap();

        assert_eq!(parsed.general().name(), "vmgenid-vm");
        assert!(result.yaml.contains("vmgenid:"));
        assert!(result.yaml.contains("b42d5b83-fee2-47dc-98a8-7856b18542ec"));
        assert!(result.mapped_keys.contains(&"vmgenid".to_string()));
    }
}

