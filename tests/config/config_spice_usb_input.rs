use super::*;

#[test]
fn test_config_with_spice_audio_devices() {
    let yaml = r#"
name: "audio-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 4096
  cpu:
    vcpus: 4
    model: "host"
spice:
  enabled: true
  port: 5903
  addr: "0.0.0.0"
  disable_ticketing: true
  audio: true

devices:
  audio:
    - type: "ich9-intel-hda"
      id: "audiodev0"
      bus: "pci.2"
      addr: "0xc"
    - type: "hda-micro"
      id: "audiodev0-codec0"
      bus: "audiodev0.0"
      cad: 0
      audiodev: "spice-backend0"
    - type: "hda-duplex"
      id: "audiodev0-codec1"
      bus: "audiodev0.0"
      cad: 1
      audiodev: "spice-backend0"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.audio.len(), 3);
    assert_eq!(config.devices.audio[0].r#type, "ich9-intel-hda");
    assert_eq!(config.devices.audio[0].bus.as_deref(), Some("pci.2"));
    assert_eq!(config.devices.audio[0].addr.as_deref(), Some("0xc"));
    assert_eq!(config.devices.audio[1].cad, Some(0));
    assert_eq!(
        config.devices.audio[1].audiodev.as_deref(),
        Some("spice-backend0")
    );
}

#[test]
fn test_spice_audio_requires_audio_devices() {
    let yaml = r#"
name: "audio-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 4096
  cpu:
    vcpus: 4
    model: "host"
spice:
  enabled: true
  audio: true
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("SPICE audio requires at least one configured audio device")
    );
}

#[test]
fn test_config_with_input_devices() {
    let yaml = r#"
name: "input-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
spice:
  enabled: true
  vdagent: true

devices:
  input:
    - type: "virtio-mouse"
    - type: "virtio-keyboard"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.input.len(), 2);
    assert_eq!(config.devices.input[0].r#type, "virtio-mouse");
    assert_eq!(config.devices.input[1].r#type, "virtio-keyboard");
}

#[test]
fn test_config_with_usb_tablet_input_device() {
    let yaml = r#"
name: "input-vm"
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
  input:
    - type: "usb-tablet"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.devices.input.len(), 1);
    assert_eq!(config.devices.input[0].r#type, "usb-tablet");
}

#[test]
fn test_duplicate_input_devices_are_rejected() {
    let yaml = r#"
name: "input-vm"
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
  input:
    - type: "virtio-mouse"
    - type: "virtio-mouse"
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Duplicate input device type configured")
    );
}

#[test]
fn test_config_with_ivshmem_bus_and_mem_path() {
    let yaml = r#"
name: "ivshmem-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
    ivshmem:
      enabled: true
      size: 128
      vectors: 1
      id: "ivshmem0"
      bus: "pcie.0"
      mem_path: "/dev/kvmfr0"
  cpu:
    vcpus: 2
    model: "host"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    let ivshmem = config.system.memory.ivshmem.as_ref().unwrap();
    assert_eq!(ivshmem.id, "ivshmem0");
    assert_eq!(ivshmem.bus.as_deref(), Some("pcie.0"));
    assert_eq!(ivshmem.mem_path, "/dev/kvmfr0");
}

#[test]
fn test_ivshmem_mem_path_must_be_absolute() {
    let yaml = r#"
name: "ivshmem-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
    ivshmem:
      enabled: true
      mem_path: "dev/kvmfr0"
  cpu:
    vcpus: 2
    model: "host"
"#;

    let result = VmConfig::from_str(yaml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("ivshmem mem_path must be an absolute path")
    );
}

#[test]
fn test_config_with_xhci_controller_and_usb_hostport() {
    let yaml = r#"
name: "usb-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
controllers:
  xhci:
    - id: "xhci"
      p2: 15
      p3: 15
      bus: "pci.1"
      addr: "0x1b"

host:
  usb:
    - id: "usb0"
      hostbus: "1"
      hostport: "2.2"
      bus: "xhci.0"
      port: "1"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.controllers.xhci.len(), 1);
    assert_eq!(config.controllers.xhci[0].p2, Some(15));
    assert_eq!(config.controllers.xhci[0].p3, Some(15));
    assert_eq!(config.controllers.xhci[0].bus.as_deref(), Some("pci.1"));
    assert_eq!(config.controllers.xhci[0].addr.as_deref(), Some("0x1b"));
    assert_eq!(config.host.usb[0].hostbus.as_deref(), Some("1"));
    assert_eq!(config.host.usb[0].hostport.as_deref(), Some("2.2"));
}

#[test]
fn test_usb_device_accepts_proxmox_host_form() {
    let yaml = r#"
name: "usb-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"
host:
  usb:
    - id: "usb0"
      host: "1-2.2"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.host.usb[0].host, "1-2.2");
}
