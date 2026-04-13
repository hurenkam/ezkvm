use super::*;

#[test]
fn test_full_config_file_parsing() {
    let root = std::env::temp_dir().join(format!(
        "ezkvm-integration-parse-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();

    let temp_disk_path = root.join("test-disk.qcow2");
    let temp_config_path = root.join("test_config.yaml");

    let config_content = format!(
        r#"
name: "integration-test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 1
  cpu_model: "host"

boot:
  firmware: "bios"
  boot_order: ["disk"]

devices:
  drives:
    - id: "root"
      path: "{}"
      interface: "virtio"
      type: "disk"
      format: "qcow2"

  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "user"

  displays:
    - type: "qxl"
      vram: 64

options:
  enable_kvm: true
  daemonize: false
"#,
        temp_disk_path.display()
    );

    fs::write(&temp_config_path, config_content).unwrap();

    let config = VmConfig::from_file(&temp_config_path).unwrap();

    assert_eq!(config.name, "integration-test-vm");
    assert_eq!(config.system.memory, 1024);
    assert_eq!(config.devices.drives.len(), 1);
    assert_eq!(config.devices.networks.len(), 1);
    assert_eq!(config.devices.displays.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_config_validation() {
    let invalid_yaml = r#"
name: "invalid-vm"
backend: "qemu"

system:
  architecture: "invalid_arch"
  machine: "q35"
  memory: 1024
  vcpus: 2
  cpu_model: "host"
"#;

    let result = VmConfig::from_str(invalid_yaml);
    assert!(result.is_err());
}

#[test]
fn test_minimal_config() {
    let minimal_yaml = r#"
name: "minimal-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "pc"
  memory: 512
  vcpus: 1
  cpu_model: "qemu64"
"#;

    let config = VmConfig::from_str(minimal_yaml).unwrap();
    assert_eq!(config.name, "minimal-vm");
    assert_eq!(config.system.architecture, "x86_64");
    assert_eq!(config.system.memory, 512);
    assert_eq!(config.system.vcpus, 1);
    assert!(config.options.enable_kvm);
    assert!(!config.options.daemonize);
}
