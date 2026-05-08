use crate::qemu::{QemuManager, types::QemuArgs};

impl QemuManager {
    pub(super) fn add_usb_args(&self, args: &mut QemuArgs) {
        let has_q35_usb_topology = self
            .config
            .system
            .readconfig
            .iter()
            .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
        let fallback_bus = if has_q35_usb_topology {
            Some("pci.1")
        } else {
            None
        };
        let fallback_addr = if has_q35_usb_topology {
            Some("0x1b")
        } else {
            None
        };

        if !self.config.controllers_xhci().is_empty() {
            for xhci_controller in self.config.controllers_xhci() {
                args.add_xhci_controller(
                    &xhci_controller.id,
                    xhci_controller.p2,
                    xhci_controller.p3,
                    xhci_controller.bus.as_deref().or(fallback_bus),
                    xhci_controller.addr.as_deref().or(fallback_addr),
                );
            }
        } else if !self.config.host_usb().is_empty() {
            // When auto-synthesizing an XHCI controller, keep ADR-0005 preferred
            // placement for Q35-compatible topologies.
            args.add_xhci_controller("xhci", None, None, fallback_bus, fallback_addr);
        }

        for usb_device in self.config.host_usb() {
            args.add_usb_host(
                &usb_device.host,
                usb_device.hostbus.as_deref(),
                usb_device.hostport.as_deref(),
                &usb_device.id,
                usb_device.bus.as_deref(),
                usb_device.port.as_deref(),
            );
        }
    }

    pub(super) fn add_spice_and_audio_args(&self, args: &mut QemuArgs) {
        if let Some(vnc) = &self.config.vnc
            && vnc.enabled
        {
            args.add_vnc(&vnc.display, vnc.password);
        }

        if let Some(spice) = &self.config.spice
            && spice.enabled
        {
            let has_q35_bridge_readconfig = self
                .config
                .system
                .readconfig
                .iter()
                .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
            let has_serial_controller = self
                .config
                .options_guest_agent()
                .map(|guest_agent| {
                    // Proxmox-style pinned guest-agent controllers should not be reused
                    // for SPICE vdagent. Keep a dedicated vdagent serial controller.
                    guest_agent.enabled
                        && guest_agent.bus.is_none()
                        && guest_agent.addr.is_none()
                        && !has_q35_bridge_readconfig
                })
                .unwrap_or(false);
            let attach_display_device = !self.has_primary_passthrough_gpu();
            let vdagent_serial_bus = if has_q35_bridge_readconfig && !has_serial_controller {
                Some("pci.0")
            } else {
                None
            };
            let vdagent_serial_addr = if has_q35_bridge_readconfig && !has_serial_controller {
                Some("0x9")
            } else {
                None
            };
            args.add_spice(
                spice.port,
                &spice.addr,
                spice.disable_ticketing,
                attach_display_device,
                crate::qemu::args::SpiceVdagentConfig {
                    enabled: spice.vdagent,
                    has_serial_controller,
                    serial_bus: vdagent_serial_bus,
                    serial_addr: vdagent_serial_addr,
                },
            );

            if spice.audio {
                let mut emitted_backends: Vec<&str> = Vec::new();

                for audio_device in self.config.devices_audio() {
                    if let Some(audiodev) = audio_device.audiodev.as_deref()
                        && !emitted_backends.contains(&audiodev)
                    {
                        args.add_spice_audiodev(audiodev);
                        emitted_backends.push(audiodev);
                    }
                }

                for audio_device in self.config.devices_audio() {
                    let bus = self.normalize_legacy_root_bus(audio_device.bus.as_deref());
                    args.add_audio_device(
                        &audio_device.r#type,
                        &audio_device.id,
                        bus.as_deref(),
                        audio_device.addr.as_deref(),
                        audio_device.cad,
                        audio_device.audiodev.as_deref(),
                    );
                }
            }
        }
    }
}
