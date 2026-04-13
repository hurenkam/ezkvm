pub(super) fn is_valid_pci_address(addr: &str) -> bool {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 3 {
        return false;
    }

    let bus_slot_func: Vec<&str> = parts[2].split('.').collect();
    if bus_slot_func.len() != 2 {
        return false;
    }

    if parts[0].len() != 4 || parts[1].len() != 2 {
        return false;
    }

    if bus_slot_func[0].len() != 2 || bus_slot_func[1].len() != 1 {
        return false;
    }

    u16::from_str_radix(parts[0], 16).is_ok()
        && u8::from_str_radix(parts[1], 16).is_ok()
        && u8::from_str_radix(bus_slot_func[0], 16).is_ok()
        && u8::from_str_radix(bus_slot_func[1], 16).is_ok()
}

pub(super) fn is_valid_usb_spec(spec: &str) -> bool {
    if spec.contains('-') && spec.contains('.') {
        return true;
    }

    if spec.contains(':') {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() == 2 {
            return u16::from_str_radix(parts[0], 16).is_ok()
                && u16::from_str_radix(parts[1], 16).is_ok();
        }
    }

    false
}

pub(super) fn is_valid_usb_hostport(hostport: &str) -> bool {
    !hostport.is_empty()
        && hostport
            .split('.')
            .all(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
}
