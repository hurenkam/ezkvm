use super::{QemuManager, types::QemuArgs};
use crate::config::QmpSocketType;
use anyhow::{Result, anyhow};

impl QemuManager {
    /// Generate the complete QEMU command line
    pub fn build_command(&self) -> Result<QemuArgs> {
        let mut args = QemuArgs::new();

        // Add VM name first
        args.add_name(&self.config.name);

        // Add system arguments
        args.extend(QemuArgs::from(self.config.system.clone()));

        // Load readconfig files first; they must precede devices that reference those buses.
        for path in &self.config.system.readconfig {
            args.push_str("-readconfig");
            args.push(path.clone());
        }

        // Add SCSI controller arguments before any drives that reference them.
        for scsi_controller in &self.config.scsi_controllers {
            args.add_scsi_controller(
                &scsi_controller.id,
                &scsi_controller.r#type,
                scsi_controller.iothread.as_deref(),
                scsi_controller.max_targets,
                scsi_controller.bus.as_deref(),
                scsi_controller.addr.as_deref(),
            );
        }

        // Add device and boot arguments.
        args.extend(QemuArgs::from(self.config.devices.clone()));
        args.extend(self.build_boot_args());

        // Add TPM arguments
        if let Some(tpm) = &self.config.tpm {
            let socket_path = self.resolve_tpm_socket_path();
            let external_swtpm = self.uses_external_swtpm();
            args.add_tpm(
                &tpm.version,
                &tpm.backend,
                &socket_path,
                &tpm.model,
                external_swtpm,
            )
            .map_err(|e| anyhow!("Failed to configure TPM: {}", e))?;
        }

        if let Some(guest_agent) = &self.config.guest_agent
            && guest_agent.enabled
        {
            args.add_guest_agent(
                guest_agent.socket_path.as_deref(),
                guest_agent.freeze_cpu,
                guest_agent.bus.as_deref(),
                guest_agent.addr.as_deref(),
            );
        }

        if let Some(ballooning) = &self.config.ballooning
            && ballooning.enabled
        {
            args.add_balloon(
                &ballooning.model,
                ballooning.free_page_reporting,
                ballooning.id.as_deref(),
                ballooning.bus.as_deref(),
                ballooning.addr.as_deref(),
            );
        }

        for hostpci in &self.config.hostpci {
            args.add_vfio_pci(
                &hostpci.device,
                &hostpci.id,
                hostpci.pcie,
                hostpci.x_vga,
                hostpci.bus.as_deref(),
                hostpci.addr.as_deref(),
                hostpci.multifunction,
                hostpci.romfile.as_deref(),
            );
        }

        if !self.config.xhci_controllers.is_empty() {
            for xhci_controller in &self.config.xhci_controllers {
                args.add_xhci_controller(
                    &xhci_controller.id,
                    xhci_controller.p2,
                    xhci_controller.p3,
                    xhci_controller.bus.as_deref(),
                    xhci_controller.addr.as_deref(),
                );
            }
        } else if !self.config.usb_devices.is_empty() {
            // Backward-compatible implicit XHCI controller if USB devices are present.
            args.add_xhci_controller("xhci", None, None, None, None);
        }

        for usb_device in &self.config.usb_devices {
            args.add_usb_host(
                &usb_device.host,
                usb_device.hostbus.as_deref(),
                usb_device.hostport.as_deref(),
                &usb_device.id,
                usb_device.bus.as_deref(),
                usb_device.port.as_deref(),
            );
        }

        if let Some(spice) = &self.config.spice
            && spice.enabled
        {
            let has_serial_controller = self
                .config
                .guest_agent
                .as_ref()
                .map(|guest_agent| guest_agent.enabled)
                .unwrap_or(false);
            let attach_display_device = !self.has_primary_passthrough_gpu();
            args.add_spice(
                spice.port,
                &spice.addr,
                spice.disable_ticketing,
                spice.vdagent,
                has_serial_controller,
                attach_display_device,
            );

            if spice.audio {
                let mut emitted_backends: Vec<&str> = Vec::new();

                for audio_device in &self.config.audio_devices {
                    if let Some(audiodev) = audio_device.audiodev.as_deref()
                        && !emitted_backends.contains(&audiodev)
                    {
                        args.add_spice_audiodev(audiodev);
                        emitted_backends.push(audiodev);
                    }
                }

                for audio_device in &self.config.audio_devices {
                    args.add_audio_device(
                        &audio_device.r#type,
                        &audio_device.id,
                        audio_device.bus.as_deref(),
                        audio_device.addr.as_deref(),
                        audio_device.cad,
                        audio_device.audiodev.as_deref(),
                    );
                }
            }
        }

        for input_device in &self.config.input_devices {
            args.add_input_device(&input_device.r#type);
        }

        if let Some(ivshmem) = &self.config.ivshmem
            && ivshmem.enabled
        {
            args.add_ivshmem(
                ivshmem.size,
                ivshmem.vectors,
                &ivshmem.id,
                ivshmem.bus.as_deref(),
                &ivshmem.mem_path,
            );
        }

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

        if let Some(qmp) = &self.config.qmp
            && qmp.enabled
        {
            let socket_type = match qmp.socket_type {
                QmpSocketType::Tcp => "tcp",
                QmpSocketType::Unix => "unix",
            };
            args.add_qmp(qmp.socket_path.as_deref(), socket_type);
        }

        if let Some(smbios) = &self.config.smbios {
            args.add_smbios(
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

        for numa in &self.config.numa {
            args.add_numa_node(numa.id, numa.memory, &numa.cpus, numa.host_node);
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

        args.extend(self.build_option_args()?);

        if self.config.options.enable_kvm {
            args.push_str("-enable-kvm");
        }

        if self.config.options.daemonize {
            args.push_str("-daemonize");
        }

        Ok(args)
    }

    /// Build option-related arguments
    fn build_option_args(&self) -> Result<QemuArgs> {
        let mut args = QemuArgs::new();

        let pid_file = crate::state::get_pid_file_at(
            &self.config.name,
            self.config.options.pid_file.as_deref(),
        )?;
        args.add_pidfile(&pid_file.to_string_lossy());

        if self.config.options.nodefaults {
            args.add_nodefaults();
        }

        if self.has_primary_passthrough_gpu() {
            args.add_vga_none();
            args.add_nographic();
        }

        for global in &self.config.options.global_options {
            args.add_global(global);
        }

        if let Some(rtc) = &self.config.options.rtc {
            args.add_rtc(rtc.base.as_deref(), rtc.driftfix.as_deref());
        }

        Ok(args)
    }
}
