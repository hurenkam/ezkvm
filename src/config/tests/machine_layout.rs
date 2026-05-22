use super::*;

#[test]
fn parses_hierarchy_first_machine_layout_buses() {
    let yaml = r#"
name: vm-layout-tree
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
  machine_layout:
    buses:
      - id: pcie.0
        devices:
          - id: ich9-pcie-port-1
            driver: pcie-root-port
            addr: "1c.0"
            buses:
              - id: ich9-pcie-port-1.0
                devices:
                  - id: hostpci0
                    driver: vfio-pci
                    addr: "0x0.0"
"#;

    let config = VmConfig::from_str(yaml).expect("config should parse");
    let layout = config
        .canonical_machine_layout()
        .expect("layout should normalize");

    assert_eq!(layout.buses.len(), 1);
    assert_eq!(layout.buses[0].id, "pcie.0");
    assert_eq!(layout.buses[0].devices[0].id, "ich9-pcie-port-1");
}

#[test]
fn normalizes_flat_nodes_into_hierarchy_tree() {
    let yaml = r#"
name: vm-layout-nodes
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
  machine_layout:
    nodes:
      - id: pcie.0
        kind: bus
      - id: ich9-pcie-port-1
        kind: device
        driver: pcie-root-port
        addr: "1c.0"
        parent_id: pcie.0
      - id: ich9-pcie-port-1.0
        kind: bus
        parent_id: ich9-pcie-port-1
      - id: hostpci0
        kind: device
        driver: vfio-pci
        addr: "0x0.0"
        parent_id: ich9-pcie-port-1.0
"#;

    let config = VmConfig::from_str(yaml).expect("config should parse");
    let layout = config
        .canonical_machine_layout()
        .expect("layout should normalize");

    assert!(layout.nodes.is_empty());
    assert_eq!(layout.buses[0].id, "pcie.0");
    assert_eq!(layout.buses[0].devices[0].id, "ich9-pcie-port-1");
    assert_eq!(layout.buses[0].devices[0].buses[0].id, "ich9-pcie-port-1.0");
}

#[test]
fn rejects_duplicate_machine_layout_ids() {
    let yaml = r#"
name: vm-layout-dup
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
  machine_layout:
    nodes:
      - id: pcie.0
        kind: bus
      - id: pcie.0
        kind: bus
"#;

    let err = VmConfig::from_str(yaml).expect_err("duplicate ids must fail");
    assert!(err.to_string().contains("duplicate id"));
}

#[test]
fn rejects_machine_layout_broken_parent_links() {
    let yaml = r#"
name: vm-layout-parent
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
  machine_layout:
    nodes:
      - id: pcie.0
        kind: bus
      - id: hostpci0
        kind: device
        driver: vfio-pci
        parent_id: does-not-exist
"#;

    let err = VmConfig::from_str(yaml).expect_err("broken parent link must fail");
    assert!(err.to_string().contains("broken parent link"));
}

#[test]
fn rejects_machine_layout_parent_cycles() {
    let yaml = r#"
name: vm-layout-cycle
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    model: host
    vcpus: 4
  machine_layout:
    nodes:
      - id: pcie.0
        kind: bus
        parent_id: hostpci0
      - id: hostpci0
        kind: device
        driver: vfio-pci
        parent_id: pcie.0
"#;

    let err = VmConfig::from_str(yaml).expect_err("cycles must fail");
    assert!(err.to_string().contains("cycle"));
}

#[test]
fn existing_flat_configs_remain_valid_with_inferred_layout() {
    let yaml = r#"
name: vm-layout-flat
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
  networks:
    - id: net0
      model: virtio-net-pci
      backend:
        type: user
      bus: pci.0
      addr: "0x12"
controllers:
  scsi:
    - id: scsihw0
      type: pvscsi
      bus: pci.0
      addr: "0x5"
host:
  pci:
    - id: hostpci0
      device: "0000:03:00.0"
      bus: ich9-pcie-port-1
      addr: "0x0.0"
"#;

    let config = VmConfig::from_str(yaml).expect("legacy shape should stay valid");
    let layout = config
        .canonical_machine_layout()
        .expect("inferred layout should succeed");

    assert!(!layout.buses.is_empty());
}
