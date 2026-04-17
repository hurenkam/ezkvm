use super::*;

#[test]
fn normalizes_nested_scsi_drives_and_xhci_usb_into_canonical_sections() {
    let yaml = r#"
name: nested-controller-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
controllers:
  scsi:
    - type: pvscsi
      drives:
        - path: /dev/vm1/vm-108-boot
          interface: scsi
          type: disk
          format: raw
  xhci:
    - p2: 15
      p3: 15
      usb:
        - hostbus: "1"
          hostport: "2.2"
"#;

    let config = VmConfig::from_str(yaml).expect("config should parse");

    assert_eq!(config.controllers.scsi.len(), 1);
    assert_eq!(config.controllers.scsi[0].id, "scsihw0");

    assert_eq!(config.devices.drives.len(), 1);
    assert_eq!(config.devices.drives[0].controller.as_deref(), Some("scsihw0"));
    assert_eq!(config.devices.drives[0].id, "scsi0");

    assert_eq!(config.controllers.xhci.len(), 1);
    assert_eq!(config.controllers.xhci[0].id, "xhci");

    assert_eq!(config.host.usb.len(), 1);
    assert_eq!(config.host.usb[0].bus.as_deref(), Some("xhci.0"));
    assert_eq!(config.host.usb[0].id, "usb0");
}

#[test]
fn preserves_explicit_controller_links_in_nested_shape() {
    let yaml = r#"
name: nested-controller-vm-explicit
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
controllers:
  scsi:
    - id: my-scsi
      type: pvscsi
      drives:
        - id: bootdisk
          path: /dev/vm1/vm-108-boot
          interface: scsi
          type: disk
          format: raw
          controller: override-scsi
  xhci:
    - id: usb-root
      p2: 8
      usb:
        - id: passthrough0
          hostbus: "1"
          hostport: "2.1"
          bus: custom-usb.0
"#;

    let config = VmConfig::from_str(yaml).expect("config should parse");

    assert_eq!(config.controllers.scsi[0].id, "my-scsi");
    assert_eq!(config.devices.drives[0].id, "bootdisk");
    assert_eq!(
        config.devices.drives[0].controller.as_deref(),
        Some("override-scsi")
    );

    assert_eq!(config.controllers.xhci[0].id, "usb-root");
    assert_eq!(config.host.usb[0].id, "passthrough0");
    assert_eq!(config.host.usb[0].bus.as_deref(), Some("custom-usb.0"));
}

#[test]
fn normalizes_devices_sequence_controller_blocks() {
    let yaml = r#"
name: devices-sequence-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
devices:
  - controller: pvscsi
    drives:
      - path: /dev/vm1/vm-108-boot
        type: disk
        format: raw
  - controller: xhci
    usb:
      - hostbus: "1"
        hostport: "2.2"
"#;

    let config = VmConfig::from_str(yaml).expect("config should parse");

    assert_eq!(config.controllers.scsi.len(), 1);
    assert_eq!(config.controllers.scsi[0].r#type, "pvscsi");
    assert_eq!(config.devices.drives.len(), 1);
    assert_eq!(config.devices.drives[0].interface, "scsi");
    assert_eq!(config.devices.drives[0].controller.as_deref(), Some("scsihw0"));

    assert_eq!(config.controllers.xhci.len(), 1);
    assert_eq!(config.controllers.xhci[0].id, "xhci");
    assert_eq!(config.host.usb.len(), 1);
    assert_eq!(config.host.usb[0].bus.as_deref(), Some("xhci.0"));
}

#[test]
fn keeps_legacy_devices_drives_shape_supported() {
    let yaml = r#"
name: legacy-devices-drives
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
devices:
  drives:
    - path: /dev/vm1/vm-108-boot
      interface: scsi
      type: disk
      format: raw
"#;

    let config = VmConfig::from_str(yaml).expect("legacy devices.drives should still parse");
    assert_eq!(config.devices.drives.len(), 1);
    assert_eq!(config.devices.drives[0].interface, "scsi");
}
