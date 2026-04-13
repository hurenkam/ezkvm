use crate::qemu::types::QemuArgs;

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
        vdagent: bool,
        has_serial_controller: bool,
        attach_display_device: bool,
    ) {
        self.push_str("-spice");
        self.push(build_spice_server_spec(port, addr, disable_ticketing));

        if attach_display_device {
            self.push_str("-device");
            self.push("qxl-vga,id=video0".to_string());
        }

        if vdagent {
            add_spice_vdagent_args(self, has_serial_controller);
        }
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
}

fn build_spice_server_spec(port: u16, addr: &str, disable_ticketing: bool) -> String {
    let mut spice_spec = format!("port={},addr={}", port, addr);
    if disable_ticketing {
        spice_spec.push_str(",disable-ticketing=on");
    }
    spice_spec
}

fn add_spice_vdagent_args(args: &mut QemuArgs, has_serial_controller: bool) {
    if !has_serial_controller {
        args.push_str("-device");
        args.push("virtio-serial-pci,id=virtio-serial0".to_string());
    }

    args.push_str("-chardev");
    args.push("spicevmc,id=vdagent,name=vdagent".to_string());
    args.push_str("-device");
    args.push("virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string());
}
