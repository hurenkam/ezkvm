/// Build a RuntimeModel instance from a Proxmox configuration schema instance.
use std::{collections::HashMap, sync::Arc};

use crate::{
    config_format::proxmox::{
        ProxmoxConfigSchema,
        schema::{ProxmoxCompoundValue, ProxmoxOption, ProxmoxValue},
        storage_resolver::ProxmoxStorageConfig,
    },
    runtime_model::{
        BiosModel, BootModel, BusRegister, BusRegistrationApi, Chipset, Cpu, CpuModel, Display,
        DisplayModelBuilder, GuestAgent, GuestAgentModelBuilder, I440fxChipset, Memory,
        NetworkResource, PciDeviceApi, PciDeviceType, PcieAddress, PcieDeviceApi, PcieDeviceType,
        PvScsiController, Q35Chipset, RuntimeModel, RuntimeModelBuilder, ScsiAddress,
        ScsiControllerApi, ScsiDeviceBuilder, ScsiDeviceType, SeaBiosModel, StorageResource, Tpm,
        TpmModelBuilder, UefiModel, VirtioNetController,
    },
};

impl RuntimeModelBuilder {
    /// Converts a Proxmox configuration schema into a RuntimeModel instance.
    ///
    /// Maps Proxmox `.conf` fields to runtime components:
    /// - `name` → RuntimeModel.name
    /// - `memory` (MB) → Memory
    /// - `cores`, `sockets`, `cpu` → Cpu
    /// - `machine` (q35|i440fx) → Chipset
    /// - `bios` (seabios|ovmf) → BootModel
    ///
    /// # Errors
    /// Returns an error if required fields are missing or values cannot be parsed.
    pub fn build_from_proxmox_config(
        config: &ProxmoxConfigSchema,
        storage_config: &ProxmoxStorageConfig,
    ) -> Result<RuntimeModel, String> {
        let global = &config.global.entries;

        // Extract required fields with friendly error messages
        let name = extract_scalar_string(global, "name")?;
        let memory_mb = extract_scalar_u64(global, "memory")?;
        let machine = extract_machine_token(global)?;
        let cpu_model = extract_scalar_string(global, "cpu").unwrap_or_else(|_| "host".to_string());

        // Extract CPU topology with defaults
        let cores = extract_scalar_u8(global, "cores").unwrap_or(1);
        let sockets = extract_scalar_u8(global, "sockets").unwrap_or(1);

        // Determine chipset
        let mut bus_register = BusRegister::new();
        let chipset = match parse_machine_chipset(&machine) {
            Some(MachineChipset::Q35) => Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            Some(MachineChipset::I440fx) => Chipset::I440FX(I440fxChipset::new(&bus_register)),
            None => return Err(format!("Unsupported chipset: {}", machine)),
        };

        // Create CPU with parsed topology
        let cpu = Cpu::new(parse_cpu_model(&cpu_model), cores, 1, sockets);

        // Create Memory from megabytes
        let hugepages_kb = parse_hugepages_kb(global);
        let numa_enabled = parse_numa_enabled(global);
        let memory = Memory::megabytes(memory_mb as usize)
            .with_hugepages_kb(hugepages_kb)
            .with_numa_enabled(numa_enabled);

        let guest_agent = parse_guest_agent(global);
        let gpu = parse_gpu(global);
        let smbios_uuid = parse_smbios_uuid(global);
        let vmgenid = parse_vmgenid(global);
        // Proxmox implicitly enables SPICE when vga is QXL; infer a default SPICE
        // display when the conf has no explicit `spice:` field.
        let display = parse_display(global).or_else(|| {
            if matches!(gpu, Some(ProxmoxGpu::Qxl)) {
                serde_json::from_value(serde_json::json!({
                    "spice": {
                        "listen": "0.0.0.0",
                        "port": 5900,
                        "disable_ticketing": true
                    }
                }))
                .ok()
            } else {
                None
            }
        });

        let mut storage_resources: HashMap<String, StorageResource> = HashMap::new();
        let mut network_resources: HashMap<String, NetworkResource> = HashMap::new();

        if let Some(storage) = parse_storage_field(global.get("efidisk0"), storage_config) {
            storage_resources.insert("efidisk0".to_string(), storage);
        }

        if let Some(storage) = parse_storage_field(global.get("tpmstate0"), storage_config) {
            storage_resources.insert("tpmstate0".to_string(), storage);
        }

        let mut scsi_disks = collect_scsi_devices(global, storage_config)?;
        scsi_disks.sort_by_key(|(idx, _, _)| *idx);
        for (index, storage, _) in &scsi_disks {
            storage_resources.insert(format!("scsi{index}"), storage.clone());
        }

        let mut net_devices = collect_network_devices(global)?;
        net_devices.sort_by_key(|(idx, _)| *idx);
        for (index, net) in &net_devices {
            network_resources.insert(format!("net{index}"), net.clone());
        }

        let mut hostpci_devices = collect_hostpci_devices(global)?;
        hostpci_devices.sort_by_key(|(idx, _)| *idx);

        // Determine BIOS type from "bios" field
        let bios_model = match extract_scalar_string(global, "bios") {
            Ok(bios_type) if bios_type.as_str() == "ovmf" => storage_resources
                .get("efidisk0")
                .cloned()
                .map(UefiModel::new)
                .map(BiosModel::Uefi)
                .unwrap_or_else(|| BiosModel::SeaBios(SeaBiosModel::default())),
            _ => BiosModel::SeaBios(SeaBiosModel::default()),
        };

        let boot = BootModel::new(bios_model);

        let tpm = match storage_resources.get("tpmstate0") {
            Some(_) => Some(TpmModelBuilder::build(
                Tpm::Emulated {
                    swtpm: serde_json::from_value(serde_json::json!({
                        "version": 2.0,
                        "resource": "tpmstate0"
                    }))
                    .map_err(|e| format!("failed to build TPM metadata: {e}"))?,
                },
                &storage_resources,
            )?),
            None => None,
        };

        if !scsi_disks.is_empty() {
            let scsi_controller = Arc::new(PvScsiController::default());
            let pcie_addr = PcieAddress::new(0, 0);

            match bus_register.pcie_busses().get(&0) {
                Some(root) => {
                    let controller_as_pcie: Arc<dyn PcieDeviceApi> = scsi_controller.clone();
                    root.register_pcie_device(controller_as_pcie, Some(pcie_addr))?;
                }
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            }

            let scsi_bus_id = bus_register.register_scsi_bus(scsi_controller.clone())?;

            for (index, _storage, kind) in scsi_disks {
                let resource = format!("scsi{index}");
                let device_type = match kind {
                    ProxmoxScsiKind::Cdrom => ScsiDeviceType::Cdrom { resource },
                    ProxmoxScsiKind::Ssd => ScsiDeviceType::Ssd { resource },
                    ProxmoxScsiKind::Hdd => ScsiDeviceType::Hdd { resource },
                };
                let device = ScsiDeviceBuilder::build(&device_type, &storage_resources)?;
                let address = ScsiAddress::new(0, index as u8);
                scsi_controller.register_scsi_device(device, Some(address))?;
            }

            let _ = scsi_bus_id;
        }

        for (index, network_resource) in net_devices {
            let pcie_device: Arc<dyn PcieDeviceApi> =
                Arc::new(VirtioNetController::new(Some(network_resource.clone())));
            let preferred = Some(PcieAddress::new(0x10 + index as u8, 0));
            match bus_register.pcie_busses().get(&0) {
                Some(root) => root.register_pcie_device(pcie_device, preferred)?,
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            }
        }

        for (_index, passthrough) in hostpci_devices {
            match bus_register.pcie_busses().get(&0) {
                Some(root) => {
                    let device: Arc<dyn PcieDeviceApi> = (&passthrough).into();
                    root.register_pcie_device(device, None)?;
                }
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            }
        }

        match gpu {
            Some(ProxmoxGpu::Standard) => match bus_register.pcie_busses().get(&0) {
                Some(root) => {
                    let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::StandardGpu).into();
                    root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))?;
                }
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            },
            Some(ProxmoxGpu::Virtio) => match bus_register.pcie_busses().get(&0) {
                Some(root) => {
                    let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::VirtioGpu).into();
                    root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))?;
                }
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            },
            Some(ProxmoxGpu::Qxl) => {
                if let Some(root) = bus_register.pci_busses().get(&0) {
                    let device: Arc<dyn PciDeviceApi> = (&PciDeviceType::QxlGpu).into();
                    root.register_pci_device(device, None)?;
                } else if let Some(root) = bus_register.pcie_busses().get(&0) {
                    let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::StandardGpu).into();
                    root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))?;
                } else {
                    return Err("Neither PCI nor PCIe root bus exists".to_string());
                }
            }
            Some(ProxmoxGpu::Headless) | None => {}
        }

        // Create RuntimeModel with the builder
        Ok(RuntimeModel::new(
            name,
            cpu,
            memory,
            chipset,
            boot,
            smbios_uuid,
            vmgenid,
            tpm,
            display.map(DisplayModelBuilder::build),
            None,
            guest_agent.map(GuestAgentModelBuilder::build),
            bus_register,
        ))
    }
}

#[derive(Clone, Copy)]
enum ProxmoxGpu {
    Standard,
    Virtio,
    Qxl,
    Headless,
}

fn parse_guest_agent(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
) -> Option<GuestAgent> {
    let value = entries.get("agent")?;

    let enabled = match value {
        ProxmoxValue::Scalar { value } => {
            let token = value.trim();
            !(token == "0"
                || token.eq_ignore_ascii_case("no")
                || token.eq_ignore_ascii_case("false"))
        }
        ProxmoxValue::Compound(compound) => option_value(compound, "enabled")
            .map(|v| v != "0")
            .unwrap_or(true),
    };

    Some(GuestAgent { enabled })
}

fn parse_smbios_uuid(entries: &std::collections::BTreeMap<String, ProxmoxValue>) -> Option<String> {
    let value = entries.get("smbios1")?;

    match value {
        ProxmoxValue::Scalar { value } => {
            if let Some(uuid) = parse_uuid_token(value.as_str()) {
                return Some(uuid);
            }
        }
        ProxmoxValue::Compound(compound) => {
            if let Some(uuid) = parse_uuid_token(compound.head.as_str()) {
                return Some(uuid);
            }
            if let Some(uuid) = option_value(compound, "uuid")
                && !uuid.trim().is_empty()
            {
                return Some(uuid.trim().to_string());
            }
        }
    }

    None
}

fn parse_uuid_token(token: &str) -> Option<String> {
    let raw = token.trim();
    let uuid = raw.strip_prefix("uuid=")?.trim();
    if uuid.is_empty() {
        return None;
    }
    Some(uuid.to_string())
}

fn parse_vmgenid(entries: &std::collections::BTreeMap<String, ProxmoxValue>) -> Option<String> {
    let value = entries.get("vmgenid")?;
    let vmgenid = match value {
        ProxmoxValue::Scalar { value } => value.trim(),
        ProxmoxValue::Compound(compound) => compound.head.trim(),
    };

    if vmgenid.is_empty() {
        return None;
    }

    Some(vmgenid.to_string())
}

fn parse_hugepages_kb(entries: &std::collections::BTreeMap<String, ProxmoxValue>) -> Option<usize> {
    let value = entries.get("hugepages")?;
    let token = match value {
        ProxmoxValue::Scalar { value } => value.trim(),
        ProxmoxValue::Compound(compound) => compound.head.trim(),
    };

    let hugepages_mb = token.parse::<usize>().ok()?;
    if hugepages_mb == 0 {
        return None;
    }

    Some(hugepages_mb * 1024)
}

fn parse_numa_enabled(entries: &std::collections::BTreeMap<String, ProxmoxValue>) -> bool {
    let Some(value) = entries.get("numa") else {
        return false;
    };

    let token = match value {
        ProxmoxValue::Scalar { value } => value.as_str(),
        ProxmoxValue::Compound(compound) => compound.head.as_str(),
    };

    parse_proxmox_bool(token).unwrap_or(false)
}

fn parse_gpu(entries: &std::collections::BTreeMap<String, ProxmoxValue>) -> Option<ProxmoxGpu> {
    let value = entries.get("vga")?;
    let token = match value {
        ProxmoxValue::Scalar { value } => value.as_str(),
        ProxmoxValue::Compound(compound) => compound.head.as_str(),
    }
    .split(',')
    .next()
    .unwrap_or_default()
    .trim()
    .to_ascii_lowercase();

    if token.is_empty() {
        return Some(ProxmoxGpu::Standard);
    }
    if token == "none" || token == "serial0" {
        return Some(ProxmoxGpu::Headless);
    }
    if token.starts_with("qxl") {
        return Some(ProxmoxGpu::Qxl);
    }
    if token.starts_with("virtio") {
        return Some(ProxmoxGpu::Virtio);
    }

    Some(ProxmoxGpu::Standard)
}

fn parse_display(entries: &std::collections::BTreeMap<String, ProxmoxValue>) -> Option<Display> {
    if let Some(value) = entries.get("spice") {
        let spec = match value {
            ProxmoxValue::Scalar { value } => value.clone(),
            ProxmoxValue::Compound(compound) => {
                let mut items = vec![compound.head.clone()];
                for option in &compound.options {
                    match option {
                        ProxmoxOption::Flag { value } => items.push(value.clone()),
                        ProxmoxOption::KeyValue { key, value } => {
                            items.push(format!("{key}={value}"))
                        }
                    }
                }
                items.join(",")
            }
        };

        let mut listen = "0.0.0.0".to_string();
        let mut port: u16 = 5900;
        let mut disable_ticketing = false;

        for token in spec.split(',').map(str::trim) {
            if let Some(value) = token.strip_prefix("addr=") {
                listen = value.to_string();
            }
            if let Some(value) = token.strip_prefix("port=")
                && let Ok(parsed) = value.parse::<u16>()
            {
                port = parsed;
            }
            if let Some(value) = token.strip_prefix("tls-port=")
                && let Ok(parsed) = value.parse::<u16>()
            {
                port = parsed;
            }
            if token == "disable-ticketing=on" {
                disable_ticketing = true;
            }
        }

        return serde_json::from_value(serde_json::json!({
            "spice": {
                "listen": listen,
                "port": port,
                "disable_ticketing": disable_ticketing
            }
        }))
        .ok();
    }

    None
}

enum MachineChipset {
    Q35,
    I440fx,
}

fn parse_machine_chipset(machine: &str) -> Option<MachineChipset> {
    let token = machine.trim();

    if matches!(token, "q35" | "pc-q35") || token.starts_with("pc-q35-") {
        return Some(MachineChipset::Q35);
    }

    if matches!(token, "i440fx" | "pc-i440fx") || token.starts_with("pc-i440fx-") {
        return Some(MachineChipset::I440fx);
    }

    None
}

/// Parse CPU model string into CpuModel enum.
fn parse_cpu_model(model_str: &str) -> CpuModel {
    match model_str {
        "host" => CpuModel::Host,
        _ => CpuModel::Host, // Default to Host for unknown models
    }
}

#[derive(Clone, Copy)]
enum ProxmoxScsiKind {
    Hdd,
    Ssd,
    Cdrom,
}

fn collect_scsi_devices(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
    storage_config: &ProxmoxStorageConfig,
) -> Result<Vec<(usize, StorageResource, ProxmoxScsiKind)>, String> {
    let mut out = Vec::new();
    for (key, value) in entries {
        let Some(suffix) = key.strip_prefix("scsi") else {
            continue;
        };
        if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let index = suffix
            .parse::<usize>()
            .map_err(|_| format!("invalid scsi index in key '{key}'"))?;
        let Some(storage) = parse_storage_field(Some(value), storage_config) else {
            continue;
        };

        let kind = match value {
            ProxmoxValue::Compound(compound) => {
                if has_option(compound, "media", "cdrom") {
                    ProxmoxScsiKind::Cdrom
                } else if has_option(compound, "ssd", "1") {
                    ProxmoxScsiKind::Ssd
                } else {
                    ProxmoxScsiKind::Hdd
                }
            }
            ProxmoxValue::Scalar { .. } => ProxmoxScsiKind::Hdd,
        };

        out.push((index, storage, kind));
    }
    Ok(out)
}

fn collect_network_devices(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
) -> Result<Vec<(usize, NetworkResource)>, String> {
    let mut out = Vec::new();
    for (key, value) in entries {
        let Some(suffix) = key.strip_prefix("net") else {
            continue;
        };
        if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let index = suffix
            .parse::<usize>()
            .map_err(|_| format!("invalid network index in key '{key}'"))?;

        let network = match value {
            ProxmoxValue::Compound(compound) => {
                if let Some(bridge) = option_value(compound, "bridge") {
                    Some(NetworkResource::Bridge {
                        bridge: bridge.to_string(),
                    })
                } else {
                    option_value(compound, "ifname").map(|tap| NetworkResource::Tap {
                        tap: tap.to_string(),
                    })
                }
            }
            ProxmoxValue::Scalar { .. } => None,
        };

        if let Some(network) = network {
            out.push((index, network));
        }
    }
    Ok(out)
}

fn collect_hostpci_devices(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
) -> Result<Vec<(usize, PcieDeviceType)>, String> {
    let mut out = Vec::new();

    for (key, value) in entries {
        let Some(suffix) = key.strip_prefix("hostpci") else {
            continue;
        };
        if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }

        let index = suffix
            .parse::<usize>()
            .map_err(|_| format!("invalid hostpci index in key '{key}'"))?;

        let (host, resource, multifunction, rombar, romfile) = match value {
            ProxmoxValue::Scalar { value } => {
                let host = value.trim().to_string();
                if host.is_empty() {
                    continue;
                }
                (host, Some(key.clone()), None, None, None)
            }
            ProxmoxValue::Compound(compound) => {
                let host = compound.head.trim().to_string();
                if host.is_empty() {
                    continue;
                }

                let multifunction =
                    option_value(compound, "multifunction").and_then(parse_proxmox_bool);
                let rombar = option_value(compound, "rombar").and_then(parse_proxmox_bool);
                let romfile = option_value(compound, "romfile")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string);

                (host, Some(key.clone()), multifunction, rombar, romfile)
            }
        };

        out.push((
            index,
            PcieDeviceType::Passthrough {
                resource,
                host: Some(host),
                id: None,
                multifunction,
                rombar,
                romfile,
            },
        ));
    }

    Ok(out)
}

fn parse_proxmox_bool(value: &str) -> Option<bool> {
    let token = value.trim();
    if token.eq_ignore_ascii_case("1")
        || token.eq_ignore_ascii_case("on")
        || token.eq_ignore_ascii_case("yes")
        || token.eq_ignore_ascii_case("true")
    {
        return Some(true);
    }
    if token.eq_ignore_ascii_case("0")
        || token.eq_ignore_ascii_case("off")
        || token.eq_ignore_ascii_case("no")
        || token.eq_ignore_ascii_case("false")
    {
        return Some(false);
    }
    None
}

fn parse_storage_field(
    value: Option<&ProxmoxValue>,
    storage_config: &ProxmoxStorageConfig,
) -> Option<StorageResource> {
    let token = match value {
        Some(ProxmoxValue::Scalar { value }) => value.as_str(),
        Some(ProxmoxValue::Compound(compound)) => compound.head.as_str(),
        None => return None,
    };

    storage_config.token_to_resource(token)
}

fn option_value<'a>(compound: &'a ProxmoxCompoundValue, key: &str) -> Option<&'a str> {
    compound.options.iter().find_map(|option| match option {
        ProxmoxOption::KeyValue { key: k, value } if k == key => Some(value.as_str()),
        _ => None,
    })
}

fn has_option(compound: &ProxmoxCompoundValue, key: &str, expected: &str) -> bool {
    option_value(compound, key).is_some_and(|value| value == expected)
}

/// Extract a scalar string value from the schema entries.
fn extract_scalar_string(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
    key: &str,
) -> Result<String, String> {
    entries
        .get(key)
        .ok_or_else(|| format!("Required field '{}' not found", key))
        .and_then(|value| match value {
            ProxmoxValue::Scalar { value } => Ok(value.clone()),
            ProxmoxValue::Compound(_) => {
                Err(format!("Expected scalar value for '{}', got compound", key))
            }
        })
}

/// Extract and parse a scalar u64 value from the schema entries.
fn extract_scalar_u64(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
    key: &str,
) -> Result<u64, String> {
    extract_scalar_string(entries, key).and_then(|value| {
        value
            .parse::<u64>()
            .map_err(|_| format!("Field '{}' has non-numeric value: '{}'", key, value))
    })
}

fn extract_machine_token(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
) -> Result<String, String> {
    entries
        .get("machine")
        .ok_or_else(|| "Required field 'machine' not found".to_string())
        .map(|value| match value {
            ProxmoxValue::Scalar { value } => value.clone(),
            ProxmoxValue::Compound(compound) => compound.head.clone(),
        })
}

/// Extract and parse a scalar u8 value from the schema entries.
fn extract_scalar_u8(
    entries: &std::collections::BTreeMap<String, ProxmoxValue>,
    key: &str,
) -> Result<u8, String> {
    extract_scalar_string(entries, key).and_then(|value| {
        value
            .parse::<u8>()
            .map_err(|_| format!("Field '{}' has non-numeric value: '{}'", key, value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage_cfg() -> ProxmoxStorageConfig {
        ProxmoxStorageConfig::parse(
            r#"
dir: local
    path /var/lib/vz

lvmthin: vm1-pool
    vgname vm1

lvm: vm1
    vgname vm1
"#,
        )
        .expect("storage cfg should parse")
    }

    #[test]
    fn builds_runtime_model_from_basic_proxmox_config() {
        let config_text = r#"
name: test-vm
memory: 4096
machine: q35
cpu: host
cores: 4
sockets: 1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_ok(), "Should successfully build RuntimeModel");

        let model = result.unwrap();
        assert_eq!(model.name(), "test-vm");
    }

    #[test]
    fn builds_i440fx_chipset_when_specified() {
        let config_text = r#"
name: legacy-vm
memory: 2048
machine: i440fx
cpu: host
cores: 2
sockets: 1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_ok());
    }

    #[test]
    fn builds_q35_chipset_from_versioned_machine_value() {
        let config_text = r#"
name: versioned-q35-vm
memory: 2048
machine: pc-q35-8.1
cpu: host
cores: 2
sockets: 1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_ok());
    }

    #[test]
    fn builds_q35_chipset_from_compound_machine_value() {
        let config_text = r#"
name: compound-q35-vm
memory: 2048
machine: pc-q35-8.1,viommu=intel
cpu: host
cores: 2
sockets: 1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_missing_required_name_field() {
        let config_text = r#"
memory: 4096
machine: q35
cpu: host
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_err());
        match result {
            Err(e) => assert!(e.contains("name"), "Error should mention 'name': {}", e),
            Ok(_) => panic!("Should have failed due to missing name"),
        }
    }

    #[test]
    fn rejects_missing_required_memory_field() {
        let config_text = r#"
name: incomplete-vm
machine: q35
cpu: host
cores: 2
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_err());
        match result {
            Err(e) => assert!(e.contains("memory"), "Error should mention 'memory': {}", e),
            Ok(_) => panic!("Should have failed due to missing memory"),
        }
    }

    #[test]
    fn rejects_invalid_memory_value() {
        let config_text = r#"
name: bad-memory-vm
memory: not-a-number
machine: q35
cpu: host
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_err());
        match result {
            Err(e) => assert!(
                e.contains("non-numeric"),
                "Error should mention parsing issue: {}",
                e
            ),
            Ok(_) => panic!("Should have failed due to invalid memory value"),
        }
    }

    #[test]
    fn rejects_unsupported_chipset() {
        let config_text = r#"
name: unsupported-chipset-vm
memory: 4096
machine: unsupported
cpu: host
cores: 2
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(result.is_err());
        match result {
            Err(e) => assert!(
                e.contains("Unsupported chipset"),
                "Error should mention unsupported chipset: {}",
                e
            ),
            Ok(_) => panic!("Should have failed due to unsupported chipset"),
        }
    }

    #[test]
    fn uses_default_cpu_model_host() {
        let config_text = r#"
name: default-cpu-vm
memory: 4096
machine: q35
cores: 2
sockets: 1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(
            result.is_ok(),
            "Should use default CPU model when not specified"
        );
    }

    #[test]
    fn uses_default_cpu_topology() {
        let config_text = r#"
name: default-topology-vm
memory: 4096
machine: q35
cpu: host
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage);
        assert!(
            result.is_ok(),
            "Should use default CPU topology when not specified"
        );
    }

    #[test]
    fn maps_uefi_tpm_scsi_and_network_into_runtime() {
        let config_text = r#"
name: rich-vm
memory: 8192
machine: q35
cpu: host
cores: 4
sockets: 1
bios: ovmf
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,size=4M
tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
scsihw: pvscsi
scsi0: vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1
net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("rich proxmox config should map into runtime model");

        let command = model.qemu_command();
        assert!(command.iter().any(|arg| arg.contains("if=pflash,unit=1")));
        assert!(command.iter().any(|arg| arg.contains("tpm-tis")));
        assert!(command.iter().any(|arg| arg.contains("pvscsi")));
        assert!(command.iter().any(|arg| arg.contains("scsi-hd")));
        assert!(command.iter().any(|arg| arg.contains("br=vmbr0")));
    }

    #[test]
    fn maps_guest_agent_and_virtio_gpu_from_proxmox_fields() {
        let config_text = r#"
name: agent-gpu-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
agent: 1
vga: virtio,memory=128
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("should build runtime model");

        let command = model.qemu_command();
        assert!(command.iter().any(|arg| arg.contains("virtio-gpu-pci")));
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("org.qemu.guest_agent.0"))
        );
    }

    #[test]
    fn maps_qxl_gpu_and_spice_display_from_proxmox_fields() {
        let config_text = r#"
name: qxl-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
vga: qxl,memory=64
spice: port=5905,addr=127.0.0.1,disable-ticketing=on
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("should build runtime model");

        let command = model.qemu_command();
        assert!(command.iter().any(|arg| arg == "qxl" || arg == "VGA"));
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("port=5905,addr=127.0.0.1,disable-ticketing=on"))
        );
    }

    #[test]
    fn infers_spice_display_from_qxl_vga_when_no_spice_field() {
        let config_text = r#"
name: qxl-implicit-spice
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
vga: qxl,memory=64
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("should build runtime model");

        let command = model.qemu_command();
        // SPICE should be inferred even though no explicit spice: field was present
        assert!(
            command.iter().any(|arg| arg.contains("port=5900")),
            "expected inferred SPICE at port 5900, got: {:?}",
            command
        );
    }

    #[test]
    fn imports_identity_fields_from_proxmox_config() {
        let config_text = r#"
name: identity-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
smbios1: uuid=1f0f0f0f-1111-2222-3333-444444444444
vmgenid: 55555555-6666-7777-8888-999999999999
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("Should successfully build RuntimeModel");

        assert_eq!(
            model.smbios_uuid().as_deref(),
            Some("1f0f0f0f-1111-2222-3333-444444444444")
        );
        assert_eq!(
            model.vmgenid().as_deref(),
            Some("55555555-6666-7777-8888-999999999999")
        );
    }

    #[test]
    fn imports_smbios_uuid_when_not_first_token() {
        let config_text = r#"
name: identity-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
smbios1: manufacturer=acme,uuid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("Should successfully build RuntimeModel");

        assert_eq!(
            model.smbios_uuid().as_deref(),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
    }

    #[test]
    fn imports_hostpci_passthrough_devices() {
        let config_text = r#"
name: hostpci-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
hostpci0: 0000:0e:11.6,pcie=1,rombar=0
hostpci1: 0000:01:00.1,pcie=1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("Should successfully build RuntimeModel");

        let rendered = model.qemu_command();
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("vfio-pci,host=0000:0e:11.6"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("vfio-pci,host=0000:01:00.1"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("rombar=0")));
    }

    #[test]
    fn imports_hugepages_and_numa_memory_backend() {
        let config_text = r#"
name: numa-vm
memory: 16384
machine: q35
cpu: host
cores: 8
sockets: 1
hugepages: 1024
numa: 1
"#;
        let schema = ProxmoxConfigSchema::parse(config_text).expect("Failed to parse config");

        let storage = storage_cfg();
        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("Should successfully build RuntimeModel");

        let rendered = model.qemu_command();
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("memory-backend-file,id=ram-node0,size=16384M"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("mem-path=/run/hugepages/kvm/1048576kB"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("node,nodeid=0,cpus=0-7,memdev=ram-node0"))
        );
    }
}
