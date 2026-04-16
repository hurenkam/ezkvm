use super::*;

#[test]
fn test_config_with_non_vfio_device_placement() {
    let yaml = r#"
name: "placement-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 4096
    ballooning:
      enabled: true
      model: "virtio-balloon-pci"
      id: "balloon0"
      bus: "pci.0"
      addr: "0x3"
  cpu:
    vcpus: 4
    model: "host"
devices:
  drives:
    - id: "scsi0"
      path: "/dev/vm1/vm-108-boot"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
      scsi_id: 0
      boot_index: 100
    - id: "ide2"
      path: ""
      interface: "ide"
      type: "cdrom"
      format: "raw"
      readonly: true
      bus: "ide.1"
      unit: 0
      boot_index: 101

options:
  enable_kvm: true
  daemonize: false
  guest_agent:
    enabled: true
    socket_path: "/var/run/qemu-server/108.qga"
    bus: "pci.0"
    addr: "0x8"

controllers:
  scsi:
    - id: "scsihw0"
      type: "pvscsi"
      bus: "pci.0"
      addr: "0x5"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.drives[0].scsi_id, Some(0));
    assert_eq!(config.devices.drives[0].boot_index, Some(100));
    assert_eq!(config.devices.drives[1].bus.as_deref(), Some("ide.1"));
    assert_eq!(config.devices.drives[1].unit, Some(0));
    assert_eq!(
        config.options.guest_agent.as_ref().unwrap().bus.as_deref(),
        Some("pci.0")
    );
    assert_eq!(
        config
            .system
            .memory
            .ballooning
            .as_ref()
            .unwrap()
            .addr
            .as_deref(),
        Some("0x3")
    );
    assert_eq!(config.controllers.scsi[0].addr.as_deref(), Some("0x5"));
}

#[test]
fn test_config_with_real_serial_backends() {
    let yaml = r#"
name: "serial-vm"
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
  serials:
    - type: "file"
      path: "/tmp/serial.log"
    - type: "socket"
      host: "127.0.0.1"
      socket_port: 4444
      server: true
      wait: false
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.serials.len(), 2);
    assert_eq!(
        config.devices.serials[0].path.as_deref(),
        Some("/tmp/serial.log")
    );
    assert_eq!(config.devices.serials[1].host.as_deref(), Some("127.0.0.1"));
    assert_eq!(config.devices.serials[1].socket_port, Some(4444));
}

#[test]
fn test_serial_file_backend_requires_path() {
    let yaml = r#"
name: "serial-vm"
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
  serials:
    - type: "file"
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Serial file backend requires a non-empty path")
    );
}

#[test]
fn test_serial_socket_backend_requires_host_and_port() {
    let yaml = r#"
name: "serial-vm"
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
  serials:
    - type: "socket"
      host: ""
      socket_port: 0
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Serial socket backend requires either a non-empty path or host")
            || error.contains("Serial socket backend requires a TCP port between 1 and 65535")
    );
}

#[test]
fn test_unsupported_display_vram_is_rejected() {
    let yaml = r#"
name: "display-vm"
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
  displays:
    - type: "cirrus"
      vram: 64
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("does not support configurable VRAM")
    );
}
