use super::proxmox_model::{ProxmoxDiskEntry, ProxmoxHostPciEntry, ProxmoxVmConfig};
use super::report::EzkvmImportResult;
use super::ImportError;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn map_to_ezkvm_yaml(
    proxmox: &ProxmoxVmConfig,
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
        guest_uuid.as_deref(),
        &mut mapped_keys,
        &mut skipped_keys,
        &mut warnings,
    ) {
        system.insert(s("bios"), Value::Mapping(bios_map));
    }

    if let Some(tpm_map) = map_tpm(proxmox, &name, &mut mapped_keys, &mut skipped_keys, &mut warnings)
    {
        system.insert(s("tpm"), Value::Mapping(tpm_map));
    }

    if !system.is_empty() {
        root.insert(s("system"), Value::Mapping(system));
    }

    let (gpu_passthrough, host_passthrough) = split_gpu_passthrough_devices(&proxmox.host_pci);

    if let Some(spice) = map_spice(
        proxmox,
        &gpu_passthrough,
        &mut mapped_keys,
        &mut warnings,
    ) {
        root.insert(s("spice"), Value::Mapping(spice));
    }

    if let Some(vnc) = map_vnc(
        proxmox,
        &gpu_passthrough,
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

    let storage = map_storage(&proxmox.disks, &mut mapped_keys, &mut skipped_keys, &mut warnings);
    if !storage.is_empty() {
        root.insert(s("storage"), Value::Sequence(storage));
    }

    let network = map_network(proxmox, &mut mapped_keys, &mut skipped_keys, &mut warnings);
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
    let mut flags = Some(String::new());

    if let Some((key, parsed_model)) = model.split_once('=') {
        if key == "cputype" || key == "model" {
            model = parsed_model.trim().to_string();
        }
    }

    for token in tokens {
        if let Some((key, raw_value)) = token.split_once('=') {
            match key.trim() {
                "flags" => {
                    flags = Some(raw_value.trim().to_string());
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

    cpu.insert(s("model"), s(&model));
    cpu.insert(s("flags"), s(flags.as_deref().unwrap_or("")));
    mapped_keys.push("cpu".to_string());
}

fn map_bios(
    proxmox: &ProxmoxVmConfig,
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
            bios.insert(s("file"), s(&source));
            mapped_keys.push("efidisk0".to_string());

            if !source.starts_with('/') {
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
    name: &str,
    mapped_keys: &mut Vec<String>,
    skipped_keys: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<Mapping> {
    let raw = proxmox.scalars.get("tpmstate0")?;
    let (source, options) = parse_inline_options(raw);

    if !source.starts_with('/') {
        warnings.push(format!(
            "tpmstate0 source '{}' is not an absolute path; skipping typed tpm mapping",
            source
        ));
        skipped_keys.push("tpmstate0".to_string());
        return None;
    }

    let mut tpm = Mapping::new();
    tpm.insert(s("type"), s("swtpm"));
    tpm.insert(s("disk"), s(&source));
    tpm.insert(s("socket"), s(&format!("/var/ezkvm/{}-tpm.socket", name)));

    if let Some(version) = options.get("version") {
        tpm.insert(s("version"), s(version.trim_start_matches('v')));
    }

    mapped_keys.push("tpmstate0".to_string());
    Some(tpm)
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
        map.insert(s("vm_id"), s(&format!("{:x}", item.index + 1)));
        map.insert(s("host_id"), s(&ensure_pci_function(&item.host)));

        if let Some(mf) = item.options.get("multifunction").and_then(|value| parse_boolish(value)) {
            map.insert(s("multi_function"), Value::Bool(mf));
        }

        mapped_keys.push(item.key.clone());
        pci_items.push(Value::Mapping(map));
    }

    pci_items
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
            warnings.push(format!(
                "display inferred as remote-viewer from Proxmox vga '{}'",
                vga_model(vga)
            ));
            mapped_keys.push("display".to_string());
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

fn map_spice(
    proxmox: &ProxmoxVmConfig,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
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
            warnings.push(format!(
                "spice endpoint inferred with ezkvm defaults from Proxmox vga '{}'",
                vga_model(vga)
            ));
            mapped_keys.push("spice".to_string());
            Some(Mapping::new())
        }
        _ => None,
    }
}

fn map_vnc(
    proxmox: &ProxmoxVmConfig,
    gpu_passthrough: &[&ProxmoxHostPciEntry],
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
        "std" | "cirrus" => {
            warnings.push(format!(
                "vnc endpoint inferred with ezkvm defaults from Proxmox vga '{}'",
                vga_model(vga)
            ));
            mapped_keys.push("vnc".to_string());
            Some(Mapping::new())
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
        assert!(result.yaml.contains("- -name"));
        assert!(result.yaml.contains("- desktop vm"));
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
        assert!(result.yaml.contains("usb:"));
        assert!(!result.yaml.contains("host:\n  pci:"));
    }
}
