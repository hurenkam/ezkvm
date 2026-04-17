use super::QemuArgs;

#[test]
fn test_spice_audio_devices() {
    let mut args = QemuArgs::new();
    args.add_spice(5903, "0.0.0.0", true, true, false, true);
    args.add_spice_audiodev("spice-backend0");
    args.add_audio_device(
        "ich9-intel-hda",
        "audiodev0",
        Some("pci.2"),
        Some("0xc"),
        None,
        None,
    );
    args.add_audio_device(
        "hda-micro",
        "audiodev0-codec0",
        Some("audiodev0.0"),
        None,
        Some(0),
        Some("spice-backend0"),
    );
    args.add_audio_device(
        "hda-duplex",
        "audiodev0-codec1",
        Some("audiodev0.0"),
        None,
        Some(1),
        Some("spice-backend0"),
    );

    let built = args.build();
    assert!(built.iter().any(|arg| arg == "spice,id=spice-backend0"));
    assert!(
        built
            .iter()
            .any(|arg| arg == "ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc")
    );
    assert!(built.iter().any(|arg| {
        arg == "hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0"
    }));
    assert!(built.iter().any(|arg| {
        arg == "hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0"
    }));
    assert!(!built.iter().any(|arg| arg.contains("spice-audio")));
}

#[test]
fn test_input_devices() {
    let mut args = QemuArgs::new();
    args.add_spice(5903, "0.0.0.0", true, true, false, true);
    args.add_input_device("virtio-mouse");
    args.add_input_device("virtio-keyboard");

    let built = args.build();
    let mouse_count = built.iter().filter(|arg| *arg == "virtio-mouse").count();
    let keyboard_count = built.iter().filter(|arg| *arg == "virtio-keyboard").count();

    assert_eq!(mouse_count, 1);
    assert_eq!(keyboard_count, 1);
    assert!(
        built
            .iter()
            .any(|arg| arg == "virtio-serial-pci,id=virtio-serial0")
    );
    assert!(
        built
            .iter()
            .any(|arg| arg == "virtserialport,chardev=vdagent,name=com.redhat.spice.0")
    );
}

#[test]
fn test_spice_vdagent_reuses_existing_serial_controller() {
    let mut args = QemuArgs::new();
    args.add_guest_agent(
        Some("/var/run/qemu-server/108.qga"),
        false,
        Some("pci.0"),
        Some("0x8"),
    );
    args.add_spice(5903, "0.0.0.0", true, true, true, true);

    let built = args.build();
    let serial_controller_count = built
        .iter()
        .filter(|arg| arg.starts_with("virtio-serial-pci"))
        .count();

    assert_eq!(serial_controller_count, 1);
    assert!(
        built
            .iter()
            .any(|arg| arg == "virtserialport,chardev=vdagent,name=com.redhat.spice.0")
    );
    assert!(
        built
            .iter()
            .any(|arg| arg == "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0")
    );
}

#[test]
fn test_spice_without_display_device_for_passthrough_vm() {
    let mut args = QemuArgs::new();
    args.add_spice(5903, "0.0.0.0", true, true, false, false);

    let built = args.build();
    assert!(
        built
            .iter()
            .any(|arg| arg == "port=5903,addr=0.0.0.0,disable-ticketing=on")
    );
    assert!(!built.iter().any(|arg| arg == "qxl-vga,id=video0"));
    assert!(
        built
            .iter()
            .any(|arg| arg == "spicevmc,id=vdagent,name=vdagent")
    );
}

#[test]
fn test_vnc_endpoint_with_password() {
    let mut args = QemuArgs::new();
    args.add_vnc("unix:/var/run/qemu-server/405.vnc", true);

    let built = args.build();
    assert_eq!(
        built,
        vec!["-vnc", "unix:/var/run/qemu-server/405.vnc,password=on"]
    );
}
