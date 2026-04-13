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
  memory:
    size: 1024
  cpu:
    vcpus: 1
    model: "host"
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
      backend:
        type: "user"

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
    assert_eq!(config.system.memory.size, 1024);
    assert_eq!(config.devices.drives.len(), 1);
    assert_eq!(config.devices.networks.len(), 1);
    assert_eq!(config.devices.displays.len(), 1);
    assert_eq!(
        config.devices.networks[0]
            .backend
            .as_ref()
            .map(|backend| backend.backend_type.as_str()),
        Some("user")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_legacy_network_mode_remains_supported() {
    let yaml = r#"
name: "canonical-network-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 512
  cpu:
    vcpus: 1
    model: "host"

devices:
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      backend:
        type: "tap"
        ifname: "tap0"
        script: "no"
        downscript: "no"
        vhost: true
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    let network = &config.devices.networks[0];
    assert!(network.backend.is_some());
    let resolved = network.resolved_backend().unwrap();
    assert_eq!(resolved.backend_type, "tap");
    assert_eq!(resolved.ifname.as_deref(), Some("tap0"));
    assert_eq!(resolved.script.as_deref(), Some("no"));
    assert_eq!(resolved.downscript.as_deref(), Some("no"));
    assert_eq!(resolved.vhost, Some(true));
}

#[test]
fn test_missing_device_ids_are_generated_when_loading_from_file() {
    let root = std::env::temp_dir().join(format!(
        "ezkvm-integration-generated-ids-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();

    let temp_disk_path = root.join("generated-disk.qcow2");
    let temp_config_path = root.join("generated_ids.yaml");
    fs::write(&temp_disk_path, "").unwrap();

    let config_content = format!(
        r#"
name: "generated-id-file-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    vcpus: 1
    model: "host"

devices:
  drives:
    - path: "{}"
      interface: "scsi"
      type: "disk"
      format: "qcow2"

  networks:
    - model: "virtio-net"
      backend:
        type: "user"
"#,
        temp_disk_path.display()
    );

    fs::write(&temp_config_path, config_content).unwrap();

    let config = VmConfig::from_file(&temp_config_path).unwrap();

    assert_eq!(config.devices.drives[0].id, "scsi0");
    assert_eq!(config.devices.networks[0].id, "net0");

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
  memory:
    size: 1024
  cpu:
    vcpus: 2
    model: "host"
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
  memory:
    size: 512
  cpu:
    vcpus: 1
    model: "qemu64"
"#;

    let config = VmConfig::from_str(minimal_yaml).unwrap();
    assert_eq!(config.name, "minimal-vm");
    assert_eq!(config.system.architecture, "x86_64");
    assert_eq!(config.system.memory.size, 512);
    assert_eq!(config.system.cpu.vcpus, 1);
    assert!(config.options.enable_kvm);
    assert!(!config.options.daemonize);
}

#[test]
fn test_new_schema_paths_are_normalized_before_validation() {
    let yaml = r#"
name: "normalized-schema-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
    ballooning:
      enabled: true
      model: "virtio-balloon-pci"
    ivshmem:
      enabled: true
      mem_path: "/dev/kvmfr0"
  cpu:
    model: "host"
    vcpus: 1
    features:
      - "hv_time"
      - "kvm=off"
  boot:
    firmware: "bios"

options:
  enable_kvm: true
  daemonize: false
  guest_agent:
    enabled: true
  qmp:
    enabled: true
    socket_path: "/tmp/ezkvm-test.qmp"

controllers:
  scsi:
    - id: "scsihw0"
      type: "pvscsi"
  xhci:
    - id: "xhci0"

host:
  pci:
    - device: "0000:03:00.0"
      id: "hostpci0"
  usb:
    - id: "usb0"
      hostbus: "1"
      hostport: "2.1"

devices:
  input:
    - type: "virtio-mouse"
"#;

    let config = VmConfig::from_str(yaml).unwrap();

    assert_eq!(config.system.memory.size, 1024);
    assert_eq!(config.system.cpu.features, vec!["hv_time", "kvm=off"]);
    assert_eq!(config.system.boot.firmware.as_deref(), Some("bios"));
    assert!(
        config
            .options
            .guest_agent
            .as_ref()
            .map(|g| g.enabled)
            .unwrap_or(false)
    );
    assert_eq!(
        config
            .options
            .qmp
            .as_ref()
            .and_then(|q| q.socket_path.as_deref()),
        Some("/tmp/ezkvm-test.qmp")
    );
    assert!(config.system.memory.ballooning.is_some());
    assert!(config.system.memory.ivshmem.is_some());
    assert_eq!(config.controllers.scsi.len(), 1);
    assert_eq!(config.controllers.xhci.len(), 1);
    assert_eq!(config.host.pci.len(), 1);
    assert_eq!(config.host.usb.len(), 1);
    assert_eq!(config.devices.input.len(), 1);
}

#[test]
fn test_legacy_scsi_controllers_path_is_rejected() {
    let yaml = r#"
name: "list-concat-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    model: "host"
    vcpus: 1

scsi_controllers:
  - id: "legacy-scsi"
    type: "pvscsi"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert!(config.controllers.scsi.is_empty());
}

#[test]
fn test_legacy_scalar_paths_are_rejected() {
    let yaml = r#"
name: "mixed-scalar-precedence-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
    ballooning:
      enabled: true
      model: "virtio-balloon-pci"
    ivshmem:
      enabled: true
      mem_path: "/dev/kvmfr0"
  cpu:
    model: "host"
    vcpus: 1
  boot:
    firmware: "bios"
  tpm:
    version: "2.0"
    backend: "emulator"

guest_agent:
  enabled: true

options:
  enable_kvm: true
  daemonize: false
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert!(config.options.guest_agent.is_none());
}

#[test]
fn test_legacy_hostpci_path_is_rejected() {
    let yaml = r#"
name: "host-pci-list-concat-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    model: "host"
    vcpus: 1

hostpci:
  - device: "0000:03:00.0"
    id: "legacy-hostpci0"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert!(config.host.pci.is_empty());
}

#[test]
fn test_system_memory_object_requires_size_field() {
    let yaml = r#"
name: "invalid-memory-object-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    ballooning:
      enabled: true
  cpu:
    model: "host"
    vcpus: 1

options:
  enable_kvm: true
  daemonize: false
"#;

    let err = VmConfig::from_str(yaml).unwrap_err();
    assert!(err.to_string().contains("missing field `size`"));
}

#[test]
fn test_new_list_paths_require_sequence_type() {
    let yaml = r#"
name: "invalid-controllers-scsi-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    model: "host"
    vcpus: 1

controllers:
  scsi:
    id: "not-a-list"

options:
  enable_kvm: true
  daemonize: false
"#;

    let err = VmConfig::from_str(yaml).unwrap_err();
    assert!(err.to_string().contains("invalid type"));
}

#[test]
fn test_new_object_paths_require_mapping_type() {
    let yaml = r#"
name: "invalid-options-qmp-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    model: "host"
    vcpus: 1

options:
  enable_kvm: true
  daemonize: false
  qmp: true
"#;

    let err = VmConfig::from_str(yaml).unwrap_err();
    assert!(err.to_string().contains("invalid type"));
}
