use crate::config::QmpSocketType;
use crate::qemu::{QemuManager, types::QemuArgs};
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};

// Must stay aligned with the number of ich9-pcie-port-* entries in ezkvm-q35.cfg.
const MAX_RUNTIME_Q35_ROOT_PORTS: u8 = 8;

impl QemuManager {
    pub(crate) fn add_base_args(&self, args: &mut QemuArgs) {
        args.add_name(&self.config.name);
        args.extend(QemuArgs::from(self.config.system.clone()));

        for path in &self.config.system.readconfig {
            args.push_str("-readconfig");
            args.push(path.clone());
        }

        for scsi_controller in self.config.controllers_scsi() {
            args.add_scsi_controller(
                &scsi_controller.id,
                &scsi_controller.r#type,
                scsi_controller.iothread.as_deref(),
                scsi_controller.max_targets,
                scsi_controller.bus.as_deref(),
                scsi_controller.addr.as_deref(),
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

    pub(crate) fn add_devices_and_boot_args(&self, args: &mut QemuArgs) -> Result<()> {
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
        }

        args.extend(QemuArgs::from(devices));
        args.extend(self.build_boot_args());
        self.add_tpm_args(args)?;
        self.add_guest_agent_args(args);
        self.add_balloon_args(args);
        self.add_iommu_args(args);
        self.add_hostpci_args(args)?;
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
            let bus = self.normalize_legacy_root_bus(guest_agent.bus.as_deref());
            args.add_guest_agent(
                guest_agent.socket_path.as_deref(),
                guest_agent.freeze_cpu,
                bus.as_deref(),
                guest_agent.addr.as_deref(),
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

    fn add_hostpci_args(&self, args: &mut QemuArgs) -> Result<()> {
        let has_q35_bridge_readconfig = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));

        let mut used_root_ports: HashSet<u8> = self
            .config
            .host_pci()
            .iter()
            .filter_map(|h| h.bus.as_deref())
            .filter_map(parse_ich9_root_port)
            .collect();
        let mut next_root_port = 1u8;

        let mut slot_by_base_device: HashMap<String, (String, String)> = HashMap::new();
        let mut used_slots: HashMap<(String, String), String> = HashMap::new();

        for hostpci in self.config.host_pci() {
            let mut bus = self
                .normalize_legacy_root_bus(hostpci.bus.as_deref())
                .map(|b| b.into_owned());
            let mut addr = hostpci.addr.clone();
            let parsed_function = parse_pci_device_function(&hostpci.device);

            if has_q35_bridge_readconfig && hostpci.pcie && bus.is_none() {
                if let Some((base, function)) = &parsed_function {
                    if *function > 0
                        && let Some((base_bus, base_addr)) = slot_by_base_device.get(base)
                    {
                        bus = Some(base_bus.clone());
                        if addr.is_none() {
                            addr = address_for_function(base_addr, *function);
                        }
                    }

                    if bus.is_none() {
                        while next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS
                            && used_root_ports.contains(&next_root_port)
                        {
                            next_root_port += 1;
                        }

                        if next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS {
                            let allocated_bus = format!("ich9-pcie-port-{}", next_root_port);
                            used_root_ports.insert(next_root_port);
                            next_root_port += 1;
                            bus = Some(allocated_bus.clone());

                            if addr.is_none() {
                                addr = Some(format!("0x0.{}", function));
                            }

                            if *function == 0
                                && let Some(assigned_addr) = addr.clone()
                            {
                                slot_by_base_device
                                    .insert(base.clone(), (allocated_bus, assigned_addr));
                            }
                        } else {
                            bus = Some("pcie.0".to_string());
                            if addr.is_none() {
                                addr = Some(format!("0x0.{}", function));
                            }
                        }
                    }
                } else {
                    while next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS
                        && used_root_ports.contains(&next_root_port)
                    {
                        next_root_port += 1;
                    }

                    if next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS {
                        bus = Some(format!("ich9-pcie-port-{}", next_root_port));
                        used_root_ports.insert(next_root_port);
                        next_root_port += 1;
                    } else {
                        bus = Some("pcie.0".to_string());
                    }

                    if addr.is_none() {
                        addr = Some("0x0.0".to_string());
                    }
                }
            }

            if let (Some(effective_bus), Some(effective_addr)) = (bus.as_ref(), addr.as_ref()) {
                let key = (effective_bus.clone(), effective_addr.clone());
                if let Some(existing_device) =
                    used_slots.insert(key.clone(), hostpci.device.clone())
                {
                    return Err(anyhow!(
                        "hostpci slot conflict before QEMU start: '{}' and '{}' both use bus='{}', addr='{}'",
                        existing_device,
                        hostpci.device,
                        key.0,
                        key.1
                    ));
                }
            }

            args.add_vfio_pci(
                &hostpci.device,
                &hostpci.id,
                hostpci.pcie,
                hostpci.x_vga,
                bus.as_deref(),
                addr.as_deref(),
                hostpci.multifunction,
                hostpci.romfile.as_deref(),
            );
        }

        Ok(())
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
                args.add_input_device(&input_device.r#type);
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

    pub(crate) fn add_platform_args(&self, args: &mut QemuArgs) -> Result<()> {
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

    pub(crate) fn add_monitoring_and_identity_args(&self, args: &mut QemuArgs) {
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

fn parse_ich9_root_port(bus: &str) -> Option<u8> {
    bus.strip_prefix("ich9-pcie-port-")?.parse::<u8>().ok()
}

fn parse_pci_device_function(device: &str) -> Option<(String, u8)> {
    let (base, function_str) = device.rsplit_once('.')?;
    let function = function_str.parse::<u8>().ok()?;
    Some((base.to_string(), function))
}

fn address_for_function(base_addr: &str, function: u8) -> Option<String> {
    let (slot, _) = base_addr.rsplit_once('.')?;
    Some(format!("{}.{}", slot, function))
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
            RuntimeCliOverrides::default(),
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
            RuntimeCliOverrides::default(),
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
    fn defaults_hostpci_bus_and_addr_for_q35_when_omitted() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-hostpci-defaults"
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

host:
    pci:
        - device: "0000:03:00.0"
          pcie: true
          multifunction: true
        - device: "0000:03:00.1"
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

        assert!(args.iter().any(|arg| {
            arg == "vfio-pci,host=0000:03:00.0,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on"
        }));
        assert!(args.iter().any(|arg| {
            arg == "vfio-pci,host=0000:03:00.1,id=hostpci1,bus=ich9-pcie-port-1,addr=0x0.1"
        }));
    }

    #[test]
    fn fails_fast_on_hostpci_slot_conflicts() {
        let vm = VmConfig::from_str(
            r#"
name: "vm-hostpci-conflict"
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

host:
    pci:
        - device: "0000:03:00.0"
          pcie: true
          bus: "ich9-pcie-port-1"
          addr: "0x0.0"
        - device: "0000:04:00.0"
          pcie: true
          bus: "ich9-pcie-port-1"
          addr: "0x0.0"
"#,
        )
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );

        let err = manager
            .build_command()
            .expect_err("conflicting hostpci slot should fail before qemu start");
        assert!(
            err.to_string()
                .contains("hostpci slot conflict before QEMU start"),
            "unexpected error: {err}"
        );
    }
}
