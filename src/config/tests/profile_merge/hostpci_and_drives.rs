use super::*;

#[test]
fn test_vm_config_from_file_merges_hostpci_by_id() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-hostpci-id-merge");
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
hostpci:
  - id: "hostpci0.0"
    device: "0000:03:00.0"
    bus: "ich9-pcie-port-1"
    multifunction: true
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("gpu_overlay.yaml"),
        r#"
hostpci:
  - id: "hostpci0.0"
    addr: "0x0.0"
  - id: "hostpci0.1"
    device: "0000:03:00.1"
    bus: "ich9-pcie-port-1"
    addr: "0x0.1"
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
name: "hostpci-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "gpu_overlay"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.hostpci.len(), 2);

    let gpu0 = config
        .hostpci
        .iter()
        .find(|d| d.id == "hostpci0.0")
        .unwrap();
    assert_eq!(gpu0.device, "0000:03:00.0");
    assert_eq!(gpu0.bus.as_deref(), Some("ich9-pcie-port-1"));
    assert_eq!(gpu0.addr.as_deref(), Some("0x0.0"));
    assert!(gpu0.multifunction);

    let gpu1 = config
        .hostpci
        .iter()
        .find(|d| d.id == "hostpci0.1")
        .unwrap();
    assert_eq!(gpu1.device, "0000:03:00.1");

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_merges_devices_drives_by_id() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-drives-id-merge");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    let root_disk = root.join("root.qcow2");
    let data_disk = root.join("data.raw");

    std::fs::write(
        profile_dir.join("base.yaml"),
        format!(
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
devices:
  drives:
    - id: "root"
      path: "{}"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#,
            root_disk.display()
        ),
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("drive_overlay.yaml"),
        format!(
            r#"
devices:
  drives:
    - id: "root"
      cache: "none"
      boot_index: 100
    - id: "data"
      path: "{}"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
"#,
            data_disk.display()
        ),
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
name: "drive-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "drive_overlay"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.devices.drives.len(), 2);

    let root_drive = config
        .devices
        .drives
        .iter()
        .find(|d| d.id == "root")
        .unwrap();
    assert_eq!(root_drive.path, root_disk.to_str().unwrap());
    assert_eq!(root_drive.cache.as_deref(), Some("none"));
    assert_eq!(root_drive.boot_index, Some(100));

    let data_drive = config
        .devices
        .drives
        .iter()
        .find(|d| d.id == "data")
        .unwrap();
    assert_eq!(data_drive.interface, "scsi");
    assert_eq!(data_drive.format, "raw");

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}
