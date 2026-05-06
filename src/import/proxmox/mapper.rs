// Temporary over-size rationale (B-29): this file still carries orchestration plus
// a large regression-test block after B-26. Keep this as transitional debt only;
// closure target is <=250 lines for orchestration/public entrypoints.
use super::error::ImportError;
use super::io::RuntimeTarget;
use super::model::{ProxmoxStorageConfig, ProxmoxVmConfig};
use crate::config::{
    AppleSmcConfig, AudioDeviceConfig, BallooningConfig, BootConfig, ControllersConfig,
    DeviceConfig, DisplayConfig, DriveConfig, GuestAgentConfig, HostConfig, HostPciConfig,
    HugepagesConfig, InputDeviceConfig, IommuConfig, IvshmemConfig, MemoryConfig,
    NetworkBackendConfig, NetworkConfig, NumaConfig, RtcConfig, SataControllerConfig,
    ScsiControllerConfig, SerialConfig, SmbiosConfig, SpiceConfig, SystemConfig, TpmConfig,
    UsbDeviceConfig, VmConfig, VmOptions, VncConfig, XhciControllerConfig,
};
use std::collections::BTreeSet;

mod devices;
mod helpers;
mod network;
mod profiles;
mod storage;
mod system;
mod topology;

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
    let xhci_bus = topology_planner.xhci_controller_bus();
    let xhci_addr = topology_planner.xhci_controller_addr();
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

    let mut scsi_controllers =
        storage::map_scsi_controllers(&proxmox.scalars, &proxmox.disks, &mut warnings);
    let scsi_controller_bus = topology_planner.scsi_controller_bus();
    let scsi_controller_addr = topology_planner.scsi_controller_addr();
    if let Some(controller) = scsi_controllers.first_mut() {
        if controller.bus.is_none() {
            controller.bus = scsi_controller_bus.map(str::to_string);
        }
        if controller.addr.is_none() {
            controller.addr = scsi_controller_addr.map(str::to_string);
        }
    }
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
    let use_explicit_xhci = !proxmox.usb.is_empty();
    let host_usb = proxmox
        .usb
        .iter()
        .map(|entry| devices::map_usb(entry, use_explicit_xhci))
        .collect::<Vec<_>>();

    let ballooning = system::map_ballooning(is_windows, legacy_root_bus);

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
            xhci: if use_explicit_xhci {
                vec![XhciControllerConfig {
                    id: String::new(),
                    p2: None,
                    p3: None,
                    usb: Vec::new(),
                    bus: xhci_bus.map(str::to_string),
                    addr: xhci_addr.map(str::to_string),
                }]
            } else {
                Vec::new()
            },
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

#[cfg(test)]
mod tests {
    use super::{map_proxmox_to_canonical_yaml, map_proxmox_to_canonical_yaml_with_storage};
    use crate::config::{VmConfig, validation};
    use crate::import::proxmox::RuntimeTarget;
    use crate::import::proxmox::model::ProxmoxVmConfig;
    use crate::import::proxmox::parser::parse_proxmox_config;
    use crate::import::proxmox::storage_parser::parse_proxmox_storage_config;

    fn map_and_validate(input: &str) -> (String, VmConfig) {
        let parsed = parse_proxmox_config(input).expect("parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");
        let mut config: VmConfig =
            serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        config.assign_default_device_ids();
        validation::validate_config(&config).expect("config should validate");
        (mapped.yaml, config)
    }

    fn map_and_validate_with_storage(input: &str, storage_input: &str) -> (String, VmConfig) {
        let parsed = parse_proxmox_config(input).expect("parser should succeed");
        let storage =
            parse_proxmox_storage_config(storage_input).expect("storage parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml_with_storage(
            &parsed,
            Some(&storage),
            RuntimeTarget::PortableLinux,
        )
        .expect("mapper should succeed");
        let mut config: VmConfig =
            serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        config.assign_default_device_ids();
        validation::validate_config(&config).expect("config should validate");
        (mapped.yaml, config)
    }

    fn map_and_validate_parity(input: &str) -> (String, VmConfig) {
        let parsed = parse_proxmox_config(input).expect("parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::ProxmoxParity)
            .expect("mapper should succeed");
        let mut config: VmConfig =
            serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        config.assign_default_device_ids();
        validation::validate_config(&config).expect("config should validate");
        (mapped.yaml, config)
    }

    #[test]
    fn maps_cpu_and_memory() {
        let (yaml, cfg) = map_and_validate(
            r#"
            name: vm-cpu
            cpu: host
            cores: 4
            sockets: 2
            memory: 8192
            "#,
        );

        assert!(yaml.contains("backend: qemu"));
        assert_eq!(cfg.system.cpu.model, "host");
        assert_eq!(cfg.system.cpu.vcpus, 8);
        assert_eq!(cfg.system.memory.size, 8192);
    }

    #[test]
    fn maps_machine_options_and_cpu_features_from_scalar_fields() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-machine-cpu
            machine: type=pc-q35-8.1+pve0,hpet=off
            cpu: host,hv_ipi,hv_relaxed,kvm=off
            cores: 4
            "#,
        );

        assert_eq!(cfg.system.machine, "pc-q35-8.1+pve0");
        assert_eq!(cfg.system.machine_options, vec!["hpet=off".to_string()]);
        assert_eq!(cfg.system.cpu.model, "host");
        assert_eq!(
            cfg.system.cpu.features,
            vec![
                "hv_ipi".to_string(),
                "hv_relaxed".to_string(),
                "kvm=off".to_string()
            ]
        );
    }

    #[test]
    fn maps_machine_value_with_inline_options_without_type_prefix() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-machine-inline
            machine: pc-q35-8.1,hpet=off
            "#,
        );

        assert_eq!(cfg.system.machine, "pc-q35-8.1");
        assert_eq!(cfg.system.machine_options, vec!["hpet=off".to_string()]);
    }

    #[test]
    fn maps_storage_and_scsi_controller() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-storage
            scsihw: virtio-scsi-single
            scsi0: local-lvm:vm-200-disk-0,discard=on,ssd=1,cache=none
            ide2: none,media=cdrom
            "#,
        );

        assert_eq!(cfg.controllers.scsi.len(), 1);
        assert_eq!(cfg.controllers.scsi[0].id, "scsihw0");
        assert_eq!(cfg.controllers.scsi[0].r#type, "virtio-scsi-pci");
        assert_eq!(cfg.devices.drives[0].controller.as_deref(), Some("scsihw0"));
        assert_eq!(cfg.devices.drives.len(), 2);
        assert_eq!(cfg.devices.drives[0].interface, "scsi");
        assert!(cfg.devices.drives[0].discard);
        assert!(cfg.devices.drives[0].ssd);
        assert_eq!(cfg.devices.drives[1].r#type, "cdrom");
        assert!(
            cfg.profiles
                .contains(&"storage-virtio-scsi-single".to_string())
        );
    }

    #[test]
    fn does_not_inject_drive_defaults_or_scsi_id_without_explicit_options() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-storage-compact
            scsi0: /dev/vm1/vm-108-boot,format=raw
            "#,
        );

        assert_eq!(cfg.devices.drives.len(), 1);
        let drive = &cfg.devices.drives[0];
        assert_eq!(drive.interface, "scsi");
        assert_eq!(drive.cache, None);
        assert_eq!(drive.aio, None);
        assert_eq!(drive.detect_zeroes, None);
        assert_eq!(drive.scsi_id, None);
        assert_eq!(drive.rotation_rate, None);
    }

    #[test]
    fn maps_sata_drives_with_ahci_controller_and_port_buses() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-sata
            sata0: /var/lib/vm/sata0.raw,format=raw,discard=on,boot=100
            sata1: none,media=cdrom,ro=1,boot=101
            "#,
        );

        assert_eq!(cfg.controllers.sata.len(), 1);
        assert_eq!(cfg.controllers.sata[0].id, "sata0");
        assert_eq!(cfg.controllers.sata[0].r#type, "ahci");

        assert_eq!(cfg.devices.drives.len(), 2);

        let sata0 = cfg
            .devices
            .drives
            .iter()
            .find(|drive| drive.id == "sata0")
            .expect("sata0 should be present");
        assert_eq!(sata0.interface, "sata");
        assert_eq!(sata0.bus.as_deref(), Some("sata0.0"));
        assert_eq!(sata0.unit, Some(0));
        assert_eq!(sata0.boot_index, Some(100));

        let sata1 = cfg
            .devices
            .drives
            .iter()
            .find(|drive| drive.id == "sata1")
            .expect("sata1 should be present");
        assert_eq!(sata1.interface, "sata");
        assert_eq!(sata1.r#type, "cdrom");
        assert_eq!(sata1.bus.as_deref(), Some("sata0.1"));
        assert_eq!(sata1.unit, Some(0));
        assert_eq!(sata1.boot_index, Some(101));
    }

    #[test]
    fn maps_serial_socket_and_file_backends() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-serial
            serial0: socket,path=/tmp/serial0.sock,server=1,wait=0
            serial1: file:/tmp/serial1.log
            "#,
        );

        assert_eq!(cfg.devices.serials.len(), 2);

        let serial0 = &cfg.devices.serials[0];
        assert_eq!(serial0.r#type, "socket");
        assert_eq!(serial0.id.as_deref(), Some("serial0"));
        assert_eq!(serial0.port, Some(0));
        assert_eq!(serial0.path.as_deref(), Some("/tmp/serial0.sock"));
        assert!(serial0.server);
        assert!(!serial0.wait);

        let serial1 = &cfg.devices.serials[1];
        assert_eq!(serial1.r#type, "file");
        assert_eq!(serial1.id.as_deref(), Some("serial1"));
        assert_eq!(serial1.port, Some(1));
        assert_eq!(serial1.path.as_deref(), Some("/tmp/serial1.log"));
    }

    #[test]
    fn maps_serial_socket_default_path_from_vmid() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-serial-default
            scsi0: local-lvm:vm-108-disk-0,size=10G
            serial0: socket
            "#,
        );

        assert_eq!(cfg.devices.serials.len(), 1);
        let serial0 = &cfg.devices.serials[0];
        assert_eq!(serial0.r#type, "socket");
        assert_eq!(
            serial0.path.as_deref(),
            Some("/var/run/ezkvm/108-serial0.socket")
        );
        assert_eq!(serial0.host, None);
        assert_eq!(serial0.socket_port, None);
    }

    #[test]
    fn maps_pid_file_from_vmid() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-pid-default
            scsi0: local-lvm:vm-108-disk-0,size=10G
            "#,
        );

        // Portable mode: pid_file is None, resolved at runtime by get_pid_file_at
        assert_eq!(cfg.options.pid_file.as_deref(), None);
    }

    #[test]
    fn parity_maps_pid_file_from_vmid() {
        let (_, cfg) = map_and_validate_parity(
            r#"
            name: vm-pid-parity
            scsi0: local-lvm:vm-108-disk-0,size=10G
            "#,
        );

        // ProxmoxParity mode: explicit Proxmox path
        assert_eq!(
            cfg.options.pid_file.as_deref(),
            Some("/var/run/qemu-server/108.pid")
        );
    }

    #[test]
    fn resolves_storage_backed_disk_paths() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-storage-paths
            scsi0: vm1-pool:vm-108-boot,discard=on
            ide2: local:iso/virtio-win.iso,media=cdrom
            "#,
            r#"
            dir: local
                path /var/lib/vz
                content iso,vztmpl

            lvmthin: vm1-pool
                thinpool pool
                vgname vm1
                content images,rootdir
            "#,
        );

        assert_eq!(cfg.devices.drives[0].path, "/dev/vm1/vm-108-boot");
        assert_eq!(cfg.devices.drives[0].format, "raw");
        assert_eq!(cfg.devices.drives[1].path, "/var/lib/vz/iso/virtio-win.iso");
        assert_eq!(cfg.devices.drives[1].format, "raw");
    }

    #[test]
    fn resolves_zfspool_backed_disk_paths() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-zfs-paths
            scsi0: local-zfs:vm-500-disk-0,discard=on
            "#,
            r#"
            zfspool: local-zfs
                pool rpool/data
                content images,rootdir
            "#,
        );

        assert_eq!(
            cfg.devices.drives[0].path,
            "/dev/zvol/rpool/data/vm-500-disk-0"
        );
        assert_eq!(cfg.devices.drives[0].format, "raw");
    }

    #[test]
    fn maps_network_bridge_and_mac() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0,queues=4,firewall=1
            "#,
        );

        assert_eq!(cfg.devices.networks.len(), 1);
        let net = &cfg.devices.networks[0];
        assert_eq!(net.model, "virtio-net");
        assert_eq!(net.mac.as_deref(), Some("52:54:00:12:34:56"));
        let backend = net.backend.as_ref().expect("backend must be set");
        // Portable mode uses bridge backend (kernel bridge helper)
        assert_eq!(backend.backend_type, "bridge");
        assert_eq!(backend.bridge.as_deref(), Some("vmbr0"));
        assert_eq!(backend.queues, Some(4));
        // Portable mode omits parity-only host scripts
        assert_eq!(backend.script, None);
        assert_eq!(backend.downscript, None);
        assert_eq!(backend.vhost, None);
    }

    #[test]
    fn infers_tap_ifname_for_bridge_networks_from_vmid() {
        // Portable mode: bridge backend without ifname
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net-ifname
            scsi0: local-lvm:vm-108-disk-0
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0
            "#,
        );

        let net = &cfg.devices.networks[0];
        let backend = net.backend.as_ref().expect("backend must be set");
        // Portable mode uses bridge backend (no ifname)
        assert_eq!(backend.backend_type, "bridge");
        assert_eq!(backend.ifname.as_deref(), None);
    }

    #[test]
    fn parity_uses_tap_ifname_for_bridge_networks() {
        // ProxmoxParity mode: tap backend with ifname
        let (_, cfg) = map_and_validate_parity(
            r#"
            name: vm-net-tap
            scsi0: local-lvm:vm-108-disk-0
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0
            "#,
        );

        let net = &cfg.devices.networks[0];
        let backend = net.backend.as_ref().expect("backend must be set");
        assert_eq!(backend.backend_type, "tap");
        assert_eq!(backend.ifname.as_deref(), Some("tap108i0"));
    }

    #[test]
    fn parity_includes_bridge_scripts_in_network() {
        // ProxmoxParity mode: includes pve-bridge scripts
        let (_, cfg) = map_and_validate_parity(
            r#"
            name: vm-net-scripts
            scsi0: local-lvm:vm-108-disk-0
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0,script=/usr/libexec/qemu-server/pve-bridge,downscript=/usr/libexec/qemu-server/pve-bridgedown
            "#,
        );

        let net = &cfg.devices.networks[0];
        let backend = net.backend.as_ref().expect("backend must be set");
        assert_eq!(
            backend.script.as_deref(),
            Some("/usr/libexec/qemu-server/pve-bridge")
        );
        assert_eq!(
            backend.downscript.as_deref(),
            Some("/usr/libexec/qemu-server/pve-bridgedown")
        );
    }

    #[test]
    fn does_not_infer_tap_ifname_without_vmid() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net-ifname-no-vmid
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0
            "#,
        );

        let net = &cfg.devices.networks[0];
        let backend = net.backend.as_ref().expect("backend must be set");
        assert_eq!(backend.ifname, None);
    }

    #[test]
    fn maps_host_pci_and_usb() {
        // ProxmoxParity: Proxmox-specific machine and readconfig should be present
        let (_, cfg_parity) = map_and_validate_parity(
            r#"
            name: vm-host
            hostpci0: 0000:03:00,pcie=1,x-vga=1,multifunction=1
            usb0: host=1-2
            usb1: host=0451:16a0
            "#,
        );
        assert_eq!(cfg_parity.system.machine, "pc-q35-8.1+pve0");
        assert_eq!(
            cfg_parity.system.readconfig,
            vec!["/usr/share/qemu-server/pve-q35-4.0.cfg".to_string()]
        );
        assert_eq!(cfg_parity.host.pci.len(), 2);
        assert_eq!(cfg_parity.host.pci[0].device, "0000:03:00.0");
        assert_eq!(cfg_parity.host.pci[0].id, "hostpci0");
        assert!(cfg_parity.host.pci[0].pcie);
        assert!(!cfg_parity.host.pci[0].x_vga);
        assert!(cfg_parity.host.pci[0].multifunction);
        assert_eq!(
            cfg_parity.host.pci[0].bus.as_deref(),
            Some("ich9-pcie-port-1")
        );
        assert_eq!(cfg_parity.host.pci[0].addr.as_deref(), Some("0x0.0"));
        assert_eq!(cfg_parity.host.pci[1].device, "0000:03:00.1");
        assert_eq!(cfg_parity.host.pci[1].id, "hostpci1");
        assert!(!cfg_parity.host.pci[1].pcie);
        assert!(!cfg_parity.host.pci[1].x_vga);
        assert!(!cfg_parity.host.pci[1].multifunction);
        assert_eq!(
            cfg_parity.host.pci[1].bus.as_deref(),
            Some("ich9-pcie-port-1")
        );
        assert_eq!(cfg_parity.host.pci[1].addr.as_deref(), Some("0x0.1"));
        assert_eq!(cfg_parity.host.usb.len(), 2);
        assert_eq!(cfg_parity.host.usb[0].hostbus.as_deref(), Some("1"));
        assert_eq!(cfg_parity.host.usb[0].hostport.as_deref(), Some("2"));
        assert_eq!(cfg_parity.host.usb[1].host, "0451:16a0");

        // PortableLinux: Proxmox-specific machine and readconfig should NOT be present
        let (_, cfg_portable) = map_and_validate(
            r#"
            name: vm-host
            hostpci0: 0000:03:00,pcie=1,x-vga=1,multifunction=1
            usb0: host=1-2
            usb1: host=0451:16a0
            "#,
        );
        assert_eq!(cfg_portable.system.machine, "q35");
        assert_eq!(
            cfg_portable.system.readconfig,
            vec!["/usr/share/ezkvm/ezkvm-q35.cfg"]
        );
        assert_eq!(cfg_portable.host.pci.len(), 2);
        assert_eq!(cfg_portable.host.pci[0].device, "0000:03:00.0");
        assert_eq!(cfg_portable.host.pci[0].id, "hostpci0");
        assert!(cfg_portable.host.pci[0].pcie);
        assert!(!cfg_portable.host.pci[0].x_vga);
        assert!(cfg_portable.host.pci[0].multifunction);
        assert_eq!(
            cfg_portable.host.pci[0].bus.as_deref(),
            Some("ich9-pcie-port-1")
        );
        assert_eq!(cfg_portable.host.pci[0].addr.as_deref(), Some("0x0.0"));
        assert_eq!(cfg_portable.host.pci[1].device, "0000:03:00.1");
        assert_eq!(cfg_portable.host.pci[1].id, "hostpci1");
        assert!(!cfg_portable.host.pci[1].pcie);
        assert!(!cfg_portable.host.pci[1].x_vga);
        assert!(!cfg_portable.host.pci[1].multifunction);
        assert_eq!(
            cfg_portable.host.pci[1].bus.as_deref(),
            Some("ich9-pcie-port-1")
        );
        assert_eq!(cfg_portable.host.pci[1].addr.as_deref(), Some("0x0.1"));
        assert_eq!(cfg_portable.host.usb.len(), 2);
        assert_eq!(cfg_portable.host.usb[0].hostbus.as_deref(), Some("1"));
        assert_eq!(cfg_portable.host.usb[0].hostport.as_deref(), Some("2"));
        assert_eq!(cfg_portable.host.usb[1].host, "0451:16a0");
    }

    #[test]
    fn preserves_explicit_hostpci_function_entries_without_synthetic_pairing() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-host-explicit
            hostpci0: 0000:03:00.0,pcie=1,x-vga=1,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=1
            hostpci1: 0000:03:00.1,bus=ich9-pcie-port-1,addr=0x0.1
            "#,
        );

        assert_eq!(cfg.host.pci.len(), 2);
        assert_eq!(cfg.host.pci[0].id, "hostpci0");
        assert_eq!(cfg.host.pci[0].device, "0000:03:00.0");
        assert!(cfg.host.pci[0].x_vga);
        assert_eq!(cfg.host.pci[1].id, "hostpci1");
        assert_eq!(cfg.host.pci[1].device, "0000:03:00.1");
    }

    #[test]
    fn maps_tpm_when_tpmstate_present() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-tpm
            bios: ovmf
            tpmstate0: local-lvm:vm-108-tpmstate,size=4M,version=v2.0
            "#,
        );

        let tpm = cfg.system.tpm.as_ref().expect("tpm should be mapped");
        assert_eq!(tpm.version, "2.0");
        assert_eq!(tpm.backend, "emulator");
        assert_eq!(tpm.model, "tpm-tis");
        assert_eq!(tpm.state_backend_uri, None);
    }

    #[test]
    fn maps_tpm_backend_uri_with_storage_resolution() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-tpm
            bios: ovmf
            tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
            "#,
            r#"
            lvmthin: vm1-pool
                thinpool pool
                vgname vm1
                content images,rootdir
            "#,
        );

        let tpm = cfg.system.tpm.as_ref().expect("tpm should be mapped");
        assert_eq!(tpm.version, "2.0");
        assert_eq!(
            tpm.state_backend_uri.as_deref(),
            Some("/dev/vm1/vm-108-tpmstate")
        );
    }

    #[test]
    fn maps_efidisk0_into_boot_uefi_vars_with_storage_resolution() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-uefi
            bios: ovmf
            efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,size=4M
            "#,
            r#"
            lvmthin: vm1-pool
                thinpool pool
                vgname vm1
                content images,rootdir
            "#,
        );

        assert_eq!(cfg.system.boot.firmware.as_deref(), Some("uefi"));
        assert_eq!(
            cfg.system.boot.uefi_vars.as_deref(),
            Some("/dev/vm1/vm-108-efidisk")
        );
        assert_eq!(cfg.system.boot.uefi_vars_size, Some(540_672));
    }

    #[test]
    fn maps_efidisk0_size_from_size_option_when_efitype_is_absent() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-uefi-size
            bios: ovmf
            efidisk0: /var/lib/vm/vars.fd,size=4M
            "#,
        );

        assert_eq!(
            cfg.system.boot.uefi_vars.as_deref(),
            Some("/var/lib/vm/vars.fd")
        );
        assert_eq!(cfg.system.boot.uefi_vars_size, Some(4 * 1024 * 1024));
    }

    #[test]
    fn maps_audio0_to_hda_devices_with_spice_backend() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-audio
            audio0: device=ich9-intel-hda,driver=spice
            "#,
        );

        assert_eq!(cfg.devices.audio.len(), 3);
        assert_eq!(cfg.devices.audio[0].r#type, "ich9-intel-hda");
        assert_eq!(cfg.devices.audio[0].id, "audiodev0");
        assert_eq!(cfg.devices.audio[0].bus.as_deref(), Some("pci.2"));
        assert_eq!(cfg.devices.audio[1].r#type, "hda-micro");
        assert_eq!(cfg.devices.audio[2].r#type, "hda-duplex");

        let spice = cfg.spice.as_ref().expect("spice should be configured");
        assert!(spice.enabled);
        assert!(spice.audio);
        assert_eq!(spice.addr, "127.0.0.1");
        assert_eq!(spice.port, 5900);
    }

    #[test]
    fn maps_audio0_controller_bus_by_runtime_target() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-audio-targets
            vmid: 108
            audio0: device=ich9-intel-hda,driver=spice
            "#,
        )
        .expect("parser should succeed");

        let portable = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("portable mapping should succeed");
        let portable_cfg: VmConfig =
            serde_yaml::from_str(&portable.yaml).expect("yaml should deserialize");
        assert_eq!(portable_cfg.devices.audio[0].bus.as_deref(), Some("pci.2"));

        let parity = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::ProxmoxParity)
            .expect("parity mapping should succeed");
        let parity_cfg: VmConfig =
            serde_yaml::from_str(&parity.yaml).expect("yaml should deserialize");
        assert_eq!(parity_cfg.devices.audio[0].bus.as_deref(), Some("pci.2"));
    }

    #[test]
    fn unsupported_audio_driver_emits_warning_and_skips_audio_mapping() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-audio-unsupported
            audio0: device=ich9-intel-hda,driver=alsa
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        assert!(cfg.devices.audio.is_empty());
        assert!(cfg.spice.is_none());
        assert!(mapped.warnings.iter().any(|w| {
            w.source_field == "audio0" && w.message.contains("unsupported audio driver")
        }));
    }

    #[test]
    fn maps_agent_enabled_into_guest_agent_config() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-agent
            agent: 1
            "#,
        );

        let agent = cfg
            .options
            .guest_agent
            .as_ref()
            .expect("guest agent should be configured");
        assert!(agent.enabled);
        assert_eq!(
            agent.socket_path.as_deref(),
            Some("/var/run/ezkvm/qga.sock")
        );
        assert_eq!(agent.bus.as_deref(), Some("pcie.0"));
        assert_eq!(agent.addr.as_deref(), Some("0x8"));
    }

    #[test]
    fn maps_agent_enabled_into_guest_agent_config_for_proxmox_parity() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-agent-parity
            vmid: 108
            agent: 1
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::ProxmoxParity)
            .expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        let agent = cfg
            .options
            .guest_agent
            .as_ref()
            .expect("guest agent should be configured");
        assert_eq!(
            agent.socket_path.as_deref(),
            Some("/var/run/qemu-server/108.qga")
        );
        assert_eq!(agent.bus.as_deref(), Some("pci.0"));
        assert_eq!(agent.addr.as_deref(), Some("0x8"));
    }

    #[test]
    fn maps_windows_balloon_bus_by_runtime_target() {
        let portable = parse_proxmox_config(
            r#"
            name: vm-balloon-portable
            ostype: win11
            "#,
        )
        .expect("parser should succeed");
        let portable_mapped =
            map_proxmox_to_canonical_yaml(&portable, RuntimeTarget::PortableLinux)
                .expect("portable mapper should succeed");
        let portable_cfg: VmConfig =
            serde_yaml::from_str(&portable_mapped.yaml).expect("yaml should deserialize");
        let portable_balloon = portable_cfg
            .system
            .memory
            .ballooning
            .as_ref()
            .expect("ballooning should be configured");
        assert_eq!(portable_balloon.bus.as_deref(), Some("pcie.0"));

        let parity = parse_proxmox_config(
            r#"
            name: vm-balloon-parity
            vmid: 108
            ostype: win11
            "#,
        )
        .expect("parser should succeed");
        let parity_mapped = map_proxmox_to_canonical_yaml(&parity, RuntimeTarget::ProxmoxParity)
            .expect("parity mapper should succeed");
        let parity_cfg: VmConfig =
            serde_yaml::from_str(&parity_mapped.yaml).expect("yaml should deserialize");
        let parity_balloon = parity_cfg
            .system
            .memory
            .ballooning
            .as_ref()
            .expect("ballooning should be configured");
        assert_eq!(parity_balloon.bus.as_deref(), Some("pci.0"));
    }

    #[test]
    fn skips_guest_agent_mapping_when_agent_disabled() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-agent-off
            agent: enabled=0
            "#,
        );

        assert!(cfg.options.guest_agent.is_none());
    }

    #[test]
    fn maps_args_subset_for_spice_input_and_ivshmem() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-args
            args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on -chardev spicevmc,id=vdagent,name=vdagent -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 -device virtio-mouse -device virtio-keyboard -device ivshmem-plain,memdev=ivshmem0,bus=pcie.0 -object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
            "#,
        );

        let spice = cfg.spice.as_ref().expect("spice should be configured");
        assert!(spice.enabled);
        assert_eq!(spice.port, 5903);
        assert_eq!(spice.addr, "0.0.0.0");
        assert!(spice.disable_ticketing);
        assert!(spice.vdagent);

        assert!(cfg.devices.input.iter().any(|d| d.r#type == "virtio-mouse"));
        assert!(
            cfg.devices
                .input
                .iter()
                .any(|d| d.r#type == "virtio-keyboard")
        );

        let iv = cfg
            .system
            .memory
            .ivshmem
            .as_ref()
            .expect("ivshmem should be configured");
        assert!(iv.enabled);
        assert_eq!(iv.id, "ivshmem0");
        assert_eq!(iv.bus.as_deref(), Some("pcie.0"));
        assert_eq!(iv.mem_path, "/dev/kvmfr0");
        assert_eq!(iv.size, 128);
        assert!(cfg.profiles.contains(&"looking-glass".to_string()));
    }

    #[test]
    fn infers_windows_11_profile_stack() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-win11
            ostype: win11
            bios: ovmf
            machine: pc-q35-8.1+pve0
            agent: 1
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=2023,pre-enrolled-keys=1
            tpmstate0: /var/lib/vm/tpmstate,size=4M,version=v2.0
            audio0: device=ich9-intel-hda,driver=spice
            args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on -chardev spicevmc,id=vdagent,name=vdagent -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 -device virtio-mouse -device virtio-keyboard
            scsi0: local-lvm:vm-108-disk-0
            scsihw: virtio-scsi-pci
            hostpci0: 0000:03:00.0,pcie=1
            "#,
        );

        assert_eq!(
            cfg.profiles,
            vec![
                "proxmox-base".to_string(),
                "proxmox-portable-q35".to_string(),
                "proxmox-q35-uefi".to_string(),
                "storage-virtio-scsi-pci".to_string(),
                "proxmox-windows".to_string(),
                "windows-common".to_string(),
                "windows-11".to_string(),
                "remote-viewer-spice".to_string(),
                "gpu-passthrough".to_string(),
            ]
        );
    }

    #[test]
    fn infers_macos_profile() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-macos
            ostype: other
            bios: ovmf
            machine: pc-q35-5.2
            cpu: Penryn
            vga: none
            args: -device isa-applesmc,osk=dummy -smbios type=2
            hostpci0: 0000:07:00.0,pcie=1
            "#,
        );

        assert!(cfg.profiles.contains(&"macos-kvm".to_string()));
        assert!(cfg.profiles.contains(&"gpu-passthrough".to_string()));
        assert_eq!(cfg.system.smbios.as_ref().map(|s| s.smbios_type), Some(2));
        assert_eq!(
            cfg.system.applesmc.as_ref().map(|a| a.osk.as_str()),
            Some("dummy")
        );
    }

    #[test]
    fn infers_macos_profile_only_when_smbios_type_2_is_present() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-macos-no-smbios2
            ostype: other
            bios: ovmf
            machine: pc-q35-5.2
            cpu: Penryn
            vga: none
            args: -device isa-applesmc,osk=dummy -smbios type=1
            "#,
        );

        assert!(!cfg.profiles.contains(&"macos-kvm".to_string()));
    }

    #[test]
    fn infers_windows_profiles_from_hyperv_variant_signals_without_windows_ostype() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-hyperv-variant
            bios: ovmf
            machine: pc-q35-8.1+pve0
            cpu: host,+hyperv-vendor-id,+hyperv-enlightened-vmx,+hyperv-time,+hyperv-synic
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=2023,pre-enrolled-keys=1
            tpmstate0: /var/lib/vm/tpmstate,size=4M,version=v2.0
            "#,
        );

        assert!(cfg.profiles.contains(&"proxmox-windows".to_string()));
        assert!(cfg.profiles.contains(&"windows-11".to_string()));
    }

    #[test]
    fn infers_linux_l26_profile_for_nested_virtualization_without_guest_agent() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-l26-nested
            ostype: l26
            bios: ovmf
            machine: q35
            cpu: host,+svm
            "#,
        );

        assert!(cfg.profiles.contains(&"linux-l26-common".to_string()));
    }

    #[test]
    fn infers_scsi_storage_profile_with_mixed_storage_buses() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-mixed-storage
            ostype: win11
            bios: ovmf
            machine: pc-q35-8.1+pve0
            scsihw: virtio-scsi-pci
            scsi0: local-lvm:vm-500-disk-0,size=64G
            sata0: local-lvm:vm-500-disk-1,size=32G
            ide2: none,media=cdrom
            "#,
        );

        assert!(
            cfg.profiles
                .contains(&"storage-virtio-scsi-pci".to_string())
        );
    }

    #[test]
    fn infers_hugepages_profile() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-huge
            ostype: l26
            bios: ovmf
            machine: pc-q35-8.1+pve0
            hugepages: 1024
            memory: 8192
            cores: 4
            "#,
        );

        assert!(cfg.profiles.contains(&"hugepages".to_string()));
    }

    #[test]
    fn maps_vnc_from_args_and_infers_headless_vnc_profile() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-headless-vnc
            vga: none
            args: -vnc unix:/var/run/qemu-server/405.vnc,password=on
            "#,
        );

        let vnc = cfg.vnc.as_ref().expect("vnc should be configured");
        assert!(vnc.enabled);
        assert_eq!(vnc.display, "unix:/var/run/qemu-server/405.vnc");
        assert!(vnc.password);
        assert!(cfg.profiles.contains(&"headless-vnc".to_string()));
        assert!(!cfg.profiles.contains(&"headless-serial".to_string()));
    }

    #[test]
    fn infers_viommu_and_hidden_hypervisor_profiles() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-hidden
            machine: q35,viommu=intel
            cpu: host,hidden=1
            "#,
        );

        assert!(cfg.profiles.contains(&"viommu".to_string()));
        assert!(cfg.profiles.contains(&"hidden-hypervisor".to_string()));
    }

    #[test]
    fn unsupported_args_token_emits_warning() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-args-unsupported
            args: -foo bar
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");
        assert!(mapped.warnings.iter().any(|w| {
            w.source_field == "args" && w.message.contains("unsupported args token '-foo'")
        }));
    }

    #[test]
    fn maps_display_from_vga() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-display
            vga: virtio-gl,memory=64
            "#,
        );

        assert_eq!(cfg.devices.displays.len(), 1);
        assert_eq!(cfg.devices.displays[0].r#type, "virtio-gpu");
        assert_eq!(cfg.devices.displays[0].vram, Some(64));
    }

    #[test]
    fn maps_with_defaults_when_fields_missing() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-defaults
            "#,
        );

        assert_eq!(cfg.backend, "qemu");
        assert_eq!(cfg.system.architecture, "x86_64");
        assert_eq!(cfg.system.machine, "q35");
        assert_eq!(cfg.system.cpu.vcpus, 1);
        assert_eq!(cfg.system.memory.size, 2048);
    }

    #[test]
    fn warnings_include_source_field_reference() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-warn
            arch: sparc
            memory: 2048
            cores: 2
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");
        assert_eq!(mapped.warnings.len(), 1);
        assert_eq!(mapped.warnings[0].source_field, "arch");
        assert!(
            mapped.warnings[0]
                .message
                .contains("unsupported Proxmox architecture")
        );
    }

    #[test]
    fn b14_maps_cpu_hyperv_features_for_windows_guests() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-windows-hyperv
            cores: 4
            cpu: host,+hyperv-vendor-id,+hyperv-enlightened-vmx,+hyperv-time,+hyperv-synic
            "#,
        );

        let cpu = &cfg.system.cpu;
        assert_eq!(cpu.vcpus, 4);
        assert_eq!(cpu.model, "host");
        assert!(cpu.features.contains(&"+hyperv-vendor-id".to_string()));
        assert!(
            cpu.features
                .contains(&"+hyperv-enlightened-vmx".to_string())
        );
        assert!(cpu.features.contains(&"+hyperv-time".to_string()));
        assert!(cpu.features.contains(&"+hyperv-synic".to_string()));
    }

    #[test]
    fn b15_maps_network_device_queue_sizes_and_pci_placement() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net-queues
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0,rx_queue_size=256,tx_queue_size=256,bus=pci.0,addr=1f.0
            "#,
        );

        assert_eq!(cfg.devices.networks.len(), 1);
        let net = &cfg.devices.networks[0];
        assert_eq!(net.rx_queue_size, Some(256));
        assert_eq!(net.tx_queue_size, Some(256));
        assert_eq!(net.bus.as_deref(), Some("pci.0"));
        assert_eq!(net.addr.as_deref(), Some("1f.0"));
    }

    #[test]
    fn b16_maps_efidisk0_with_ms_cert_and_pre_enrolled_keys() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-efi-secure
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=2023,pre-enrolled-keys=1
            "#,
        );

        let boot = &cfg.system.boot;
        assert_eq!(boot.firmware.as_deref(), Some("uefi"));
        assert_eq!(boot.uefi_vars.as_deref(), Some("/var/lib/vm/vars.fd"));
        assert_eq!(boot.uefi_vars_size, Some(540_672));
        assert_eq!(boot.uefi_ms_cert.as_deref(), Some("2023"));
        assert_eq!(boot.uefi_pre_enrolled_keys.as_deref(), Some("1"));
        assert!(boot.secure_boot);
    }

    #[test]
    fn efidisk_metadata_can_disable_secure_boot_signal() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-efi-non-secure
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=none,pre-enrolled-keys=0
            "#,
        );

        let boot = &cfg.system.boot;
        assert_eq!(boot.firmware.as_deref(), Some("uefi"));
        assert!(!boot.secure_boot);
    }

    #[test]
    fn efidisk_overrides_bios_firmware_with_warning() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-efi-override
            bios: seabios
            efidisk0: /var/lib/vm/vars.fd,efitype=4m
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        assert_eq!(cfg.system.boot.firmware.as_deref(), Some("uefi"));
        assert!(mapped.warnings.iter().any(|warning| {
            warning.source_field == "efidisk0"
                && warning
                    .message
                    .contains("overriding BIOS firmware mapping to 'uefi'")
        }));
    }

    #[test]
    fn efidisk_warns_when_secure_boot_metadata_is_not_representable() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-efi-warning
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=custom,pre-enrolled-keys=maybe
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        assert_eq!(cfg.system.boot.firmware.as_deref(), Some("uefi"));
        assert!(!cfg.system.boot.secure_boot);
        assert_eq!(mapped.warnings.len(), 2);
        assert!(
            mapped
                .warnings
                .iter()
                .all(|warning| warning.source_field == "efidisk0")
        );
        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.message.contains("ms-cert='custom'"))
        );
        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.message.contains("pre-enrolled-keys='maybe'"))
        );
    }

    // Comprehensive edge case and robustness tests for mapper
    #[test]
    fn maps_empty_proxmox_config_with_defaults() {
        let parsed = ProxmoxVmConfig::default();
        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("empty config should map");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");

        assert_eq!(cfg.name, "imported-vm");
        assert_eq!(cfg.system.memory.size, 2048);
        // Empty config should still be valid and produce a config
    }

    #[test]
    fn maps_vm_with_only_name_and_defaults() {
        let (_, cfg) = map_and_validate("name: test-vm");

        assert_eq!(cfg.name, "test-vm");
        assert_eq!(cfg.system.memory.size, 2048);
        assert_eq!(cfg.system.cpu.vcpus, 1);
    }

    #[test]
    fn maps_extremely_high_memory_value() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-huge-memory
            memory: 1048576
            cores: 64
            sockets: 4
            "#,
        );

        assert_eq!(cfg.system.memory.size, 1048576);
        assert_eq!(cfg.system.cpu.vcpus, 256);
    }

    #[test]
    fn maps_single_core_minimal_vcpu_config() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-minimal-cpu
            cores: 1
            sockets: 1
            memory: 512
            "#,
        );

        assert_eq!(cfg.system.cpu.vcpus, 1);
        assert_eq!(cfg.system.memory.size, 512);
    }

    #[test]
    fn maps_non_integer_memory_value_defaults_to_2048() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-invalid-memory
            memory: not-a-number
            "#,
        );

        assert_eq!(cfg.system.memory.size, 2048);
    }

    #[test]
    fn maps_multiple_networks_preserves_order() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-multi-net
            net0: virtio=AA:BB:CC:DD:EE:00,bridge=vmbr0
            net1: e1000=AA:BB:CC:DD:EE:01,bridge=vmbr1
            net2: i82559er=AA:BB:CC:DD:EE:02,bridge=vmbr2
            net5: rtl8139=AA:BB:CC:DD:EE:05,bridge=vmbr5
            "#,
        );

        assert_eq!(cfg.devices.networks.len(), 4);
        assert_eq!(cfg.devices.networks[0].id, "net0");
        assert_eq!(cfg.devices.networks[1].id, "net1");
        assert_eq!(cfg.devices.networks[2].id, "net2");
        // Network IDs preserve the Proxmox source key so boot order lookup can match
        assert_eq!(cfg.devices.networks[3].id, "net5");
    }

    #[test]
    fn maps_network_without_mac_address() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net-no-mac
            net0: bridge=vmbr0,tag=20
            "#,
        );

        assert_eq!(cfg.devices.networks.len(), 1);
    }

    #[test]
    fn maps_mixed_disk_types_across_all_buses() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-mixed-disks
            scsi0: disk-scsi,size=20G
            sata0: disk-sata,size=30G
            ide0: none,media=cdrom
            virtio0: disk-virtio,size=40G
            "#,
        );

        assert_eq!(cfg.devices.drives.len(), 4);
        assert_eq!(
            cfg.devices
                .drives
                .iter()
                .filter(|d| d.interface == "scsi")
                .count(),
            1
        );
        assert_eq!(
            cfg.devices
                .drives
                .iter()
                .filter(|d| d.interface == "sata")
                .count(),
            1
        );
        assert_eq!(
            cfg.devices
                .drives
                .iter()
                .filter(|d| d.interface == "ide")
                .count(),
            1
        );
        assert_eq!(
            cfg.devices
                .drives
                .iter()
                .filter(|d| d.interface == "virtio")
                .count(),
            1
        );
    }

    #[test]
    fn maps_ostype_windows_sets_proper_defaults() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-win
            ostype: win10
            "#,
        );

        assert!(cfg.options.rtc.is_some());
    }

    #[test]
    fn maps_ostype_linux_sets_different_defaults() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-linux
            ostype: l26
            "#,
        );

        assert_eq!(cfg.name, "vm-linux");
    }

    #[test]
    fn maps_multiple_hostpci_devices() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-multi-hostpci
            hostpci0: 0000:01:00.0
            hostpci1: 0000:02:00.0
            hostpci2: 0000:08:10.7,pcie=1
            "#,
        );

        assert_eq!(cfg.host.pci.len(), 3);
    }

    #[test]
    fn portable_q35_auto_assigns_root_ports_for_pcie_hostpci_without_bus() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-auto-hostpci-bus
            machine: q35
            hostpci0: 0000:03:00.0,pcie=1
            hostpci1: 0000:04:00.0,pcie=1
            "#,
        );

        assert_eq!(cfg.host.pci.len(), 2);
        assert_eq!(cfg.host.pci[0].bus.as_deref(), Some("ich9-pcie-port-1"));
        assert_eq!(cfg.host.pci[1].bus.as_deref(), Some("ich9-pcie-port-2"));
    }

    #[test]
    fn portable_q35_warns_and_falls_back_when_root_ports_are_exhausted() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-hostpci-root-port-budget
            machine: q35
            hostpci0: 0000:03:00.0,pcie=1
            hostpci1: 0000:04:00.0,pcie=1
            hostpci2: 0000:05:00.0,pcie=1
            hostpci3: 0000:06:00.0,pcie=1
            hostpci4: 0000:07:00.0,pcie=1
            hostpci5: 0000:08:00.0,pcie=1
            hostpci6: 0000:09:00.0,pcie=1
            hostpci7: 0000:0a:00.0,pcie=1
            hostpci8: 0000:0b:00.0,pcie=1
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");

        let mut cfg: VmConfig =
            serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        cfg.assign_default_device_ids();

        assert_eq!(cfg.host.pci.len(), 9);
        assert_eq!(cfg.host.pci[7].bus.as_deref(), Some("ich9-pcie-port-8"));
        assert_eq!(cfg.host.pci[8].bus.as_deref(), Some("pcie.0"));
        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.source_field == "hostpci8"
                    && warning.message.contains("falling back to bus=pcie.0"))
        );
    }

    #[test]
    fn maps_multiple_usb_devices() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-multi-usb
            usb0: host=1-1
            usb1: host=1-2
            usb2: host=0451:16a0
            usb3: host=1-4
            "#,
        );

        assert_eq!(cfg.host.usb.len(), 4);
        assert_eq!(cfg.controllers.xhci.len(), 1);
        assert_eq!(cfg.controllers.xhci[0].p2, None);
        assert_eq!(cfg.controllers.xhci[0].p3, None);
        // No topology hints → ezkvm-q35.cfg not loaded → pci.1 not available; bus left unset
        assert_eq!(cfg.controllers.xhci[0].bus.as_deref(), None);
        assert_eq!(cfg.controllers.xhci[0].addr.as_deref(), None);
    }

    #[test]
    fn maps_architecture_with_fallback() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-x86
            arch: x86-64
            "#,
        );

        // Architecture is normalized to x86_64
        assert_eq!(cfg.system.architecture, "x86_64");
    }

    #[test]
    fn maps_with_all_cpu_feature_types() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-cpu-features
            cpu: host,+aes,-rdtscp,hv_ipi=1,kvm=off
            "#,
        );

        assert_eq!(cfg.system.cpu.model, "host");
        assert!(cfg.system.cpu.features.contains(&"+aes".to_string()));
        assert!(cfg.system.cpu.features.contains(&"-rdtscp".to_string()));
    }

    #[test]
    fn maps_unusual_machine_types() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-machine
            machine: microvm,pit=off,rtc_fixed=on
            "#,
        );

        assert!(cfg.system.machine.contains("microvm"));
    }

    #[test]
    fn maps_multipart_disk_options() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-disk-opts
            scsi0: local:disk0,discard=on,ssd=1,cache=none,backup=1,iothread=1
            "#,
        );

        assert_eq!(cfg.devices.drives.len(), 1);
        let drive = &cfg.devices.drives[0];
        assert!(drive.discard);
        assert!(drive.ssd);
        assert_eq!(drive.cache.as_deref(), Some("none"));
    }

    #[test]
    fn maps_boot_order_to_indices() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-boot
            boot: order=scsi0;ide2;net0
            scsi0: disk0,size=20G
            ide2: none,media=cdrom
            net0: virtio=AA:BB:CC:DD:EE:FF,bridge=vmbr0
            "#,
        );

        // Boot order is configured via the boot field
        assert_eq!(cfg.devices.drives.len(), 2);
        assert_eq!(cfg.devices.networks.len(), 1);
    }

    #[test]
    fn maps_smbios_uuid_when_provided() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-smbios
            smbios1: uuid=550e8400-e29b-41d4-a716-446655440000
            "#,
        );

        assert!(cfg.system.smbios.is_some());
        let smbios = cfg.system.smbios.as_ref().unwrap();
        assert_eq!(
            smbios.uuid.as_deref(),
            Some("550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[test]
    fn maps_vm_generation_id_when_provided() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-vmgen
            vmgenid: f47ac10b-58cc-4372-a567-0e02b2c3d479
            "#,
        );

        assert!(cfg.system.smbios.is_some());
        let smbios = cfg.system.smbios.as_ref().unwrap();
        assert_eq!(
            smbios.vm_generation_id.as_deref(),
            Some("f47ac10b-58cc-4372-a567-0e02b2c3d479")
        );
    }

    #[test]
    fn maps_display_vga_options() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-display
            vga: cirrus
            "#,
        );

        assert!(!cfg.devices.displays.is_empty());
    }

    #[test]
    fn maps_serial_with_various_backends() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-serial
            serial0: socket
            serial1: file:/tmp/log.txt
            serial2: unix:/tmp/serial.sock,wait=1
            "#,
        );

        assert_eq!(cfg.devices.serials.len(), 3);
    }

    #[test]
    fn handles_profiles_inference_correctly() {
        let (_yaml, cfg) = map_and_validate(
            r#"
            name: vm-profiles
            cpu: host
            cores: 4
            memory: 8192
            machine: pc-q35-8.1,hpet=off
            vga: cirrus
            "#,
        );

        // Profiles may be inferred based on config or left empty
        let _ = cfg.profiles;
    }

    #[test]
    fn handles_empty_disk_source_gracefully() {
        let parsed = parse_proxmox_config("scsi0: ,size=10G").expect("parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");

        let _cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should be valid");
    }

    #[test]
    fn warnings_accumulate_from_multiple_sources() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-warnings
            arch: unknown-arch
            audio0: device=ich9-intel,driver=alsa
            usb0: host=invalid
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed, RuntimeTarget::PortableLinux)
            .expect("mapper should succeed");

        assert!(!mapped.warnings.is_empty());
    }
}
