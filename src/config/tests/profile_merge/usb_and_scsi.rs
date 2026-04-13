use super::*;

#[test]
fn test_vm_config_from_file_merges_usb_devices_by_id() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-usb-id-merge");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
        profile_dir.join("base.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
usb_devices:
  - id: "usb0"
    hostbus: "1"
    hostport: "2.2"
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("usb_overlay.yaml"),
        r#"
usb_devices:
  - id: "usb0"
    bus: "xhci.0"
    port: "1"
  - id: "usb1"
    hostbus: "1"
    hostport: "2.3"
"#,
    )
    .unwrap();

    let central_config_path = root.join("ezkvm.yaml");
    std::fs::write(
        &central_config_path,
        format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
    )
    .unwrap();

    let vm_config_path = root.join("vm.yaml");
    std::fs::write(
        &vm_config_path,
        r#"
name: "usb-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "usb_overlay"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.usb_devices.len(), 2);

    let usb0 = config.usb_devices.iter().find(|u| u.id == "usb0").unwrap();
    assert_eq!(usb0.hostbus.as_deref(), Some("1"));
    assert_eq!(usb0.hostport.as_deref(), Some("2.2"));
    assert_eq!(usb0.bus.as_deref(), Some("xhci.0"));
    assert_eq!(usb0.port.as_deref(), Some("1"));

    let usb1 = config.usb_devices.iter().find(|u| u.id == "usb1").unwrap();
    assert_eq!(usb1.hostport.as_deref(), Some("2.3"));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_merges_scsi_controllers_by_id() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-scsi-id-merge");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
        profile_dir.join("base.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
scsi_controllers:
  - id: "scsihw0"
    type: "pvscsi"
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("scsi_overlay.yaml"),
        r#"
scsi_controllers:
  - id: "scsihw0"
    bus: "pci.0"
    addr: "0x5"
  - id: "scsihw1"
    type: "virtio-scsi-pci"
    bus: "pci.0"
    addr: "0x6"
"#,
    )
    .unwrap();

    let central_config_path = root.join("ezkvm.yaml");
    std::fs::write(
        &central_config_path,
        format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
    )
    .unwrap();

    let vm_config_path = root.join("vm.yaml");
    std::fs::write(
        &vm_config_path,
        r#"
name: "scsi-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "scsi_overlay"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.scsi_controllers.len(), 2);

    let scsihw0 = config
        .scsi_controllers
        .iter()
        .find(|c| c.id == "scsihw0")
        .unwrap();
    assert_eq!(scsihw0.r#type, "pvscsi");
    assert_eq!(scsihw0.bus.as_deref(), Some("pci.0"));
    assert_eq!(scsihw0.addr.as_deref(), Some("0x5"));

    let scsihw1 = config
        .scsi_controllers
        .iter()
        .find(|c| c.id == "scsihw1")
        .unwrap();
    assert_eq!(scsihw1.r#type, "virtio-scsi-pci");

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}
