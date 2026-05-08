use crate::qemu::{QemuManager, types::QemuArgs};
use anyhow::{Result, anyhow};

impl QemuManager {
    pub(crate) fn add_base_args(&self, args: &mut QemuArgs) {
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

    pub(crate) fn add_devices_and_boot_args(&self, args: &mut QemuArgs) -> Result<()> {
        let mut devices = self.config.devices.clone();
        for warning in crate::state::resolve_networks_for_vm(
            &self.config.name,
            &mut devices,
            &self.central_config,
        ) {
            tracing::warn!(target: "ezkvm::qemu::network", warning = %warning, "network resolution warning");
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

    fn add_hostpci_args(&self, args: &mut QemuArgs) {
        super::composition_hostpci::add_hostpci_args(self, args);
    }

    pub(crate) fn add_platform_args(&self, args: &mut QemuArgs) -> Result<()> {
        super::composition_platform::add_platform_args(self, args)
    }

    pub(crate) fn add_monitoring_and_identity_args(&self, args: &mut QemuArgs) {
        super::composition_platform::add_monitoring_and_identity_args(self, args);
    }
}

fn infer_ide_attachment_from_drive_id(id: &str) -> Option<(String, u32)> {
    let slot = id.strip_prefix("ide")?.parse::<u32>().ok()?;
    let controller = slot / 2;
    let unit = slot % 2;
    Some((format!("ide.{controller}"), unit))
}
