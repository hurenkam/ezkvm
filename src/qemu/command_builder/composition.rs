use crate::config::QmpSocketType;
use crate::qemu::{QemuManager, types::QemuArgs};
use anyhow::{Result, anyhow};

impl QemuManager {
    pub(super) fn add_base_args(&self, args: &mut QemuArgs) {
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

    fn add_hostpci_args(&self, args: &mut QemuArgs) {
        for hostpci in self.config.host_pci() {
            let bus = self.normalize_legacy_root_bus(hostpci.bus.as_deref());
            args.add_vfio_pci(
                &hostpci.device,
                &hostpci.id,
                hostpci.pcie,
                hostpci.x_vga,
                bus.as_deref(),
                hostpci.addr.as_deref(),
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
                args.add_input_device(&input_device.r#type);
            }
        }
    }

    fn add_ivshmem_args(&self, args: &mut QemuArgs) {
        if let Some(ivshmem) = self.config.system_memory_ivshmem()
            && ivshmem.enabled
        {
            let bus = self.normalize_legacy_root_bus(ivshmem.bus.as_deref());
            args.add_ivshmem(
                ivshmem.size,
                ivshmem.vectors,
                &ivshmem.id,
                bus.as_deref(),
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
