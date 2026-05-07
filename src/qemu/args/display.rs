use crate::qemu::types::QemuArgs;

#[derive(Debug, Clone, Copy)]
pub struct SpiceVdagentConfig<'a> {
    pub enabled: bool,
    pub has_serial_controller: bool,
    pub serial_bus: Option<&'a str>,
    pub serial_addr: Option<&'a str>,
}

impl QemuArgs {
    /// Add network argument
    pub fn add_network(&mut self, model: &str, mode: &str, mac: Option<&str>) {
        // Generate a unique ID for this network device
        let id = format!("net{}", self.len());

        self.push_str("-netdev");
        self.push(format!("id={},{}", id, mode));

        self.push_str("-device");
        let mut device_spec = format!("{},netdev={}", model, id);
        if let Some(mac_addr) = mac {
            device_spec.push_str(&format!(",mac={}", mac_addr));
        }
        self.push(device_spec);
    }

    /// Add display argument
    pub fn add_display(&mut self, display_type: &str, vram_mib: Option<u32>) {
        self.push_str("-device");
        let mut spec = display_type.to_string();
        if let Some(vram) = vram_mib {
            spec.push_str(&format!(",vram={}", vram * 1024 * 1024)); // Convert MiB to bytes
        }
        self.push(spec);
    }

    /// Add SPICE display server
    pub fn add_spice(
        &mut self,
        port: u16,
        addr: &str,
        disable_ticketing: bool,
        attach_display_device: bool,
        vdagent: SpiceVdagentConfig<'_>,
    ) {
        self.push_str("-spice");
        self.push(build_spice_server_spec(port, addr, disable_ticketing));

        if attach_display_device {
            self.push_str("-device");
            self.push("qxl-vga,id=video0".to_string());
        }

        if vdagent.enabled {
            add_spice_vdagent_args(
                self,
                vdagent.has_serial_controller,
                vdagent.serial_bus,
                vdagent.serial_addr,
            );
        }
    }

    /// Add VNC display server
    pub fn add_vnc(&mut self, display: &str, password: bool) {
        self.push_str("-vnc");
        let mut spec = display.to_string();
        if password {
            spec.push_str(",password=on");
        }
        self.push(spec);
    }

    /// Add a SPICE audiodev backend
    pub fn add_spice_audiodev(&mut self, id: &str) {
        self.push_str("-audiodev");
        self.push(format!("spice,id={}", id));
    }

    /// Add an audio device
    pub fn add_audio_device(
        &mut self,
        device_type: &str,
        id: &str,
        bus: Option<&str>,
        addr: Option<&str>,
        cad: Option<u8>,
        audiodev: Option<&str>,
    ) {
        self.push_str("-device");
        let mut device_spec = format!("{},id={}", device_type, id);

        if let Some(bus) = bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = addr {
            device_spec.push_str(&format!(",addr={}", addr));
        }

        if let Some(cad) = cad {
            device_spec.push_str(&format!(",cad={}", cad));
        }

        if let Some(audiodev) = audiodev {
            device_spec.push_str(&format!(",audiodev={}", audiodev));
        }

        self.push(device_spec);
    }

    /// Add an input device
    pub fn add_input_device(&mut self, device_type: &str) {
        self.push_str("-device");
        self.push(device_type.to_string());
    }

    /// Add an input device with optional explicit PCI bus placement.
    pub fn add_input_device_with_bus(&mut self, device_type: &str, bus: Option<&str>) {
        self.push_str("-device");
        let mut spec = device_type.to_string();
        if let Some(bus) = bus {
            spec.push_str(&format!(",bus={}", bus));
        }
        self.push(spec);
    }

    /// Add a USB tablet device on the given USB bus at the given port.
    /// Use this when an ICH9/EHCI USB controller is present (e.g. via ezkvm-q35.cfg
    /// or pve-q35-4.0.cfg), so the tablet is placed on `ehci.0` as Proxmox does.
    pub fn add_usb_tablet(&mut self, bus: &str, port: u8) {
        self.push_str("-device");
        self.push(format!("usb-tablet,id=tablet,bus={},port={}", bus, port));
    }
}

fn build_spice_server_spec(port: u16, addr: &str, disable_ticketing: bool) -> String {
    let mut spice_spec = format!("port={},addr={}", port, addr);
    if disable_ticketing {
        spice_spec.push_str(",disable-ticketing=on");
    }
    spice_spec
}

fn add_spice_vdagent_args(
    args: &mut QemuArgs,
    has_serial_controller: bool,
    serial_bus: Option<&str>,
    serial_addr: Option<&str>,
) {
    if !has_serial_controller {
        args.push_str("-device");
        let mut serial_spec = "virtio-serial-pci,id=virtio-serial0".to_string();
        if let Some(bus) = serial_bus {
            serial_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = serial_addr {
            serial_spec.push_str(&format!(",addr={}", addr));
        }
        args.push(serial_spec);
    }

    args.push_str("-chardev");
    args.push("spicevmc,id=vdagent,name=vdagent".to_string());
    args.push_str("-device");
    args.push(
        "virtserialport,chardev=vdagent,name=com.redhat.spice.0,bus=virtio-serial0.0".to_string(),
    );
}
