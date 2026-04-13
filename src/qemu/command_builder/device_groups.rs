use crate::qemu::{QemuManager, types::QemuArgs};

impl QemuManager {
    pub(super) fn add_usb_args(&self, args: &mut QemuArgs) {
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
    }

    pub(super) fn add_spice_and_audio_args(&self, args: &mut QemuArgs) {
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
    }
}
