use super::*;

#[test]
fn test_config_with_spice_audio_devices() {
    let yaml = r#"
name: "audio-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 4
  cpu_model: "host"

spice:
  enabled: true
  port: 5903
  addr: "0.0.0.0"
  disable_ticketing: true
  audio: true

audio_devices:
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
    assert_eq!(config.audio_devices.len(), 3);
    assert_eq!(config.audio_devices[0].r#type, "ich9-intel-hda");
    assert_eq!(config.audio_devices[0].bus.as_deref(), Some("pci.2"));
    assert_eq!(config.audio_devices[0].addr.as_deref(), Some("0xc"));
    assert_eq!(config.audio_devices[1].cad, Some(0));
    assert_eq!(
        config.audio_devices[1].audiodev.as_deref(),
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
  memory: 4096
  vcpus: 4
  cpu_model: "host"

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
  memory: 2048
  vcpus: 2
  cpu_model: "host"

spice:
  enabled: true
  vdagent: true

input_devices:
  - type: "virtio-mouse"
  - type: "virtio-keyboard"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.input_devices.len(), 2);
    assert_eq!(config.input_devices[0].r#type, "virtio-mouse");
    assert_eq!(config.input_devices[1].r#type, "virtio-keyboard");
}

#[test]
fn test_duplicate_input_devices_are_rejected() {
    let yaml = r#"
name: "input-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

input_devices:
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
  memory: 2048
  vcpus: 2
  cpu_model: "host"

ivshmem:
  enabled: true
  size: 128
  vectors: 1
  id: "ivshmem0"
  bus: "pcie.0"
  mem_path: "/dev/kvmfr0"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    let ivshmem = config.ivshmem.as_ref().unwrap();
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
  memory: 2048
  vcpus: 2
  cpu_model: "host"

ivshmem:
  enabled: true
  mem_path: "dev/kvmfr0"
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
  memory: 2048
  vcpus: 2
  cpu_model: "host"

xhci_controllers:
  - id: "xhci"
    p2: 15
    p3: 15
    bus: "pci.1"
    addr: "0x1b"

usb_devices:
  - id: "usb0"
    hostbus: "1"
    hostport: "2.2"
    bus: "xhci.0"
    port: "1"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.xhci_controllers.len(), 1);
    assert_eq!(config.xhci_controllers[0].p2, Some(15));
    assert_eq!(config.xhci_controllers[0].p3, Some(15));
    assert_eq!(config.xhci_controllers[0].bus.as_deref(), Some("pci.1"));
    assert_eq!(config.xhci_controllers[0].addr.as_deref(), Some("0x1b"));
    assert_eq!(config.usb_devices[0].hostbus.as_deref(), Some("1"));
    assert_eq!(config.usb_devices[0].hostport.as_deref(), Some("2.2"));
}

#[test]
fn test_usb_device_accepts_proxmox_host_form() {
    let yaml = r#"
name: "usb-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

usb_devices:
  - id: "usb0"
    host: "1-2.2"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    assert_eq!(config.usb_devices[0].host, "1-2.2");
}
