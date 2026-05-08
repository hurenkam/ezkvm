use crate::test_support::built_qemu_args_from_yaml;

#[test]
fn defaults_guest_agent_and_virtio_input_to_legacy_q35_bus() {
    let args = built_qemu_args_from_yaml(
        r#"
name: "vm-q35-legacy-defaults"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  readconfig:
    - "/usr/share/ezkvm/ezkvm-q35.cfg"
  memory:
    size: 4096
  cpu:
    vcpus: 4
    model: "host"

spice:
  enabled: true
  vdagent: true
  port: 5903
  addr: "127.0.0.1"

options:
  guest_agent:
    enabled: true

devices:
  input:
    - type: "virtio-mouse"
    - type: "virtio-keyboard"
"#,
    );

    assert!(
        args.iter()
            .any(|arg| arg == "virtio-serial,id=qga0,bus=pci.0,addr=0x8")
    );
    assert!(args.iter().any(|arg| {
        arg == "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0,bus=qga0.0"
    }));
    assert!(
        args.iter()
            .any(|arg| arg == "virtio-serial-pci,id=virtio-serial0,bus=pci.0,addr=0x9")
    );
    assert!(args.iter().any(|arg| {
        arg == "virtserialport,chardev=vdagent,name=com.redhat.spice.0,bus=virtio-serial0.0"
    }));
    assert!(args.iter().any(|arg| arg == "virtio-mouse,bus=pci.0"));
    assert!(args.iter().any(|arg| arg == "virtio-keyboard,bus=pci.0"));
}

#[test]
fn defaults_pvscsi_and_ide_drive_bus_on_q35_bridge_template() {
    let args = built_qemu_args_from_yaml(
        r#"
name: "vm-q35-storage-defaults"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  readconfig:
    - "/usr/share/ezkvm/ezkvm-q35.cfg"
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: "host"

devices:
  drives:
    - id: "ide2"
      interface: "ide"
      type: "cdrom"
      format: "raw"

controllers:
  scsi:
    - id: "scsihw0"
      type: "pvscsi"
"#,
    );

    assert!(
        args.iter()
            .any(|arg| arg == "pvscsi,id=scsihw0,bus=pci.0,addr=0x5")
    );
    assert!(
        args.iter()
            .any(|arg| arg == "ide-cd,drive=drive-ide2,id=ide2,bus=ide.1,unit=0")
    );
}

#[test]
fn defaults_explicit_xhci_bus_and_addr_on_q35_bridge_template() {
    let args = built_qemu_args_from_yaml(
        r#"
name: "vm-q35-xhci-defaults"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  readconfig:
    - "/usr/share/ezkvm/ezkvm-q35.cfg"
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: "host"

controllers:
  xhci:
    - id: "xhci0"

host:
  usb:
    - id: "usb0"
      hostbus: "1"
      hostport: "2.2"
"#,
    );

    assert!(
        args.iter()
            .any(|arg| arg == "qemu-xhci,id=xhci0,bus=pci.1,addr=0x1b")
    );
}
