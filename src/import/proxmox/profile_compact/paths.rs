pub(super) fn is_id_merge_list_path(path: &[String]) -> bool {
    matches!(path, [first, second] if first == "host" && second == "pci")
        || matches!(path, [first, second] if first == "host" && second == "usb")
        || matches!(path, [first, second] if first == "controllers" && second == "scsi")
        || matches!(path, [first, second] if first == "controllers" && second == "sata")
        || matches!(path, [first, second] if first == "controllers" && second == "xhci")
        || matches!(path, [first, second] if first == "devices" && second == "audio")
}

pub(super) fn is_append_unique_list_path(path: &[String]) -> bool {
    matches!(path, [first, second, third] if first == "system" && second == "cpu" && third == "features")
        || matches!(path, [first, second] if first == "system" && second == "machine_options")
        || matches!(path, [first, second] if first == "devices" && second == "input")
        || matches!(path, [first, second] if first == "options" && second == "global_options")
}

pub(super) fn is_append_all_list_path(path: &[String]) -> bool {
    matches!(path, [first, second] if first == "devices" && second == "drives")
        || matches!(path, [first, second] if first == "devices" && second == "networks")
        || matches!(path, [first, second] if first == "policies" && second == "drives")
        || matches!(path, [first, second] if first == "policies" && second == "networks")
        || matches!(path, [first, second] if first == "policies" && second == "displays")
        || matches!(path, [first, second] if first == "policies" && second == "serials")
        || matches!(path, [first, second] if first == "policies" && second == "hostpci")
        || matches!(path, [first, second] if first == "policies" && second == "usb_devices")
        || matches!(path, [first, second] if first == "policies" && second == "xhci_controllers")
        || matches!(path, [first, second] if first == "policies" && second == "audio_devices")
        || matches!(path, [first, second] if first == "policies" && second == "scsi_controllers")
        || matches!(path, [first, second] if first == "policies" && second == "iscsi_disks")
}
