use super::*;

#[test]
fn test_profile_based_config_file_parsing() {
    let _guard = env_lock().lock().unwrap();

    let root = std::env::temp_dir().join(format!(
        "ezkvm-integration-profile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();

    let profile_dir = root.join("profiles");
    fs::create_dir_all(&profile_dir).unwrap();
    fs::write(
        profile_dir.join("windows_11.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
boot:
  firmware: "uefi"
"#,
    )
    .unwrap();

    let central_path = root.join("ezkvm.yaml");
    fs::write(
        &central_path,
        format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
    )
    .unwrap();

    let vm_path = root.join("vm.yaml");
    fs::write(
        &vm_path,
        r#"
name: "integration-profile-vm"
backend: "qemu"
profiles:
  - "windows_11"
system:
  memory: 8192
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_path);
    }
    let config = VmConfig::from_file(&vm_path).unwrap();
    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }

    assert_eq!(config.name, "integration-profile-vm");
    assert_eq!(config.system.architecture, "x86_64");
    assert_eq!(config.system.memory, 8192);
    assert_eq!(config.boot.firmware.as_deref(), Some("uefi"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_legacy_non_profile_config_still_parses() {
    let _guard = env_lock().lock().unwrap();

    let legacy_yaml = r#"
name: "legacy-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"
"#;

    let config = VmConfig::from_str(legacy_yaml).unwrap();
    assert_eq!(config.name, "legacy-vm");
    assert!(config.profiles.is_empty());
    assert_eq!(config.system.memory, 2048);
}

#[test]
fn test_profile_policies_apply_to_legacy_network_mode_with_auto_placement() {
    let _guard = env_lock().lock().unwrap();

    let root = std::env::temp_dir().join(format!(
        "ezkvm-integration-profile-legacy-placement-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();

    let profile_dir = root.join("profiles");
    fs::create_dir_all(&profile_dir).unwrap();
    fs::write(
        profile_dir.join("network_defaults.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
policies:
  networks:
    - match:
        backend_type: "tap"
      defaults:
        model: "virtio-net-pci"
        rx_queue_size: 1024
      placement:
        addr:
          scope: "bus"
          bus: "pci.0"
          start: "0x12"
          step: 1
"#,
    )
    .unwrap();

    let central_path = root.join("ezkvm.yaml");
    fs::write(
        &central_path,
        format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
    )
    .unwrap();

    let vm_path = root.join("vm.yaml");
    fs::write(
        &vm_path,
        r#"
name: "integration-legacy-placement-vm"
backend: "qemu"
profiles:
  - "network_defaults"
devices:
  networks:
    - id: "net0"
      mode: "tap,ifname=tap0,script=no,downscript=no,vhost=on"
      bus: "pci.0"
    - id: "net1"
      mode: "tap,ifname=tap1,script=no,downscript=no,vhost=on"
      bus: "pci.0"
      addr: "0x14"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_path);
    }
    let config = VmConfig::from_file(&vm_path).unwrap();
    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }

    let net0 = &config.devices.networks[0];
    assert_eq!(net0.model, "virtio-net-pci");
    assert_eq!(net0.rx_queue_size, Some(1024));
    assert_eq!(net0.addr.as_deref(), Some("0x12"));
    assert_eq!(
        net0.mode,
        "tap,ifname=tap0,script=no,downscript=no,vhost=on"
    );

    let net1 = &config.devices.networks[1];
    assert_eq!(net1.addr.as_deref(), Some("0x14"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_profile_policies_apply_to_additional_device_families() {
    let _guard = env_lock().lock().unwrap();

    let root = std::env::temp_dir().join(format!(
        "ezkvm-integration-profile-extra-families-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();

    let profile_dir = root.join("profiles");
    fs::create_dir_all(&profile_dir).unwrap();
    fs::write(
        profile_dir.join("device_defaults.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
policies:
  displays:
    - match:
        type: "qxl"
      defaults:
        vram: 64
  scsi_controllers:
    - match:
        type: "pvscsi"
      defaults:
        bus: "pci.0"
        addr: "0x5"
"#,
    )
    .unwrap();

    let central_path = root.join("ezkvm.yaml");
    fs::write(
        &central_path,
        format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
    )
    .unwrap();

    let vm_path = root.join("vm.yaml");
    fs::write(
        &vm_path,
        r#"
name: "integration-extra-families-vm"
backend: "qemu"
profiles:
  - "device_defaults"
devices:
  displays:
    - type: "qxl"
scsi_controllers:
  - id: "scsihw0"
    type: "pvscsi"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_path);
    }
    let config = VmConfig::from_file(&vm_path).unwrap();
    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }

    assert_eq!(config.devices.displays[0].vram, Some(64));
    assert_eq!(config.scsi_controllers[0].bus.as_deref(), Some("pci.0"));
    assert_eq!(config.scsi_controllers[0].addr.as_deref(), Some("0x5"));

    let _ = fs::remove_dir_all(root);
}
