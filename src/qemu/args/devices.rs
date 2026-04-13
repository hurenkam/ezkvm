use crate::qemu::types::QemuArgs;

impl QemuArgs {
    /// Add QEMU guest agent
    pub fn add_guest_agent(
        &mut self,
        socket_path: Option<&str>,
        freeze_cpu: bool,
        bus: Option<&str>,
        addr: Option<&str>,
    ) {
        // Add virtio-serial device for guest agent
        self.push_str("-device");
        let mut serial_spec = "virtio-serial-pci,id=virtio-serial0".to_string();
        if let Some(bus) = bus {
            serial_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            serial_spec.push_str(&format!(",addr={}", addr));
        }
        self.push(serial_spec);

        // Add chardev for guest agent
        let chardev_id = "qga0";
        self.push_str("-chardev");
        let chardev_spec = format!(
            "socket,path={},server=on,wait=off,id={}",
            socket_path.unwrap_or("/var/run/qemu-server/qga.sock"),
            chardev_id
        );
        self.push(chardev_spec);

        // Add guest agent channel
        self.push_str("-device");
        let mut channel_spec = format!(
            "virtserialport,chardev={},name=org.qemu.guest_agent.0",
            chardev_id
        );
        if freeze_cpu {
            channel_spec.push_str(",freeze=on");
        }
        self.push(channel_spec);
    }

    /// Add memory ballooning device
    pub fn add_balloon(
        &mut self,
        model: &str,
        free_page_reporting: bool,
        id: Option<&str>,
        bus: Option<&str>,
        addr: Option<&str>,
    ) {
        self.push_str("-device");
        let mut balloon_spec = model.to_string();
        if let Some(id) = id {
            balloon_spec.push_str(&format!(",id={}", id));
        }
        if let Some(bus) = bus {
            balloon_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            balloon_spec.push_str(&format!(",addr={}", addr));
        }
        if free_page_reporting {
            balloon_spec.push_str(",free-page-reporting=on");
        }
        self.push(balloon_spec);
    }

    /// Add VFIO-PCI device passthrough
    #[allow(clippy::too_many_arguments)]
    pub fn add_vfio_pci(
        &mut self,
        device: &str,
        id: &str,
        _pcie: bool,
        x_vga: bool,
        bus: Option<&str>,
        addr: Option<&str>,
        multifunction: bool,
        romfile: Option<&str>,
    ) {
        self.push_str("-device");
        let mut vfio_spec = format!("vfio-pci,host={},id={}", device, id);
        if x_vga {
            vfio_spec.push_str(",x-vga=on");
        }
        if let Some(bus) = bus {
            vfio_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            vfio_spec.push_str(&format!(",addr={}", addr));
        }
        if multifunction {
            vfio_spec.push_str(",multifunction=on");
        }
        if let Some(rom) = romfile {
            vfio_spec.push_str(&format!(",romfile={}", rom));
        }
        self.push(vfio_spec);
    }

    /// Add USB host device passthrough
    pub fn add_usb_host(
        &mut self,
        host_spec: &str,
        hostbus: Option<&str>,
        hostport: Option<&str>,
        id: &str,
        bus: Option<&str>,
        port: Option<&str>,
    ) {
        self.push_str("-device");

        let mut usb_spec = String::from("usb-host");
        usb_spec.push_str(&build_usb_host_location(host_spec, hostbus, hostport));
        usb_spec.push_str(&format!(",id={}", id));
        usb_spec.push_str(&build_usb_guest_placement(bus, port));

        self.push(usb_spec);
    }

    /// Add XHCI USB controller
    pub fn add_xhci_controller(
        &mut self,
        id: &str,
        p2: Option<u8>,
        p3: Option<u8>,
        bus: Option<&str>,
        addr: Option<&str>,
    ) {
        self.push_str("-device");
        let mut controller_spec = format!("qemu-xhci,id={}", id);
        if let Some(p2) = p2 {
            controller_spec.push_str(&format!(",p2={}", p2));
        }
        if let Some(p3) = p3 {
            controller_spec.push_str(&format!(",p3={}", p3));
        }
        if let Some(bus) = bus {
            controller_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            controller_spec.push_str(&format!(",addr={}", addr));
        }
        self.push(controller_spec);
    }

    /// Add Looking Glass shared memory device
    pub fn add_ivshmem(
        &mut self,
        size_mib: u32,
        _vectors: u32,
        id: &str,
        bus: Option<&str>,
        mem_path: &str,
    ) {
        self.push_str("-device");
        let mut device_spec = format!("ivshmem-plain,memdev={}", id);
        if let Some(bus) = bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }
        self.push(device_spec);

        self.push_str("-object");
        self.push(format!(
            "memory-backend-file,id={},share=on,mem-path={},size={}M",
            id, mem_path, size_mib
        ));
    }

    /// Add SCSI controller
    pub fn add_scsi_controller(
        &mut self,
        id: &str,
        controller_type: &str,
        iothread: Option<&str>,
        max_targets: Option<u32>,
        bus: Option<&str>,
        addr: Option<&str>,
    ) {
        self.push_str("-device");
        let mut controller_spec = format!("{},id={}", controller_type, id);

        if let Some(iothread) = iothread {
            controller_spec.push_str(&format!(",iothread={}", iothread));
        }

        if let Some(max_targets) = max_targets {
            controller_spec.push_str(&format!(",max_targets={}", max_targets));
        }

        if let Some(bus) = bus {
            controller_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = addr {
            controller_spec.push_str(&format!(",addr={}", addr));
        }

        self.push(controller_spec);
    }
}

fn normalize_usb_host_spec(host_spec: &str) -> Option<(&str, &str)> {
    let (hostbus, hostport) = host_spec.split_once('-')?;
    if hostbus.is_empty() || hostport.is_empty() {
        return None;
    }
    if !hostbus.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !hostport
        .split('.')
        .all(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some((hostbus, hostport))
}

fn build_usb_host_location(
    host_spec: &str,
    hostbus: Option<&str>,
    hostport: Option<&str>,
) -> String {
    if let (Some(hostbus), Some(hostport)) = (hostbus, hostport) {
        return format!(",hostbus={},hostport={}", hostbus, hostport);
    }

    if let Some((normalized_bus, normalized_port)) = normalize_usb_host_spec(host_spec) {
        return format!(",hostbus={},hostport={}", normalized_bus, normalized_port);
    }

    if !host_spec.trim().is_empty() {
        return format!(",host={}", host_spec);
    }

    String::new()
}

fn build_usb_guest_placement(bus: Option<&str>, port: Option<&str>) -> String {
    let mut placement = String::new();
    if let Some(bus) = bus {
        placement.push_str(&format!(",bus={}", bus));
    }
    if let Some(port) = port {
        placement.push_str(&format!(",port={}", port));
    }
    placement
}
