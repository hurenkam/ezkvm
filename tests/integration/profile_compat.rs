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
