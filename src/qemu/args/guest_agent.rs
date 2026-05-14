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
        let chardev_id = "qga0";
        let controller_id = guest_agent_controller_id(bus, addr);
        self.push_str("-device");
        self.push(build_guest_agent_serial_spec(controller_id, bus, addr));
        self.push_str("-chardev");
        self.push(build_guest_agent_chardev_spec(socket_path, chardev_id));
        self.push_str("-device");
        self.push(build_guest_agent_channel_spec(
            chardev_id,
            controller_id,
            freeze_cpu,
        ));
    }
}

fn guest_agent_controller_id(bus: Option<&str>, addr: Option<&str>) -> &'static str {
    if bus.is_some() || addr.is_some() {
        "qga0"
    } else {
        "virtio-serial0"
    }
}

fn build_guest_agent_serial_spec(
    controller_id: &str,
    bus: Option<&str>,
    addr: Option<&str>,
) -> String {
    // When placement is explicit, use the Proxmox-style legacy controller form.
    // This keeps device topology aligned with imported Proxmox VMs.
    let mut serial_spec = if controller_id == "qga0" {
        format!("virtio-serial,id={}", controller_id)
    } else {
        format!("virtio-serial-pci,id={}", controller_id)
    };
    if let Some(bus) = bus {
        serial_spec.push_str(&format!(",bus={}", bus));
    }
    if let Some(addr) = addr {
        serial_spec.push_str(&format!(",addr={}", addr));
    }
    serial_spec
}

fn build_guest_agent_chardev_spec(socket_path: Option<&str>, chardev_id: &str) -> String {
    format!(
        "socket,path={},server=on,wait=off,id={}",
        socket_path.unwrap_or("/var/run/qemu-server/qga.sock"),
        chardev_id
    )
}

fn build_guest_agent_channel_spec(
    chardev_id: &str,
    _controller_id: &str,
    freeze_cpu: bool,
) -> String {
    let mut channel_spec = format!(
        "virtserialport,chardev={},name=org.qemu.guest_agent.0",
        chardev_id
    );
    if freeze_cpu {
        channel_spec.push_str(",freeze=on");
    }
    channel_spec
}
