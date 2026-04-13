use super::*;

#[test]
fn test_vm_config_from_file_merges_profiles_from_profile_dir() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-merge");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
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

    std::fs::write(
        profile_dir.join("gpu_passthrough.yaml"),
        r#"
devices:
  displays: []
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
name: "test-vm"
backend: "qemu"
profiles:
  - "windows_11"
  - "gpu_passthrough"
system:
  memory: 8192
  vcpus: 8
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();
    assert_eq!(config.name, "test-vm");
    assert_eq!(config.backend, "qemu");
    assert_eq!(config.system.architecture, "x86_64");
    assert_eq!(config.system.machine, "q35");
    assert_eq!(config.system.cpu_model, "host");
    assert_eq!(config.system.memory, 8192);
    assert_eq!(config.system.vcpus, 8);
    assert!(config.devices.displays.is_empty());

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_errors_on_missing_profile_file() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-missing");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

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
name: "test-vm"
backend: "qemu"
profiles:
  - "does_not_exist"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let err = VmConfig::from_file(&vm_config_path)
        .unwrap_err()
        .to_string();
    assert!(err.contains("Unknown profile 'does_not_exist'"));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}
