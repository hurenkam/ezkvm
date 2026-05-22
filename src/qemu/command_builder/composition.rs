use crate::config::QmpSocketType;
use crate::qemu::{QemuManager, types::QemuArgs};
use anyhow::{Result, anyhow};
use std::collections::{BTreeSet, HashMap, HashSet};

// Maximum number of runtime-synthesized ich9-pcie-port-* buses.
const MAX_RUNTIME_Q35_ROOT_PORTS: u8 = 8;

impl QemuManager {
    pub(crate) fn add_base_args(&self, args: &mut QemuArgs) {
        args.add_name(&self.config.name);
        args.extend(QemuArgs::from(self.config.system.clone()));

        let synthesize_q35_topology = self.should_synthesize_portable_q35_topology();

        for path in &self.config.system.readconfig {
            if synthesize_q35_topology && path.contains("ezkvm-q35.cfg") {
                continue;
            }
            args.push_str("-readconfig");
            args.push(path.clone());
        }

        if synthesize_q35_topology {
            self.add_dynamic_q35_topology(args);
        }

        // Determine if this is a Q35 machine with the q35 bridge template.
        let has_q35_bridge_readconfig = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));

        let mut scsi_addr_counter = 0x5u8;
        for scsi_controller in self.config.controllers_scsi() {
            let mut bus = scsi_controller.bus.clone();
            let mut addr = scsi_controller.addr.clone();

            // Apply Q35 defaults per ADR-0005: pvscsi at pci.0, addr=0x5+
            if has_q35_bridge_readconfig
                && bus.is_none()
                && addr.is_none()
                && scsi_controller.r#type == "pvscsi"
            {
                bus = Some("pci.0".to_string());
                addr = Some(format!("0x{:x}", scsi_addr_counter));
                scsi_addr_counter += 1;
            }

            args.add_scsi_controller(
                &scsi_controller.id,
                &scsi_controller.r#type,
                scsi_controller.iothread.as_deref(),
                scsi_controller.max_targets,
                bus.as_deref(),
                addr.as_deref(),
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

    fn should_synthesize_portable_q35_topology(&self) -> bool {
        let mode = crate::state::detect_runtime_capability_mode(&self.config);
        if mode != crate::state::RuntimeCapabilityMode::PortableLinux {
            return false;
        }

        if !self.config.system.machine.to_lowercase().contains("q35") {
            return false;
        }

        self.config
            .system
            .readconfig
            .iter()
            .any(|path| path.contains("ezkvm-q35.cfg"))
    }

    fn add_dynamic_q35_topology(&self, args: &mut QemuArgs) {
        for port in self.dynamic_q35_required_root_ports() {
            let slot = port.saturating_sub(1);
            let spec = format!(
                "pcie-root-port,id=ich9-pcie-port-{port},x-speed=16,x-width=32,multifunction=on,bus=pcie.0,addr=1c.{slot:x},port={port},chassis={port}"
            );
            args.push_str("-device");
            args.push(spec);
        }

        let active_ehci = self.dynamic_q35_active_ehci_complexes();
        if active_ehci.contains("ehci.0") {
            args.push_str("-device");
            args.push("ich9-usb-ehci1,id=ehci,multifunction=on,bus=pcie.0,addr=1d.7".to_string());
            args.push_str("-device");
            args.push(
                "ich9-usb-uhci1,id=uhci1,multifunction=on,bus=pcie.0,addr=1d.0,masterbus=ehci.0,firstport=0"
                    .to_string(),
            );
            args.push_str("-device");
            args.push(
                "ich9-usb-uhci2,id=uhci2,multifunction=on,bus=pcie.0,addr=1d.1,masterbus=ehci.0,firstport=2"
                    .to_string(),
            );
            args.push_str("-device");
            args.push(
                "ich9-usb-uhci3,id=uhci3,multifunction=on,bus=pcie.0,addr=1d.2,masterbus=ehci.0,firstport=4"
                    .to_string(),
            );
        }
        if active_ehci.contains("ehci-2.0") {
            args.push_str("-device");
            args.push("ich9-usb-ehci2,id=ehci-2,multifunction=on,bus=pcie.0,addr=1a.7".to_string());
            args.push_str("-device");
            args.push(
                "ich9-usb-uhci4,id=uhci-4,multifunction=on,bus=pcie.0,addr=1a.0,masterbus=ehci-2.0,firstport=0"
                    .to_string(),
            );
            args.push_str("-device");
            args.push(
                "ich9-usb-uhci5,id=uhci-5,multifunction=on,bus=pcie.0,addr=1a.1,masterbus=ehci-2.0,firstport=2"
                    .to_string(),
            );
            args.push_str("-device");
            args.push(
                "ich9-usb-uhci6,id=uhci-6,multifunction=on,bus=pcie.0,addr=1a.2,masterbus=ehci-2.0,firstport=4"
                    .to_string(),
            );
        }

        let legacy_buses = self.dynamic_q35_required_legacy_pci_buses();
        if !legacy_buses.is_empty() {
            args.push_str("-device");
            args.push("i82801b11-bridge,id=pcidmi,bus=pcie.0,addr=1e.0".to_string());

            for bus in legacy_buses {
                let bridge_addr = bus + 1;
                args.push_str("-device");
                args.push(format!(
                    "pci-bridge,id=pci.{bus},bus=pcidmi,addr={bridge_addr}.0,chassis_nr={bridge_addr}"
                ));
            }
        }
    }

    fn dynamic_q35_required_root_ports(&self) -> BTreeSet<u8> {
        let mut required_ports = BTreeSet::new();

        let mut function_count_by_base: HashMap<String, usize> = HashMap::new();
        let mut multifunction_hint_by_base: HashMap<String, bool> = HashMap::new();
        let mut root_port_required_by_base: HashMap<String, bool> = HashMap::new();
        for hostpci in self.config.host_pci() {
            if let Some((base, _)) = parse_pci_device_function(&hostpci.device) {
                *function_count_by_base.entry(base.clone()).or_insert(0) += 1;
                if hostpci.multifunction || hostpci.x_vga {
                    multifunction_hint_by_base.insert(base.clone(), true);
                }
                if hostpci.pcie || hostpci.multifunction || hostpci.x_vga {
                    root_port_required_by_base.insert(base, true);
                }
            }
        }

        let mut used_root_ports: HashSet<u8> = self
            .config
            .host_pci()
            .iter()
            .filter_map(|h| h.bus.as_deref())
            .filter_map(parse_ich9_root_port)
            .collect();
        required_ports.extend(used_root_ports.iter().copied());
        let mut next_root_port = 1u8;

        let mut port_by_base_device: HashMap<String, u8> = HashMap::new();
        for hostpci in self.config.host_pci() {
            if let Some(port) = hostpci.bus.as_deref().and_then(parse_ich9_root_port) {
                if let Some((base, function)) = parse_pci_device_function(&hostpci.device)
                    && function == 0
                {
                    port_by_base_device.insert(base, port);
                }
                continue;
            }

            let Some((base, function)) = parse_pci_device_function(&hostpci.device) else {
                if hostpci.pcie {
                    while next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS
                        && used_root_ports.contains(&next_root_port)
                    {
                        next_root_port += 1;
                    }
                    if next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS {
                        used_root_ports.insert(next_root_port);
                        required_ports.insert(next_root_port);
                        next_root_port += 1;
                    }
                }
                continue;
            };

            let should_assign_root_port = root_port_required_by_base
                .get(&base)
                .copied()
                .unwrap_or(hostpci.pcie);
            if !should_assign_root_port {
                continue;
            }

            let _assign_function_addrs = function_count_by_base.get(&base).copied().unwrap_or(0)
                > 1
                || multifunction_hint_by_base
                    .get(&base)
                    .copied()
                    .unwrap_or(false);

            if function > 0
                && let Some(port) = port_by_base_device.get(&base)
            {
                required_ports.insert(*port);
                continue;
            }

            while next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS
                && used_root_ports.contains(&next_root_port)
            {
                next_root_port += 1;
            }

            if next_root_port <= MAX_RUNTIME_Q35_ROOT_PORTS {
                used_root_ports.insert(next_root_port);
                required_ports.insert(next_root_port);
                if function == 0 {
                    port_by_base_device.insert(base, next_root_port);
                }
                next_root_port += 1;
            }
        }

        required_ports
    }

    fn dynamic_q35_required_legacy_pci_buses(&self) -> BTreeSet<u8> {
        let mut buses = BTreeSet::new();

        for network in &self.config.devices.networks {
            collect_legacy_pci_bus_number(network.bus.as_deref(), &mut buses);
        }

        let mut scsi_index = 0u8;
        for controller in self.config.controllers_scsi() {
            if let Some(bus) = controller.bus.as_deref() {
                collect_legacy_pci_bus_number(Some(bus), &mut buses);
                continue;
            }

            if controller.r#type == "pvscsi" && controller.addr.is_none() {
                let _ = scsi_index;
                buses.insert(0);
                scsi_index = scsi_index.saturating_add(1);
            }
        }

        for controller in self.config.controllers_xhci() {
            if let Some(bus) = controller.bus.as_deref() {
                collect_legacy_pci_bus_number(Some(bus), &mut buses);
            } else {
                buses.insert(1);
            }
        }
        if self.config.controllers_xhci().is_empty() && !self.config.host_usb().is_empty() {
            buses.insert(1);
        }

        for hostpci in self.config.host_pci() {
            collect_legacy_pci_bus_number(hostpci.bus.as_deref(), &mut buses);
        }

        for audio in self.config.devices_audio() {
            collect_legacy_pci_bus_number(audio.bus.as_deref(), &mut buses);
        }

        if self
            .config
            .devices_input()
            .iter()
            .any(|input| input.r#type != "usb-tablet")
        {
            buses.insert(0);
        }

        if let Some(ballooning) = self.config.system_memory_ballooning() {
            collect_legacy_pci_bus_number(ballooning.bus.as_deref(), &mut buses);
        }

        if let Some(guest_agent) = self.config.options_guest_agent()
            && guest_agent.enabled
        {
            if guest_agent.bus.is_none() && guest_agent.addr.is_none() {
                buses.insert(0);
            }
            collect_legacy_pci_bus_number(guest_agent.bus.as_deref(), &mut buses);
        }

        if let Some(spice) = &self.config.spice
            && spice.enabled
            && spice.vdagent
        {
            buses.insert(0);
        }

        if let Some(ivshmem) = self.config.system_memory_ivshmem() {
            collect_legacy_pci_bus_number(ivshmem.bus.as_deref(), &mut buses);
        }

        buses
    }

    fn dynamic_q35_active_ehci_complexes(&self) -> BTreeSet<&'static str> {
        let mut active = BTreeSet::new();

        for usb in self.config.host_usb() {
            if let Some(bus) = usb.bus.as_deref() {
                if bus.starts_with("ehci.0") {
                    active.insert("ehci.0");
                }
                if bus.starts_with("ehci-2.0") {
                    active.insert("ehci-2.0");
                }
            }
        }

        if self
            .config
            .devices_input()
            .iter()
            .any(|input| input.r#type == "usb-tablet")
        {
            active.insert("ehci.0");
        }

        active
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
        for drive in &mut devices.drives {
            if drive.interface != "ide" {
                continue;
            }

            if let Some((fallback_bus, fallback_unit)) =
                infer_ide_attachment_from_drive_id(&drive.id)
            {
                if drive.bus.is_none() {
                    drive.bus = Some(fallback_bus);
                }
                if drive.unit.is_none() {
                    drive.unit = Some(fallback_unit);
                }
                continue;
            }

            if has_q35_bridge_readconfig && drive.bus.is_none() {
                drive.bus = Some("ide.1".to_string());
            }
            if drive.unit.is_none() {
                drive.unit = Some(0);
            }
        }

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
            let socket_path = guest_agent
                .socket_path
                .as_deref()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| self.resolve_guest_agent_socket_path());
            let mut bus = self.normalize_legacy_root_bus(guest_agent.bus.as_deref());
            let mut addr = guest_agent.addr.clone();

            // Default to Proxmox standard placement when not explicitly specified.
            // This maintains compatibility with imported Proxmox VMs.
            if bus.is_none() && addr.is_none() {
                bus = Some(std::borrow::Cow::Borrowed("pci.0"));
                addr = Some("0x8".to_string());
            }

            args.add_guest_agent(
                Some(&socket_path),
                guest_agent.freeze_cpu,
                bus.as_deref(),
                addr.as_deref(),
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

        let mut function_count_by_base: HashMap<String, usize> = HashMap::new();
        let mut multifunction_hint_by_base: HashMap<String, bool> = HashMap::new();
        let mut root_port_required_by_base: HashMap<String, bool> = HashMap::new();
        for hostpci in self.config.host_pci() {
            if let Some((base, _)) = parse_pci_device_function(&hostpci.device) {
                *function_count_by_base.entry(base.clone()).or_insert(0) += 1;
                if hostpci.multifunction || hostpci.x_vga {
                    multifunction_hint_by_base.insert(base.clone(), true);
                }
                if hostpci.pcie || hostpci.multifunction || hostpci.x_vga {
                    root_port_required_by_base.insert(base, true);
                }
            }
        }

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

            if has_q35_bridge_readconfig && bus.is_none() {
                if let Some((base, function)) = &parsed_function {
                    let should_assign_root_port = root_port_required_by_base
                        .get(base)
                        .copied()
                        .unwrap_or(hostpci.pcie);
                    if should_assign_root_port {
                        let assign_function_addrs =
                            function_count_by_base.get(base).copied().unwrap_or(0) > 1
                                || multifunction_hint_by_base
                                    .get(base)
                                    .copied()
                                    .unwrap_or(false);

                        if *function > 0
                            && let Some((base_bus, base_addr)) = slot_by_base_device.get(base)
                        {
                            bus = Some(base_bus.clone());
                            if assign_function_addrs && addr.is_none() {
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

                                if assign_function_addrs && addr.is_none() {
                                    addr = Some(format!("0x0.{}", function));
                                }

                                if assign_function_addrs
                                    && *function == 0
                                    && let Some(assigned_addr) = addr.clone()
                                {
                                    slot_by_base_device
                                        .insert(base.clone(), (allocated_bus, assigned_addr));
                                }
                            } else {
                                bus = Some("pcie.0".to_string());
                                if assign_function_addrs && addr.is_none() {
                                    addr = Some(format!("0x0.{}", function));
                                }
                            }
                        }
                    }
                } else if hostpci.pcie {
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
        let has_q35_bridge_readconfig = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
        for input_device in self.config.devices_input() {
            if input_device.r#type == "usb-tablet" && has_q35_usb {
                args.add_usb_tablet("ehci.0", 1);
            } else if has_q35_bridge_readconfig {
                // Apply Q35 defaults for virtio input devices: pci.0
                args.add_input_device_with_bus(&input_device.r#type, Some("pci.0"));
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

fn collect_legacy_pci_bus_number(bus: Option<&str>, out: &mut BTreeSet<u8>) {
    let Some(bus) = bus else {
        return;
    };
    let Some(index) = bus.strip_prefix("pci.") else {
        return;
    };
    let Ok(parsed) = index.parse::<u8>() else {
        return;
    };
    out.insert(parsed);
}

fn infer_ide_attachment_from_drive_id(id: &str) -> Option<(String, u32)> {
    let slot = id.strip_prefix("ide")?.parse::<u32>().ok()?;
    let controller = slot / 2;
    let unit = slot % 2;
    Some((format!("ide.{controller}"), unit))
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
