/// Build a RuntimeModel instance from a Proxmox configuration schema instance.
use std::{collections::HashMap, sync::Arc};

use crate::{
    config_format::proxmox::{
        ProxmoxConfigSchema,
        schema::{ProxmoxCompoundValue, ProxmoxOption, ProxmoxValue},
    },
    runtime_model::{
        BiosModel, BootModel, BusRegister, BusRegistrationApi, Chipset, Cpu, CpuModel,
        I440fxChipset, Memory, NetworkResource, PcieAddress, PcieDeviceApi, PvScsiController,
        Q35Chipset, RuntimeModel, RuntimeModelBuilder, ScsiAddress, ScsiControllerApi,
        ScsiDeviceBuilder, ScsiDeviceType, SeaBiosModel, StorageResource, Tpm, TpmModelBuilder,
        UefiModel, VirtioNetController,
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
    pub fn build_from_proxmox_config(config: &ProxmoxConfigSchema) -> Result<RuntimeModel, String> {
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
        let memory = Memory::megabytes(memory_mb as usize);

        let mut storage_resources: HashMap<String, StorageResource> = HashMap::new();
        let mut network_resources: HashMap<String, NetworkResource> = HashMap::new();

        if let Some(storage) = parse_storage_field(global.get("efidisk0")) {
            storage_resources.insert("efidisk0".to_string(), storage);
        }

        if let Some(storage) = parse_storage_field(global.get("tpmstate0")) {
            storage_resources.insert("tpmstate0".to_string(), storage);
        }

        let mut scsi_disks = collect_scsi_devices(global)?;
        scsi_disks.sort_by_key(|(idx, _, _)| *idx);
        for (index, storage, _) in &scsi_disks {
            storage_resources.insert(format!("scsi{index}"), storage.clone());
        }

        let mut net_devices = collect_network_devices(global)?;
        net_devices.sort_by_key(|(idx, _)| *idx);
        for (index, net) in &net_devices {
            network_resources.insert(format!("net{index}"), net.clone());
        }

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

        // Create RuntimeModel with the builder
        Ok(RuntimeModel::new(
            name,
            cpu,
            memory,
            chipset,
            boot,
            tpm,
            bus_register,
        ))
    }
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
        let Some(storage) = parse_storage_field(Some(value)) else {
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

fn parse_storage_field(value: Option<&ProxmoxValue>) -> Option<StorageResource> {
    let token = match value {
        Some(ProxmoxValue::Scalar { value }) => value.as_str(),
        Some(ProxmoxValue::Compound(compound)) => compound.head.as_str(),
        None => return None,
    };

    parse_proxmox_storage_token(token)
}

fn parse_proxmox_storage_token(token: &str) -> Option<StorageResource> {
    if token == "none" || token.is_empty() {
        return None;
    }

    if token.starts_with('/') {
        return Some(StorageResource::File {
            file: token.to_string(),
        });
    }

    let (storage_id, volume) = token.split_once(':')?;

    if storage_id == "local" {
        return Some(StorageResource::File {
            file: format!("/var/lib/vz/{volume}"),
        });
    }

    let vg = storage_id
        .strip_suffix("-pool")
        .unwrap_or(storage_id)
        .to_string();

    Some(StorageResource::BlockDevice {
        block_device: format!("/dev/{vg}/{volume}"),
    })
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let result = RuntimeModelBuilder::build_from_proxmox_config(&schema);
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

        let model = RuntimeModelBuilder::build_from_proxmox_config(&schema)
            .expect("rich proxmox config should map into runtime model");

        let command = model.qemu_command();
        assert!(command.iter().any(|arg| arg.contains("if=pflash,unit=1")));
        assert!(command.iter().any(|arg| arg.contains("tpm-tis")));
        assert!(command.iter().any(|arg| arg.contains("pvscsi")));
        assert!(command.iter().any(|arg| arg.contains("scsi-hd")));
        assert!(command.iter().any(|arg| arg.contains("br=vmbr0")));
    }
}
