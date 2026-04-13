use super::*;

#[test]
fn test_basic_config_parsing() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    vcpus: 2
    model: "host"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.name, "test-vm");
    assert_eq!(config.backend, "qemu");
    assert_eq!(config.system.architecture, "x86_64");
    assert_eq!(config.system.machine, "q35");
    assert_eq!(config.system.memory.size, 1024);
    assert_eq!(config.system.cpu.vcpus, 2);
    assert_eq!(config.system.cpu.model, "host");
}

#[test]
fn test_config_with_devices() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - id: "root"
      path: "/path/to/disk.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
      cache: "none"
      aio: "io_uring"
      detect_zeroes: "unmap"

  networks:
    - id: "net0"
      model: "virtio-net"
      backend:
        type: "user"
      mac: "52:54:00:12:34:56"
      rx_queue_size: 1024
      tx_queue_size: 256
      boot_index: 102
      bus: "pci.0"
      addr: "0x12"

  displays:
    - type: "virtio-gpu"
      vram: 256
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.drives.len(), 1);
    assert_eq!(config.devices.networks.len(), 1);
    assert_eq!(config.devices.displays.len(), 1);

    let drive = &config.devices.drives[0];
    assert_eq!(drive.id, "root");
    assert_eq!(drive.path, "/path/to/disk.qcow2");
    assert_eq!(drive.interface, "virtio");
    assert_eq!(drive.cache.as_deref(), Some("none"));
    assert_eq!(drive.aio.as_deref(), Some("io_uring"));
    assert_eq!(drive.detect_zeroes.as_deref(), Some("unmap"));

    let network = &config.devices.networks[0];
    assert_eq!(network.id, "net0");
    assert_eq!(network.model, "virtio-net");
    assert_eq!(
        network.backend.as_ref().map(|b| b.backend_type.as_str()),
        Some("user")
    );
    assert_eq!(network.mac.as_ref().unwrap(), "52:54:00:12:34:56");
    assert_eq!(network.rx_queue_size, Some(1024));
    assert_eq!(network.tx_queue_size, Some(256));
    assert_eq!(network.boot_index, Some(102));
    assert_eq!(network.bus.as_deref(), Some("pci.0"));
    assert_eq!(network.addr.as_deref(), Some("0x12"));

    let display = &config.devices.displays[0];
    assert_eq!(display.r#type, "virtio-gpu");
    assert_eq!(display.vram.unwrap(), 256);
}

#[test]
fn test_empty_cdrom_path_is_allowed() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - id: "ide2"
      path: ""
      interface: "ide"
      type: "cdrom"
      format: "raw"
      readonly: true
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.drives[0].r#type, "cdrom");
    assert!(config.devices.drives[0].path.is_empty());
}

#[test]
fn test_omitted_cdrom_path_defaults_to_empty() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - interface: "ide"
      type: "cdrom"
      format: "raw"
      readonly: true
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.drives[0].r#type, "cdrom");
    assert_eq!(config.devices.drives[0].path, "");
}

#[test]
fn test_empty_disk_path_is_rejected() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - id: "disk0"
      path: ""
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#;

    let err = VmConfig::from_str(yaml).unwrap_err();
    assert!(
        err.to_string()
            .contains("Drive path cannot be empty unless drive type is cdrom")
    );
}

#[test]
fn test_env_var_substitution() {
    let _guard = env_lock().lock().unwrap();

    unsafe {
        std::env::set_var("TEST_MEMORY", "4096");
        std::env::set_var("TEST_CPUS", "4");
    }

    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: ${TEST_MEMORY}
  cpu:
    vcpus: ${TEST_CPUS}
    model: "host"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.system.memory.size, 4096);
    assert_eq!(config.system.cpu.vcpus, 4);

    unsafe {
        std::env::remove_var("TEST_MEMORY");
        std::env::remove_var("TEST_CPUS");
    }
}

#[test]
fn test_simple_env_var_substitution() {
    let _guard = env_lock().lock().unwrap();

    unsafe {
        std::env::set_var("HOME", "/home/test");
    }

    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - id: "root"
      path: "$HOME/disk.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.drives[0].path, "/home/test/disk.qcow2");

    unsafe {
        std::env::remove_var("HOME");
    }
}

#[test]
fn test_missing_drive_and_network_ids_are_generated() {
    let yaml = r#"
name: "generated-id-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - path: "/path/to/disk0.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
    - path: ""
      interface: "ide"
      type: "cdrom"
      format: "raw"
      readonly: true

  networks:
    - model: "virtio-net"
      backend:
        type: "user"
    - id: "uplink0"
      model: "e1000"
      backend:
        type: "user"
"#;

    let config = VmConfig::from_str(yaml).unwrap();

    assert_eq!(config.devices.drives[0].id, "virtio0");
    assert_eq!(config.devices.drives[1].id, "ide1");
    assert_eq!(config.devices.networks[0].id, "net0");
    assert_eq!(config.devices.networks[1].id, "uplink0");
}

#[test]
fn test_generated_device_ids_skip_explicit_id_collisions() {
    let yaml = r#"
name: "duplicate-generated-id-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
devices:
  drives:
    - id: "virtio1"
      path: "/path/to/disk0.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
    - path: "/path/to/disk1.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.drives[0].id, "virtio1");
    assert_eq!(config.devices.drives[1].id, "virtio2");
}

#[test]
fn test_missing_env_var() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: ${MISSING_VAR}
  cpu:
    vcpus: 2
    model: "host"
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("MISSING_VAR"));
}

#[test]
fn test_invalid_config_missing_required_fields() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  # missing machine, memory, vcpus, cpu_model
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_canonical_cpu_features_override_legacy_cpu_features_when_both_present() {
    let yaml = r#"
name: "cpu-feature-precedence-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 1024
  cpu:
    vcpus: 2
    model: "host"
    features:
      - "target_feature"
  cpu_features:
    - "legacy_feature"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.system.cpu.features, vec!["target_feature"]);
}
