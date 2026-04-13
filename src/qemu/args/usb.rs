use crate::qemu::types::QemuArgs;

impl QemuArgs {
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
