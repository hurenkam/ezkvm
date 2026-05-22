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
    assert!(
        args.iter()
            .any(|arg| { arg == "virtserialport,chardev=qga0,name=org.qemu.guest_agent.0" })
    );
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

#[test]
fn portable_q35_synthesizes_topology_without_loading_ezkvm_template() {
    let args = built_qemu_args_from_yaml(
        r#"
name: "vm-q35-dynamic-synth"
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

devices:
  networks:
    - id: "net0"
      model: "virtio-net"
      backend:
        type: "user"
      bus: "pci.2"

host:
  pci:
    - id: "hostpci0"
      device: "0000:03:00.0"
      pcie: true
"#,
    );

    assert!(
        !args
            .iter()
            .any(|arg| arg == "/usr/share/ezkvm/ezkvm-q35.cfg")
    );
    assert!(
        args.iter()
            .any(|arg| arg == "pcie-root-port,id=ich9-pcie-port-1,x-speed=16,x-width=32,multifunction=on,bus=pcie.0,addr=1c.0,port=1,chassis=1")
    );
    assert!(
        args.iter()
            .any(|arg| arg == "i82801b11-bridge,id=pcidmi,bus=pcie.0,addr=1e.0")
    );
    assert!(
        args.iter()
            .any(|arg| arg == "pci-bridge,id=pci.2,bus=pcidmi,addr=3.0,chassis_nr=3")
    );
    assert!(!args.iter().any(|arg| arg.contains("id=pci.0")));
    assert!(!args.iter().any(|arg| arg.contains("id=pci.1")));
    assert!(!args.iter().any(|arg| arg.contains("id=pci.3")));
}

#[test]
fn portable_q35_only_emits_uhci_companions_for_active_ehci() {
    let args = built_qemu_args_from_yaml(
        r#"
name: "vm-q35-ehci-demand"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  readconfig:
    - "/usr/share/ezkvm/ezkvm-q35.cfg"
  memory:
    size: 2048
  cpu:
    vcpus: 2
    model: "host"

devices:
  input:
    - type: "usb-tablet"
"#,
    );

    assert!(
        args.iter()
            .any(|arg| arg == "ich9-usb-ehci1,id=ehci,multifunction=on,bus=pcie.0,addr=1d.7")
    );
    assert!(args.iter().any(|arg| arg.contains("id=uhci1")));
    assert!(args.iter().any(|arg| arg.contains("id=uhci2")));
    assert!(args.iter().any(|arg| arg.contains("id=uhci3")));
    assert!(!args.iter().any(|arg| arg.contains("id=ehci-2")));
    assert!(!args.iter().any(|arg| arg.contains("id=uhci-4")));
    assert!(!args.iter().any(|arg| arg.contains("id=uhci-5")));
    assert!(!args.iter().any(|arg| arg.contains("id=uhci-6")));
}

#[test]
fn portable_q35_keeps_explicit_hostpci_bus_and_synthesizes_that_port() {
    let args = built_qemu_args_from_yaml(
        r#"
name: "vm-q35-explicit-port"
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

host:
  pci:
    - id: "hostpci0"
      device: "0000:03:00.0"
      pcie: true
      bus: "ich9-pcie-port-3"
      addr: "0x0.0"
"#,
    );

    assert!(args.iter().any(|arg| {
        arg == "pcie-root-port,id=ich9-pcie-port-3,x-speed=16,x-width=32,multifunction=on,bus=pcie.0,addr=1c.2,port=3,chassis=3"
    }));
    assert!(args.iter().any(|arg| {
        arg == "vfio-pci,host=0000:03:00.0,id=hostpci0,bus=ich9-pcie-port-3,addr=0x0.0"
    }));
}
