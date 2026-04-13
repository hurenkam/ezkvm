use super::*;

#[test]
fn test_vm_config_from_file_deep_merges_nested_maps() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-deep-merge");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
        profile_dir.join("base_system.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: "host"
  boot:
    firmware: "uefi"
    menu: true
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("secure_boot.yaml"),
        r#"
system:
  boot:
    secure_boot: true
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
name: "deep-merge-test"
backend: "qemu"
profiles:
  - "base_system"
  - "secure_boot"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.system.boot.firmware.as_deref(), Some("uefi"));
    assert!(config.system.boot.menu);
    assert!(config.system.boot.secure_boot);

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_replaces_non_specialized_lists() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-list-replace");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
        profile_dir.join("base_system.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: "host"
devices:
  displays:
        - type: "qxl"
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("replace_displays.yaml"),
        r#"
devices:
  displays:
        - type: "cirrus"
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
name: "list-replace-test"
backend: "qemu"
profiles:
  - "base_system"
  - "replace_displays"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.devices.displays.len(), 1);
    assert_eq!(config.devices.displays[0].r#type, "cirrus");

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_uses_canonical_target_paths() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-mixed-legacy-target-precedence");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
        profile_dir.join("base.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
options:
  enable_kvm: true
  daemonize: false
  qmp:
    enabled: true
    socket_path: "/tmp/base.qmp"
controllers:
  scsi:
    - id: "base-scsi"
      type: "pvscsi"
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
name: "mixed-legacy-target-precedence-test"
backend: "qemu"
profiles:
  - "base"

options:
  enable_kvm: true
  daemonize: false
  qmp:
    enabled: true
    socket_path: "/tmp/target.qmp"

controllers:
  scsi:
    - id: "target-scsi"
      type: "pvscsi"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(
        config
            .options
            .qmp
            .as_ref()
            .and_then(|q| q.socket_path.as_deref()),
        Some("/tmp/target.qmp")
    );
    let controller_ids: Vec<&str> = config
        .controllers
        .scsi
        .iter()
        .map(|controller| controller.id.as_str())
        .collect();
    assert_eq!(controller_ids, vec!["base-scsi", "target-scsi"]);

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}
