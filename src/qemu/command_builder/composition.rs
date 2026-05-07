use crate::config::QmpSocketType;
use crate::qemu::{QemuManager, types::QemuArgs};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

const MAX_Q35_HOSTPCI_ROOT_PORTS: usize = 8;

struct HostPciSlotPlacement {
    bus: Option<String>,
    assign_function_addrs: bool,
}

impl QemuManager {
    pub(super) fn add_base_args(&self, args: &mut QemuArgs) {
        args.add_name(&self.config.name);
        args.extend(QemuArgs::from(self.config.system.clone()));

        let has_q35_bridge_readconfig = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));

        for path in &self.config.system.readconfig {
            args.push_str("-readconfig");
            args.push(path.clone());
        }

        for scsi_controller in self.config.controllers_scsi() {
            let bus = scsi_controller.bus.as_deref().or_else(|| {
                if has_q35_bridge_readconfig && scsi_controller.r#type == "pvscsi" {
                    Some("pci.0")
                } else {
                    None
                }
            });
            let addr = scsi_controller.addr.as_deref().or_else(|| {
                if has_q35_bridge_readconfig && scsi_controller.r#type == "pvscsi" {
                    Some("0x5")
                } else {
                    None
                }
            });
            args.add_scsi_controller(
                &scsi_controller.id,
                &scsi_controller.r#type,
                scsi_controller.iothread.as_deref(),
                scsi_controller.max_targets,
                bus,
                addr,
            );
        }

        for sata_controller in self.config.controllers_sata() {
            args.add_sata_controller(
                &sata_controller.id,
                sata_controller.bus.as_deref(),
                sata_controller.addr.as_deref(),
            );
        }
    }

    pub(super) fn add_devices_and_boot_args(&self, args: &mut QemuArgs) -> Result<()> {
        let mut devices = self.config.devices.clone();
        for warning in crate::state::resolve_networks_for_vm(
            &self.config.name,
            &mut devices,
            &self.central_config,
        ) {
            println!("Warning: {}", warning);
        }

        // Normalize any pci.N bus references on network devices so that portable Q35
        // configs that somehow carry pci.0 (e.g. hand-edited YAMLs) are corrected at
        // emit time, consistent with the normalization applied to guest-agent/balloon.
        for network in &mut devices.networks {
            if let Some(normalized) = self.normalize_legacy_root_bus(network.bus.as_deref()) {
                network.bus = Some(normalized.into_owned());
            }
        }

        let has_q35_bridge_readconfig = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
        if has_q35_bridge_readconfig {
            for display in &mut devices.displays {
                if display.r#type == "virtio-gpu" {
                    if display.bus.is_none() {
                        display.bus = Some("pcie.0".to_string());
                    }
                    if display.addr.is_none() {
                        display.addr = Some("0x1".to_string());
                    }
                }
            }

            for drive in &mut devices.drives {
                if drive.interface == "ide" && drive.bus.is_none() {
                    drive.bus = Some("ide.1".to_string());
                }
            }
        }

        args.extend(QemuArgs::from(devices));
        args.extend(self.build_boot_args());
        self.add_tpm_args(args)?;
        self.add_guest_agent_args(args);
        self.add_balloon_args(args);
        self.add_iommu_args(args);
        self.add_hostpci_args(args);
        self.add_usb_args(args);
        self.add_spice_and_audio_args(args);
        self.add_input_device_args(args);
        self.add_ivshmem_args(args);
        self.add_iscsi_disk_args(args);
        Ok(())
    }

    fn add_tpm_args(&self, args: &mut QemuArgs) -> Result<()> {
        if let Some(tpm) = self.config.system_tpm() {
            let socket_path = self.resolve_tpm_socket_path();
            let external_swtpm = self.uses_external_swtpm();
            let state_file_mode =
                self.tpm_placement_mode() == crate::state::TpmPlacementMode::StateFile;
            args.add_tpm(
                &tpm.version,
                &tpm.backend,
                &socket_path,
                &tpm.model,
                external_swtpm,
                state_file_mode,
            )
            .map_err(|e| anyhow!("Failed to configure TPM: {}", e))?;
        }
        Ok(())
    }

    fn add_guest_agent_args(&self, args: &mut QemuArgs) {
        if let Some(guest_agent) = self.config.options_guest_agent()
            && guest_agent.enabled
        {
            let socket_path = guest_agent
                .socket_path
                .clone()
                .unwrap_or_else(|| self.resolve_guest_agent_socket_path());
            let has_q35_bridge_readconfig = self
                .config
                .system
                .readconfig
                .iter()
                .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
            let fallback_bus = if has_q35_bridge_readconfig {
                Some("pci.0")
            } else {
                None
            };
            let bus = self
                .normalize_legacy_root_bus(guest_agent.bus.as_deref())
                .or_else(|| fallback_bus.map(std::borrow::Cow::Borrowed));
            let addr = if guest_agent.addr.is_some() {
                guest_agent.addr.as_deref()
            } else if has_q35_bridge_readconfig {
                Some("0x8")
            } else {
                None
            };
            args.add_guest_agent(
                Some(&socket_path),
                guest_agent.freeze_cpu,
                bus.as_deref(),
                addr,
            );
        }
    }

    fn add_balloon_args(&self, args: &mut QemuArgs) {
        if let Some(ballooning) = self.config.system_memory_ballooning()
            && ballooning.enabled
        {
            let bus = self.normalize_legacy_root_bus(ballooning.bus.as_deref());
            args.add_balloon(
                &ballooning.model,
                ballooning.free_page_reporting,
                ballooning.id.as_deref(),
                bus.as_deref(),
                ballooning.addr.as_deref(),
            );
        }
    }

    fn add_iommu_args(&self, args: &mut QemuArgs) {
        if let Some(iommu) = &self.config.iommu {
            args.add_iommu(
                &iommu.r#type,
                &iommu.id,
                iommu.intremap,
                iommu.caching_mode,
                iommu.eim,
            );
        }
    }

    fn add_hostpci_args(&self, args: &mut QemuArgs) {
        let has_q35_bridge_readconfig = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
        let slot_placements = if has_q35_bridge_readconfig {
            plan_q35_hostpci_slot_placement(self.config.host_pci())
        } else {
            HashMap::new()
        };

        for hostpci in self.config.host_pci() {
            let slot_placement =
                hostpci_slot_key(&hostpci.device).and_then(|slot| slot_placements.get(slot));
            let bus = self
                .normalize_legacy_root_bus(hostpci.bus.as_deref())
                .or_else(|| {
                    slot_placement.and_then(|placement| {
                        self.normalize_legacy_root_bus(placement.bus.as_deref())
                    })
                });
            let default_addr = slot_placement.and_then(|placement| {
                if placement.assign_function_addrs && placement.bus.is_some() {
                    default_q35_hostpci_function_addr(hostpci)
                } else {
                    None
                }
            });
            let addr = hostpci.addr.as_deref().or(default_addr.as_deref());
            args.add_vfio_pci(
                &hostpci.device,
                &hostpci.id,
                hostpci.pcie,
                hostpci.x_vga,
                bus.as_deref(),
                addr,
                hostpci.multifunction,
                hostpci.romfile.as_deref(),
            );
        }
    }

    fn add_input_device_args(&self, args: &mut QemuArgs) {
        let has_q35_usb = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
        for input_device in self.config.devices_input() {
            if input_device.r#type == "usb-tablet" && has_q35_usb {
                args.add_usb_tablet("ehci.0", 1);
            } else {
                let legacy_q35_input_bus = if has_q35_usb
                    && (input_device.r#type == "virtio-mouse"
                        || input_device.r#type == "virtio-keyboard")
                {
                    Some("pci.0")
                } else {
                    None
                };
                args.add_input_device_with_bus(&input_device.r#type, legacy_q35_input_bus);
            }
        }
    }

    fn add_ivshmem_args(&self, args: &mut QemuArgs) {
        if let Some(ivshmem) = self.config.system_memory_ivshmem()
            && ivshmem.enabled
        {
            let has_q35_bridge_readconfig = self
                .config
                .system
                .readconfig
                .iter()
                .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
            let bus = if ivshmem.bus.is_some() {
                self.normalize_legacy_root_bus(ivshmem.bus.as_deref())
            } else if has_q35_bridge_readconfig {
                Some(std::borrow::Cow::Borrowed("pcie.0"))
            } else {
                None
            };
            let addr = if ivshmem.addr.is_some() {
                ivshmem.addr.as_deref()
            } else if has_q35_bridge_readconfig {
                Some("0x8")
            } else {
                None
            };
            args.add_ivshmem(
                ivshmem.size,
                ivshmem.vectors,
                &ivshmem.id,
                bus.as_deref(),
                addr,
                &ivshmem.mem_path,
            );
        }
    }

    fn add_iscsi_disk_args(&self, args: &mut QemuArgs) {
        for iscsi_disk in &self.config.iscsi_disks {
            args.add_iscsi_disk(
                &iscsi_disk.id,
                &iscsi_disk.portal,
                &iscsi_disk.target,
                iscsi_disk.lun,
                iscsi_disk.initiator.as_deref(),
                iscsi_disk.username.as_deref(),
                iscsi_disk.password.as_deref(),
                iscsi_disk.controller.as_deref(),
            );
        }
    }

    pub(super) fn add_platform_args(&self, args: &mut QemuArgs) -> Result<()> {
        let hugepages = self.config.system.memory.hugepages.as_ref();
        let numa_nodes = &self.config.system.cpu.numa;

        if let Some(hp) = hugepages.filter(|h| h.enabled) {
            let mem_path = hp.effective_mem_path();
            if numa_nodes.is_empty() {
                // Synthesize a single NUMA node covering all memory
                let total_mib = self.config.system.memory.size;
                let vcpus = self.config.system.cpu.vcpus;
                let cpus: Vec<u32> = (0..vcpus).collect();
                args.add_hugepages_memory_backend("ram-node0", total_mib, &mem_path, hp.prealloc);
                args.add_numa_node_with_memdev(0, &cpus, "ram-node0");
            } else {
                for numa in numa_nodes {
                    let id = format!("ram-node{}", numa.id);
                    args.add_hugepages_memory_backend(&id, numa.memory, &mem_path, hp.prealloc);
                    args.add_numa_node_with_memdev(numa.id, &numa.cpus, &id);
                }
            }
        } else {
            for numa in numa_nodes {
                args.add_numa_node(numa.id, numa.memory, &numa.cpus, numa.host_node);
            }
        }

        if let Some(hyperv) = &self.config.hyperv
            && hyperv.enabled
        {
            args.add_hyperv(
                hyperv.relaxed,
                hyperv.vapic,
                hyperv.time,
                hyperv.crash,
                hyperv.reset,
                hyperv.vendor_id.as_deref(),
                hyperv.frequencies,
                hyperv.reenlightenment,
                hyperv.tlbflush,
                hyperv.ipi,
                hyperv.spinlock_retry,
            );
        }

        Ok(())
    }

    pub(super) fn add_monitoring_and_identity_args(&self, args: &mut QemuArgs) {
        if let Some(qmp) = self.config.options_qmp()
            && qmp.enabled
        {
            let socket_type = match qmp.socket_type {
                QmpSocketType::Tcp => "tcp",
                QmpSocketType::Unix => "unix",
            };
            args.add_qmp(qmp.socket_path.as_deref(), socket_type);
        } else {
            // Auto-add a QMP unix socket so the shutdown monitor can detect
            // guest-initiated power-off and send `quit` to QEMU.
            let socket_path = self.auto_qmp_socket_path();
            args.add_qmp(Some(&socket_path), "unix");
        }

        if let Some(smbios) = self.config.system_smbios() {
            args.add_smbios(
                smbios.smbios_type,
                smbios.manufacturer.as_deref(),
                smbios.product.as_deref(),
                smbios.version.as_deref(),
                smbios.serial.as_deref(),
                smbios.uuid.as_deref(),
                smbios.sku.as_deref(),
                smbios.family.as_deref(),
            );

            if let Some(vm_gen_id) = &smbios.vm_generation_id {
                args.add_vm_generation_id(vm_gen_id);
            }
        }

        if let Some(applesmc) = self.config.system_applesmc()
            && applesmc.enabled
        {
            args.add_isa_applesmc(&applesmc.osk);
        }
    }
}

fn plan_q35_hostpci_slot_placement(
    hostpci_devices: &[crate::config::HostPciConfig],
) -> HashMap<String, HostPciSlotPlacement> {
    #[derive(Clone)]
    struct SlotState {
        first_seen: usize,
        min_hostpci_index: usize,
        explicit_bus: Option<String>,
        device_count: usize,
        should_assign_root_port: bool,
        has_multifunction_hint: bool,
    }

    let mut states = HashMap::<String, SlotState>::new();

    for (index, device) in hostpci_devices.iter().enumerate() {
        let Some(slot) = hostpci_slot_key(&device.device) else {
            continue;
        };

        let state = states.entry(slot.to_string()).or_insert_with(|| SlotState {
            first_seen: index,
            min_hostpci_index: hostpci_id_index(&device.id).unwrap_or(index),
            explicit_bus: None,
            device_count: 0,
            should_assign_root_port: false,
            has_multifunction_hint: false,
        });

        state.min_hostpci_index = state
            .min_hostpci_index
            .min(hostpci_id_index(&device.id).unwrap_or(index));
        state.device_count += 1;
        if state.explicit_bus.is_none() {
            state.explicit_bus = device.bus.clone();
        }
        state.should_assign_root_port |= device.pcie || device.multifunction || device.x_vga;
        state.has_multifunction_hint |= device.multifunction || device.x_vga;
    }

    let mut ordered_slots = states
        .iter()
        .filter_map(|(slot, state)| {
            if state.explicit_bus.is_none() && state.should_assign_root_port {
                Some((state.min_hostpci_index, state.first_seen, slot.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ordered_slots.sort();

    let default_buses = ordered_slots
        .into_iter()
        .enumerate()
        .map(|(position, (_, _, slot))| {
            let bus = if position < MAX_Q35_HOSTPCI_ROOT_PORTS {
                format!("ich9-pcie-port-{}", position + 1)
            } else {
                "pcie.0".to_string()
            };
            (slot, bus)
        })
        .collect::<HashMap<_, _>>();

    states
        .into_iter()
        .map(|(slot, state)| {
            let bus = state
                .explicit_bus
                .or_else(|| default_buses.get(&slot).cloned());
            (
                slot,
                HostPciSlotPlacement {
                    bus,
                    assign_function_addrs: state.has_multifunction_hint || state.device_count > 1,
                },
            )
        })
        .collect()
}

fn default_q35_hostpci_function_addr(hostpci: &crate::config::HostPciConfig) -> Option<String> {
    Some(format!("0x0.{}", hostpci_function(&hostpci.device)?))
}

fn hostpci_id_index(id: &str) -> Option<usize> {
    id.strip_prefix("hostpci")?.split('.').next()?.parse().ok()
}

fn hostpci_slot_key(device: &str) -> Option<&str> {
    device.rsplit_once('.').map(|(slot, _)| slot)
}

fn hostpci_function(device: &str) -> Option<u8> {
    let (_, slot_function) = device.rsplit_once(':')?;
    let (_, function) = slot_function.split_once('.')?;
    function.parse().ok()
}

#[cfg(test)]
mod tests {
    use crate::config::{CentralConfig, RuntimeCliOverrides, VmConfig};
    use crate::qemu::QemuManager;

    #[test]
    fn defaults_virtio_gpu_and_ivshmem_placement_with_q35_readconfig() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-display-ivshmem-defaults"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    readconfig:
        - "/usr/share/ezkvm/ezkvm-q35.cfg"
    memory:
        size: 4096
        ivshmem:
            enabled: true
            size: 128
            vectors: 1
            id: "ivshmem0"
            mem_path: "/dev/kvmfr0"
    cpu:
        vcpus: 4
        model: "host"

devices:
    displays:
        - type: "virtio-gpu"
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides {
                run_dir: Some("/tmp/ezkvm-test".to_string()),
                ..Default::default()
            },
        );
        let args = manager
            .build_command()
            .expect("qemu command should build")
            .build();

        assert!(
            args.iter()
                .any(|arg| arg == "virtio-gpu,bus=pcie.0,addr=0x1")
        );
        assert!(
            args.iter()
                .any(|arg| arg == "ivshmem-plain,memdev=ivshmem0,bus=pcie.0,addr=0x8")
        );
    }

    #[test]
    fn keeps_explicit_virtio_gpu_and_ivshmem_placement() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-display-ivshmem-explicit"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    readconfig:
        - "/usr/share/ezkvm/ezkvm-q35.cfg"
    memory:
        size: 4096
        ivshmem:
            enabled: true
            size: 128
            vectors: 1
            id: "ivshmem0"
            bus: "pcie.0"
            addr: "0x9"
            mem_path: "/dev/kvmfr0"
    cpu:
        vcpus: 4
        model: "host"

devices:
    displays:
        - type: "virtio-gpu"
          bus: "pcie.0"
          addr: "0x2"
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides {
                run_dir: Some("/tmp/ezkvm-test".to_string()),
                ..Default::default()
            },
        );
        let args = manager
            .build_command()
            .expect("qemu command should build")
            .build();

        assert!(
            args.iter()
                .any(|arg| arg == "virtio-gpu,bus=pcie.0,addr=0x2")
        );
        assert!(
            args.iter()
                .any(|arg| arg == "ivshmem-plain,memdev=ivshmem0,bus=pcie.0,addr=0x9")
        );
    }

    #[test]
    fn defaults_guest_agent_and_virtio_input_to_legacy_q35_bus() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-q35-legacy-defaults"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    readconfig:
        - "/usr/share/ezkvm/ezkvm-q35.cfg"
    memory:
        size: 4096
    cpu:
        vcpus: 4
        model: "host"

spice:
    enabled: true
    vdagent: true
    port: 5903
    addr: "127.0.0.1"

options:
    guest_agent:
        enabled: true

devices:
    input:
        - type: "virtio-mouse"
        - type: "virtio-keyboard"
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );
        let args = manager
            .build_command()
            .expect("qemu command should build")
            .build();

        assert!(
            args.iter()
                .any(|arg| arg == "virtio-serial,id=qga0,bus=pci.0,addr=0x8")
        );
        assert!(
            args.iter()
                .any(|arg| arg
                    == "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0,bus=qga0.0")
        );
        assert!(
            args.iter()
                .any(|arg| arg == "virtio-serial-pci,id=virtio-serial0,bus=pci.0,addr=0x9")
        );
        assert!(args.iter().any(|arg| arg
            == "virtserialport,chardev=vdagent,name=com.redhat.spice.0,bus=virtio-serial0.0"));
        assert!(args.iter().any(|arg| arg == "virtio-mouse,bus=pci.0"));
        assert!(args.iter().any(|arg| arg == "virtio-keyboard,bus=pci.0"));
        assert!(args.iter().any(|arg| {
            arg.starts_with("socket,path=")
                && arg.contains("vm-q35-legacy-defaults.qga")
                && arg.ends_with(",server=on,wait=off,id=qga0")
        }));
    }

    #[test]
    fn defaults_pvscsi_and_ide_drive_bus_on_q35_bridge_template() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-q35-storage-defaults"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    readconfig:
        - "/usr/share/ezkvm/ezkvm-q35.cfg"
    memory:
        size: 4096
    cpu:
        vcpus: 2
        model: "host"

devices:
    drives:
        - id: "ide2"
          interface: "ide"
          type: "cdrom"
          format: "raw"

controllers:
    scsi:
      - id: "scsihw0"
        type: "pvscsi"
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );
        let args = manager
            .build_command()
            .expect("qemu command should build")
            .build();

        assert!(
            args.iter()
                .any(|arg| arg == "pvscsi,id=scsihw0,bus=pci.0,addr=0x5")
        );
        assert!(
            args.iter()
                .any(|arg| arg == "ide-cd,drive=drive-ide2,id=ide2,bus=ide.1")
        );
    }

    #[test]
    fn defaults_primary_gpu_pair_to_first_root_port_on_q35_bridge_template() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-q35-hostpci-defaults"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    readconfig:
        - "/usr/share/ezkvm/ezkvm-q35.cfg"
    memory:
        size: 4096
    cpu:
        vcpus: 2
        model: "host"

host:
    pci:
      - device: "0000:03:00.0"
        multifunction: true
      - device: "0000:03:00.1"
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );
        let args = manager
            .build_command()
            .expect("qemu command should build")
            .build();

        assert!(args.iter().any(|arg| {
            arg == "vfio-pci,host=0000:03:00.0,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on"
        }));
        assert!(args.iter().any(|arg| {
            arg == "vfio-pci,host=0000:03:00.1,id=hostpci1,bus=ich9-pcie-port-1,addr=0x0.1"
        }));
    }

    #[test]
    fn defaults_additional_pcie_hostpci_groups_to_sequential_root_ports() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-q35-hostpci-sequential-defaults"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    readconfig:
        - "/usr/share/ezkvm/ezkvm-q35.cfg"
    memory:
        size: 4096
    cpu:
        vcpus: 2
        model: "host"

host:
    pci:
      - device: "0000:07:00.0"
        pcie: true
      - device: "0000:41:00.0"
        pcie: true
        multifunction: true
      - device: "0000:41:00.1"
      - device: "0000:0e:10.4"
        pcie: true
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );
        let args = manager
            .build_command()
            .expect("qemu command should build")
            .build();

        assert!(
            args.iter().any(|arg| {
                arg == "vfio-pci,host=0000:07:00.0,id=hostpci0,bus=ich9-pcie-port-1"
            })
        );
        assert!(args.iter().any(|arg| {
            arg == "vfio-pci,host=0000:41:00.0,id=hostpci1,bus=ich9-pcie-port-2,addr=0x0.0,multifunction=on"
        }));
        assert!(args.iter().any(|arg| {
            arg == "vfio-pci,host=0000:41:00.1,id=hostpci2,bus=ich9-pcie-port-2,addr=0x0.1"
        }));
        assert!(
            args.iter().any(|arg| {
                arg == "vfio-pci,host=0000:0e:10.4,id=hostpci3,bus=ich9-pcie-port-3"
            })
        );
    }
}
