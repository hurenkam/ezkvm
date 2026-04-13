use super::QemuArgs;

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
