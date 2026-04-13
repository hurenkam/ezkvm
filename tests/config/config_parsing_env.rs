use super::*;

#[test]
fn test_basic_config_parsing() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 2
  cpu_model: "host"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.name, "test-vm");
    assert_eq!(config.backend, "qemu");
    assert_eq!(config.system.architecture, "x86_64");
    assert_eq!(config.system.machine, "q35");
    assert_eq!(config.system.memory, 1024);
    assert_eq!(config.system.vcpus, 2);
    assert_eq!(config.system.cpu_model, "host");
}

#[test]
fn test_config_with_devices() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

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
      mode: "user"
      backend: ~
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
    assert!(network.backend.is_none());
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
  memory: 2048
  vcpus: 2
  cpu_model: "host"

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
fn test_empty_disk_path_is_rejected() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

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
  memory: ${TEST_MEMORY}
  vcpus: ${TEST_CPUS}
  cpu_model: "host"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.system.memory, 4096);
    assert_eq!(config.system.vcpus, 4);

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
  memory: 2048
  vcpus: 2
  cpu_model: "host"

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
fn test_missing_env_var() {
    let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: ${MISSING_VAR}
  vcpus: 2
  cpu_model: "host"
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
