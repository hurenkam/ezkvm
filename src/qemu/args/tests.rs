use crate::qemu::types::QemuArgs;

#[test]
fn test_basic_args() {
    let mut args = QemuArgs::new();
    args.add_memory(1024);
    args.add_smp(2);

    let built = args.build();
    assert_eq!(built, vec!["-m", "1024M", "-smp", "cpus=2"]);
}

#[test]
fn test_drive_args() {
    let mut args = QemuArgs::new();
    args.add_drive("/path/to/disk.qcow2", "virtio", "qcow2", false);

    let built = args.build();
    assert_eq!(
        built,
        vec!["-drive", "file=/path/to/disk.qcow2,if=virtio,format=qcow2"]
    );
}

#[test]
fn test_readonly_drive() {
    let mut args = QemuArgs::new();
    args.add_drive("/path/to/cd.iso", "ide", "raw", true);

    let built = args.build();
    assert_eq!(
        built,
        vec![
            "-drive",
            "file=/path/to/cd.iso,if=ide,format=raw,readonly=on"
        ]
    );
}

#[test]
fn test_uefi_uses_pflash_drives() {
    let mut args = QemuArgs::new();
    args.add_uefi(
        Some("/usr/share/OVMF_CODE.fd"),
        Some("/var/lib/vm/vars.fd"),
        None,
        true,
    );

    let built = args.build();
    assert_eq!(
        built,
        vec![
            "-drive",
            "if=pflash,unit=0,format=raw,readonly=on,file=/usr/share/OVMF_CODE.fd",
            "-drive",
            "if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/var/lib/vm/vars.fd",
        ]
    );
}

#[test]
fn test_uefi_with_vars_size() {
    let mut args = QemuArgs::new();
    args.add_uefi(
        Some("/usr/share/OVMF_CODE.fd"),
        Some("/var/lib/vm/vars.fd"),
        Some(540672),
        true,
    );

    let built = args.build();
    assert_eq!(built[2], "-drive");
    assert_eq!(
        built[3],
        "if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/var/lib/vm/vars.fd,size=540672"
    );
}

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
fn test_tpm_uses_server_mode_for_internal_emulator() {
    let tpm_sock = std::env::temp_dir().join("ezkvm-test-tpm.sock");
    let tpm_sock_str = tpm_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    let result = args.add_tpm("2.0", "emulator", &tpm_sock_str, "tpm-tis", false);
    assert!(result.is_ok());

    let built = args.build();
    assert_eq!(built[0], "-chardev");
    assert!(built[1].contains("socket,id=tpmchar,server=on,wait=off,path="));
    assert!(built[1].contains("ezkvm-test-tpm.sock"));
}

#[test]
fn test_tpm_omits_wait_for_external_swtpm_client_mode() {
    let tpm_sock = std::env::temp_dir().join("ezkvm-test-external-tpm.sock");
    let tpm_sock_str = tpm_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    let result = args.add_tpm("2.0", "emulator", &tpm_sock_str, "tpm-tis", true);
    assert!(result.is_ok());

    let built = args.build();
    assert_eq!(built[0], "-chardev");
    assert!(built[1].contains("socket,id=tpmchar,path="));
    assert!(built[1].contains("ezkvm-test-external-tpm.sock"));
    // Verify server=on,wait=off are NOT present for external swtpm
    assert!(!built[1].contains("server=on"));
    assert!(!built[1].contains("wait=off"));
}

#[test]
fn test_tpm_rejects_unsupported_backend() {
    let tpm_sock = std::env::temp_dir().join("ezkvm-test-invalid-tpm.sock");
    let tpm_sock_str = tpm_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    let result = args.add_tpm(
        "2.0",
        "unsupported-backend",
        &tpm_sock_str,
        "tpm-tis",
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unsupported TPM backend"));
}

#[test]
fn test_qmp_uses_listening_unix_socket() {
    let qmp_sock = std::env::temp_dir().join("ezkvm-test-qmp.sock");
    let qmp_sock_str = qmp_sock.to_string_lossy().to_string();

    let mut args = QemuArgs::new();
    args.add_qmp(Some(&qmp_sock_str), "unix");

    let built = args.build();
    assert_eq!(built[0], "-qmp");
    assert!(built[1].starts_with("unix:"));
    assert!(built[1].contains("ezkvm-test-qmp.sock"));
    assert!(built[1].contains("server=on"));
    assert!(built[1].contains("wait=off"));
}

#[test]
fn test_iscsi_disk_with_initiator_and_auth() {
    let mut args = QemuArgs::new();
    args.add_iscsi_disk(
        "iscsi0",
        "10.0.0.1:3260",
        "iqn.2024-01.example:storage.vm0",
        1,
        Some("iqn.1993-08.org.debian:01:622fd71731a1"),
        Some("chap-user"),
        Some("chap-pass"),
        Some("scsihw0"),
    );

    let built = args.build();
    assert_eq!(built[0], "-blockdev");
    assert!(built[1].contains("driver=iscsi,portal=10.0.0.1:3260,target=iqn.2024-01.example:storage.vm0,lun=1,node-name=iscsi0"));
    assert!(built[1].contains(",initiator-name=iqn.1993-08.org.debian:01:622fd71731a1"));
    assert!(built[1].contains(",user=chap-user"));
    assert!(built[1].contains(",password=chap-pass"));
    assert_eq!(built[2], "-device");
    assert!(built[3].contains("scsi-hd,drive=iscsi0,id=iscsi0,bus=scsihw0.0"));
}

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
    assert!(
        built.iter().any(|arg| arg
            == "hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0")
    );
    assert!(
        built.iter().any(|arg| arg
            == "hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0")
    );
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
