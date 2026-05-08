//! Proxmox to canonical YAML mapping
//!
//! Maps Proxmox VM configurations (.conf format) to canonical ezkvm YAML
//! configuration. Handles device topology, boot order, firmware, TPM, and
//! runtime parity mode translation.

use super::error::ImportError;
use super::io::RuntimeTarget;
use super::model::{ProxmoxStorageConfig, ProxmoxVmConfig};
use crate::config::{
    AppleSmcConfig, AudioDeviceConfig, BallooningConfig, BootConfig, ControllersConfig,
    DeviceConfig, DisplayConfig, DriveConfig, GuestAgentConfig, HostConfig, HostPciConfig,
    HugepagesConfig, InputDeviceConfig, IommuConfig, IvshmemConfig, MemoryConfig,
    NetworkBackendConfig, NetworkConfig, NumaConfig, RtcConfig, SataControllerConfig,
    ScsiControllerConfig, SerialConfig, SmbiosConfig, SpiceConfig, SystemConfig, TpmConfig,
    UsbDeviceConfig, VmConfig, VmOptions, VncConfig,
};
use std::collections::BTreeSet;

mod devices;
mod helpers;
mod network;
mod profiles;
mod storage;
mod system;
mod topology;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingWarning {
    pub source_field: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalMappingResult {
    pub yaml: String,
    pub warnings: Vec<MappingWarning>,
}

pub fn map_proxmox_to_canonical_yaml(
    proxmox: &ProxmoxVmConfig,
    runtime_target: RuntimeTarget,
) -> Result<CanonicalMappingResult, ImportError> {
    map_proxmox_to_canonical_yaml_with_storage(proxmox, None, runtime_target)
}

pub fn map_proxmox_to_canonical_yaml_with_storage(
    proxmox: &ProxmoxVmConfig,
    storage_config: Option<&ProxmoxStorageConfig>,
    runtime_target: RuntimeTarget,
) -> Result<CanonicalMappingResult, ImportError> {
    let mut warnings = Vec::new();

    let name = proxmox
        .scalars
        .get("name")
        .cloned()
        .unwrap_or_else(|| "imported-vm".to_string());
    let architecture = system::map_architecture(proxmox.scalars.get("arch"), &mut warnings);
    let (mut machine, machine_options) =
        helpers::parse_machine_and_options(proxmox.scalars.get("machine"));
    let mut readconfig = Vec::new();
    let mut topology_planner =
        topology::Q35TopologyPlanner::new(proxmox, machine.as_str(), runtime_target);
    machine = topology_planner.apply_machine_and_readconfig(&machine, &mut readconfig);
    let legacy_root_bus = topology_planner.legacy_root_bus();
    let audio_bus = topology_planner.audio_controller_bus();
    let iommu = system::map_iommu(&machine_options, proxmox.scalars.get("args"));

    let memory = proxmox
        .scalars
        .get("memory")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2048);
    let hugepages = system::map_hugepages(&proxmox.scalars);
    let vcpus = system::map_vcpus(&proxmox.scalars);
    let (cpu_model, cpu_features) =
        helpers::parse_cpu_model_and_features(proxmox.scalars.get("cpu"));

    let firmware = proxmox
        .scalars
        .get("bios")
        .and_then(|bios| match bios.as_str() {
            "ovmf" => Some("uefi".to_string()),
            "seabios" => Some("bios".to_string()),
            _ => None,
        });

    // Determine Windows status for RTC and ballooning
    let ostype = proxmox.scalars.get("ostype").map(String::as_str);
    let is_windows = ostype.is_some_and(|ot| ot.starts_with("win"));

    let mut boot = BootConfig {
        firmware,
        ..Default::default()
    };
    system::apply_efidisk0_to_boot(&proxmox.scalars, storage_config, &mut boot, &mut warnings);

    // Parse boot order and SMBIOS early
    let boot_indices = system::parse_boot_order(&proxmox.scalars);
    let smbios_uuid = system::parse_smbios_uuid(&proxmox.scalars);

    let scsi_controllers =
        storage::map_scsi_controllers(&proxmox.scalars, &proxmox.disks, &mut warnings);
    let sata_controllers = storage::map_sata_controllers(&proxmox.disks);
    let inferred_vmid = helpers::infer_proxmox_vmid(proxmox);
    let mut drives = proxmox
        .disks
        .iter()
        .map(|disk| storage::map_drive(disk, storage_config))
        .collect::<Vec<_>>();
    let mut networks = proxmox
        .networks
        .iter()
        .map(|network| {
            network::map_network(
                network,
                inferred_vmid,
                is_windows,
                runtime_target,
                &mut warnings,
            )
        })
        .collect::<Vec<_>>();

    // Apply boot indices from parsed boot order
    for drive in &mut drives {
        if let Some(idx) = boot_indices.get(&drive.id) {
            drive.boot_index = Some(*idx);
        }
    }
    for network in &mut networks {
        if let Some(idx) = boot_indices.get(&network.id) {
            network.boot_index = Some(*idx);
        }
    }
    let displays = devices::map_displays(&proxmox.scalars, &mut warnings);
    let serials = devices::map_serials(
        &proxmox.scalars,
        inferred_vmid,
        &mut warnings,
        runtime_target,
    );
    let (audio, mut spice) =
        devices::map_audio_and_spice(&proxmox.scalars, audio_bus, &mut warnings);
    let mut vnc = None;
    let mut input_devices = Vec::new();
    let mut ivshmem = None;
    let mut applesmc = None;
    let mut smbios_type = 1u8;
    devices::apply_args_passthrough_subset(
        &proxmox.scalars,
        &mut devices::ArgsPassthroughOutput {
            spice: &mut spice,
            vnc: &mut vnc,
            input_devices: &mut input_devices,
            ivshmem: &mut ivshmem,
            applesmc: &mut applesmc,
            smbios_type: &mut smbios_type,
        },
        &mut warnings,
    );
    let explicit_host_functions = proxmox
        .host_pci
        .iter()
        .filter(|entry| {
            entry
                .host
                .rsplit_once(':')
                .is_some_and(|(_, slot)| slot.contains('.'))
        })
        .map(|entry| devices::normalize_host_pci_device(&entry.host))
        .collect::<BTreeSet<_>>();

    let mut host_pci = Vec::new();
    for entry in &proxmox.host_pci {
        let default_bus = topology_planner.allocate_hostpci_default_bus(entry, &mut warnings);
        host_pci.extend(devices::map_host_pci_entries(
            entry,
            &explicit_host_functions,
            default_bus,
        ));
    }
    let host_usb = proxmox
        .usb
        .iter()
        .map(|entry| devices::map_usb(entry, true))
        .collect::<Vec<_>>();

    let ballooning = if proxmox.scalars.contains_key("balloon") {
        system::map_ballooning(is_windows, legacy_root_bus)
    } else {
        None
    };

    let tpm = system::map_tpm(
        &proxmox.scalars,
        storage_config,
        inferred_vmid,
        runtime_target,
    );
    let guest_agent = system::map_guest_agent(
        &proxmox.scalars,
        inferred_vmid,
        runtime_target,
        legacy_root_bus,
    );

    let vm_generation_id = proxmox.scalars.get("vmgenid").cloned();
    let smbios = if smbios_uuid.is_some() || vm_generation_id.is_some() || smbios_type != 1 {
        Some(SmbiosConfig {
            smbios_type,
            manufacturer: None,
            product: None,
            version: None,
            serial: None,
            uuid: smbios_uuid,
            sku: None,
            family: None,
            vm_generation_id,
        })
    } else {
        None
    };

    let mut vm_config = VmConfig {
        name,
        backend: "qemu".to_string(),
        profiles: Vec::new(),
        system: SystemConfig {
            architecture,
            machine,
            machine_options,
            memory: MemoryConfig {
                size: memory,
                ballooning,
                ivshmem,
                hugepages: hugepages.clone(),
            },
            cpu: crate::config::CpuConfig {
                model: cpu_model,
                vcpus,
                features: cpu_features,
                numa: if hugepages.is_some() {
                    system::synthesize_hugepages_numa(memory, vcpus)
                } else {
                    Vec::new()
                },
            },
            boot,
            tpm,
            smbios,
            applesmc,
            readconfig,
        },
        devices: DeviceConfig {
            drives,
            networks,
            displays,
            serials,
            input: input_devices,
            audio,
        },
        controllers: ControllersConfig {
            scsi: scsi_controllers,
            sata: sata_controllers,
            xhci: Vec::new(),
        },
        host: HostConfig {
            pci: host_pci,
            usb: host_usb,
        },
        spice,
        vnc,
        iscsi_disks: Vec::new(),
        hyperv: None,
        iommu,
        options: VmOptions {
            rtc: if is_windows {
                Some(RtcConfig {
                    base: Some("localtime".to_string()),
                    driftfix: Some("slew".to_string()),
                })
            } else {
                None
            },
            pid_file: match (runtime_target, inferred_vmid) {
                (RuntimeTarget::ProxmoxParity, Some(id)) => {
                    Some(format!("/var/run/qemu-server/{}.pid", id))
                }
                _ => None,
            },
            guest_agent,
            ..VmOptions::default()
        },
    };

    vm_config.profiles = profiles::infer_profile_names(proxmox, &vm_config, runtime_target);

    let yaml = serde_yaml::to_string(&vm_config)
        .map_err(|e| ImportError::ParseError(format!("failed to serialize mapped config: {e}")))?;

    Ok(CanonicalMappingResult { yaml, warnings })
}
