use super::QemuArgs;

#[test]
fn test_vfio_pci_with_guest_placement_and_multifunction() {
    let mut args = QemuArgs::new();
    args.add_vfio_pci(
        "0000:03:00.0",
        "hostpci0.0",
        true,
        true,
        Some("ich9-pcie-port-1"),
        Some("0x0.0"),
        true,
        None,
    );

    let built = args.build();
    assert_eq!(built[0], "-device");
    assert_eq!(
        built[1],
        "vfio-pci,host=0000:03:00.0,id=hostpci0.0,x-vga=on,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on"
    );
}

#[test]
fn test_ivshmem_bus_and_mem_path() {
    let mut args = QemuArgs::new();
    args.add_ivshmem(128, 1, "ivshmem0", Some("pcie.0"), "/dev/kvmfr0");

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "ivshmem-plain,memdev=ivshmem0,bus=pcie.0")
    );
    assert!(built.iter().any(
        |arg| arg == "memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M"
    ));
}

#[test]
fn test_xhci_controller_with_placement() {
    let mut args = QemuArgs::new();
    args.add_xhci_controller("xhci", Some(15), Some(15), Some("pci.1"), Some("0x1b"));

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "qemu-xhci,id=xhci,p2=15,p3=15,bus=pci.1,addr=0x1b")
    );
}

#[test]
fn test_usb_host_normalizes_proxmox_form() {
    let mut args = QemuArgs::new();
    args.add_usb_host("1-2.2", None, None, "usb0", Some("xhci.0"), Some("1"));

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "usb-host,hostbus=1,hostport=2.2,id=usb0,bus=xhci.0,port=1")
    );
}

#[test]
fn test_usb_host_explicit_hostbus_hostport() {
    let mut args = QemuArgs::new();
    args.add_usb_host(
        "",
        Some("1"),
        Some("2.2"),
        "usb0",
        Some("xhci.0"),
        Some("1"),
    );

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "usb-host,hostbus=1,hostport=2.2,id=usb0,bus=xhci.0,port=1")
    );
}

#[test]
fn test_guest_agent_with_bus_and_addr() {
    let mut args = QemuArgs::new();
    args.add_guest_agent(
        Some("/var/run/qemu-server/108.qga"),
        false,
        Some("pci.0"),
        Some("0x8"),
    );

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "virtio-serial-pci,id=virtio-serial0,bus=pci.0,addr=0x8")
    );
}

#[test]
fn test_balloon_with_bus_addr_and_id() {
    let mut args = QemuArgs::new();
    args.add_balloon(
        "virtio-balloon-pci",
        true,
        Some("balloon0"),
        Some("pci.0"),
        Some("0x3"),
    );

    let built = args.build();
    assert!(built.iter().any(
        |arg| arg == "virtio-balloon-pci,id=balloon0,bus=pci.0,addr=0x3,free-page-reporting=on"
    ));
}

#[test]
fn test_scsi_controller_with_bus_and_addr() {
    let mut args = QemuArgs::new();
    args.add_scsi_controller("scsihw0", "pvscsi", None, None, Some("pci.0"), Some("0x5"));

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "pvscsi,id=scsihw0,bus=pci.0,addr=0x5")
    );
}

#[test]
fn test_sata_controller_with_bus_and_addr() {
    let mut args = QemuArgs::new();
    args.add_sata_controller("sata0", Some("pci.0"), Some("0x1f"));

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "ahci,id=sata0,bus=pci.0,addr=0x1f")
    );
}

#[test]
fn test_applesmc_device_emission() {
    let mut args = QemuArgs::new();
    args.add_isa_applesmc("dummy-osk");

    let built = args.build();
    assert!(built.iter().any(|arg| arg == "isa-applesmc,osk=dummy-osk"));
}
