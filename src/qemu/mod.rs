//! QEMU integration module
//!
//! Handles QEMU command generation and process management.

pub mod builder;
pub mod executor;
pub mod args;
pub mod types;
pub mod process;

use crate::config::{VmConfig, CentralConfig};
use crate::qemu::types::QemuArgs;
use anyhow::Result;

/// Main QEMU manager
pub struct QemuManager {
    config: VmConfig,
    central_config: CentralConfig,
}

impl QemuManager {
    fn uses_external_swtpm(&self) -> bool {
        self.config.tpm.as_ref()
            .map(|tpm| tpm.backend == "emulator")
            .unwrap_or(false)
            && self.central_config.tools.swtpm.is_some()
    }

    fn has_primary_passthrough_gpu(&self) -> bool {
        self.config.hostpci.iter().any(|device| device.x_vga)
    }

    fn resolve_tpm_socket_path(&self) -> String {
        if let Some(tpm) = &self.config.tpm {
            if let Some(state_path) = &tpm.state_path {
                return state_path.clone();
            }
        }

        if let Some(run_dir) = &self.central_config.locations.run_dir {
            return format!("{}/tpm", run_dir);
        }

        "/var/run/qemu-server/tpm".to_string()
    }
}

impl QemuManager {
    /// Create a new QEMU manager for a VM configuration
    pub fn new(config: VmConfig, central_config: CentralConfig) -> Self {
        Self { config, central_config }
    }
    
    /// Get a reference to the VM configuration
    pub fn config(&self) -> &VmConfig {
        &self.config
    }
    
    /// Generate the complete QEMU command line
    pub fn build_command(&self) -> Result<QemuArgs> {
        let mut args = QemuArgs::new();
        
        // Add VM name first
        args.add_name(&self.config.name);
        
        // Add system arguments
        args.extend(QemuArgs::from(self.config.system.clone()));
        
        // Add device arguments
        args.extend(QemuArgs::from(self.config.devices.clone()));
        
        // Add boot arguments
        args.extend(self.build_boot_args());
        
        // Add TPM arguments
        if let Some(tpm) = &self.config.tpm {
            let socket_path = self.resolve_tpm_socket_path();
            let external_swtpm = self.uses_external_swtpm();
            args.add_tpm(&tpm.version, &tpm.backend, &socket_path, &tpm.model, external_swtpm);
        }
        
        // Add guest agent arguments
        if let Some(guest_agent) = &self.config.guest_agent {
            if guest_agent.enabled {
                args.add_guest_agent(
                    guest_agent.socket_path.as_deref(),
                    guest_agent.freeze_cpu,
                    guest_agent.bus.as_deref(),
                    guest_agent.addr.as_deref(),
                );
            }
        }
        
        // Add ballooning arguments
        if let Some(ballooning) = &self.config.ballooning {
            if ballooning.enabled {
                args.add_balloon(
                    &ballooning.model,
                    ballooning.free_page_reporting,
                    ballooning.id.as_deref(),
                    ballooning.bus.as_deref(),
                    ballooning.addr.as_deref(),
                );
            }
        }
        
        // Add VFIO-PCI device arguments
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
        
        // Add USB device arguments
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
        
        // Add SPICE arguments
        if let Some(spice) = &self.config.spice {
            if spice.enabled {
                let has_serial_controller = self.config.guest_agent.as_ref()
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
                        if let Some(audiodev) = audio_device.audiodev.as_deref() {
                            if !emitted_backends.contains(&audiodev) {
                                args.add_spice_audiodev(audiodev);
                                emitted_backends.push(audiodev);
                            }
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
        }

        for input_device in &self.config.input_devices {
            args.add_input_device(&input_device.r#type);
        }
        
        // Add ivshmem arguments
        if let Some(ivshmem) = &self.config.ivshmem {
            if ivshmem.enabled {
                args.add_ivshmem(
                    ivshmem.size,
                    ivshmem.vectors,
                    &ivshmem.id,
                    ivshmem.bus.as_deref(),
                    &ivshmem.mem_path,
                );
            }
        }
        
        // Add SCSI controller arguments
        for scsi_controller in &self.config.scsi_controllers {
            args.add_scsi_controller(&scsi_controller.id, &scsi_controller.r#type, 
                                   scsi_controller.iothread.as_deref(), scsi_controller.max_targets,
                                   scsi_controller.bus.as_deref(), scsi_controller.addr.as_deref());
        }
        
        // Add iSCSI disk arguments
        for iscsi_disk in &self.config.iscsi_disks {
            args.add_iscsi_disk(&iscsi_disk.id, &iscsi_disk.portal, &iscsi_disk.target, iscsi_disk.lun,
                              iscsi_disk.initiator.as_deref(), iscsi_disk.username.as_deref(), 
                              iscsi_disk.password.as_deref(), iscsi_disk.controller.as_deref());
        }
        
        // Add QMP monitoring
        if let Some(qmp) = &self.config.qmp {
            if qmp.enabled {
                let socket_type = match qmp.socket_type {
                    super::config::QmpSocketType::Tcp => "tcp",
                    super::config::QmpSocketType::Unix => "unix",
                };
                args.add_qmp(qmp.socket_path.as_deref(), socket_type);
            }
        }
        
        // Add SMBIOS system information
        if let Some(smbios) = &self.config.smbios {
            args.add_smbios(smbios.manufacturer.as_deref(), smbios.product.as_deref(),
                          smbios.version.as_deref(), smbios.serial.as_deref(),
                          smbios.uuid.as_deref(), smbios.sku.as_deref(),
                          smbios.family.as_deref());
            
            // Add VM generation ID if specified
            if let Some(vm_gen_id) = &smbios.vm_generation_id {
                args.add_vm_generation_id(vm_gen_id);
            }
        }
        
        // Add NUMA topology
        for numa in &self.config.numa {
            args.add_numa_node(numa.id, numa.memory, &numa.cpus, numa.host_node);
        }
        
        // Add Hyper-V enlightenments
        if let Some(hyperv) = &self.config.hyperv {
            if hyperv.enabled {
                args.add_hyperv(hyperv.relaxed, hyperv.vapic, hyperv.time, hyperv.crash,
                              hyperv.reset, hyperv.vendor_id.as_deref(), hyperv.frequencies,
                              hyperv.reenlightenment, hyperv.tlbflush, hyperv.ipi,
                              hyperv.spinlock_retry);
            }
        }
        
        // Add option arguments
        args.extend(self.build_option_args()?);
        
        // Add KVM acceleration if enabled
        if self.config.options.enable_kvm {
            args.push_str("-enable-kvm");
        }
        
        // Add daemon mode if requested
        if self.config.options.daemonize {
            args.push_str("-daemonize");
        }
        
        Ok(args)
    }
    
    /// Build boot-related arguments
    fn build_boot_args(&self) -> QemuArgs {
        let mut args = QemuArgs::new();
        
        // Boot order and related boot UI options
        if !self.config.boot.boot_order.is_empty()
            || self.config.boot.menu
            || self.config.boot.strict
            || self.config.boot.reboot_timeout.is_some()
            || self.config.boot.splash.is_some()
        {
            args.push_str("-boot");
            let mut parts = Vec::new();

            if !self.config.boot.boot_order.is_empty() {
                let order: Vec<String> = self.config.boot.boot_order.iter()
                    .map(|device| match device.as_str() {
                        "disk" | "hd" => "c".to_string(),
                        "cdrom" | "cd" => "d".to_string(),
                        "floppy" => "a".to_string(),
                        "network" => "n".to_string(),
                        _ => device.clone(),
                    })
                    .collect();
                parts.push(format!("order={}", order.join("")));
            }

            if self.config.boot.menu {
                parts.push("menu=on".to_string());
            }

            if self.config.boot.strict {
                parts.push("strict=on".to_string());
            }

            if let Some(timeout) = self.config.boot.reboot_timeout {
                parts.push(format!("reboot-timeout={}", timeout));
            }

            if let Some(splash) = &self.config.boot.splash {
                parts.push(format!("splash={}", splash));
            }

            args.push(parts.join(","));
        }
        
        // Kernel boot (if specified)
        if let Some(kernel) = &self.config.boot.kernel {
            args.push_str("-kernel");
            args.push(kernel.clone());
        }
        
        if let Some(initrd) = &self.config.boot.initrd {
            args.push_str("-initrd");
            args.push(initrd.clone());
        }
        
        if let Some(cmdline) = &self.config.boot.cmdline {
            args.push_str("-append");
            args.push(cmdline.clone());
        }
        
        // UEFI firmware (enhanced support)
        if let Some(firmware) = &self.config.boot.firmware {
            if firmware == "uefi" || firmware == "ovmf" {
                let code_path = if let Some(code) = &self.config.boot.uefi_code {
                    Some(code.clone())
                } else if let Some(ovmf_dir) = &self.central_config.locations.ovmf_dir {
                    Some(format!("{}/OVMF.fd", ovmf_dir))
                } else {
                    None
                };
                args.add_uefi(
                    code_path.as_deref(),
                    self.config.boot.uefi_vars.as_deref(),
                    self.config.boot.secure_boot
                );
            }
        }
        
        args
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
    
    /// Get the QEMU binary name
    pub fn binary_name(&self) -> String {
        self.config.system.qemu_binary()
    }
}