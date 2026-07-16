use std::{str::FromStr, sync::Arc};

use crate::{
    config::EzkvmConfigSchema,
    runtime::{PvScsi, Q35Chipset, Runtime, SataAddress, ScsiAddress, Ssd},
};

mod config;
mod runtime;
mod serde_yaml;

fn main() {
    let builder = runtime::RuntimeBuilder::new();
    let bus_devices = builder.bus_devices();
    let chipset = Q35Chipset::new(bus_devices.clone());
    builder.with_memory(runtime::Memory::new(1024));
    builder.with_chipset(runtime::Chipset::Q35(chipset));
    builder.with_pcie_device(
        Arc::new(PvScsi::new(bus_devices.clone())),
        runtime::PcieAddress {
            bus: 0,
            device: 0,
            function: 0,
        },
    );
    builder.with_sata_device(
        Arc::new(Ssd::new()),
        SataAddress {
            bus: 0,
            port: 0,
            device: 0,
        },
    );
    builder.with_scsi_device(
        Arc::new(Ssd::new()),
        ScsiAddress {
            bus: 0,
            target: 0,
            lun: 0,
        },
    );
    let runtime = builder.build().unwrap();
    println!("runtime: {:?}", runtime);

    let schema = EzkvmConfigSchema::try_from(runtime).expect("Failed to build VM schema");
    println!("ezkvm schema: {:?}", schema);

    let runtime = Runtime::try_from(schema).expect("Failed to build runtime from VM schema");
    println!("runtime: {:?}", runtime);

    let input = r#"
metadata:
  schema_version: 1.0.0
  vm_name: wakiza
host:
  resources:
  - id: storage0
    storage:
      block_device: /dev/vm1/vm-108-efidisk
  - id: storage1
    storage:
      block_device: /dev/vm1/vm-108-tpmstate
  - id: net0
    network:
      bridge: vmbr0
  - id: storage2
    storage:
      block_device: /dev/vm1/vm-108-boot
  - id: storage3
    storage:
      block_device: /dev/vm1/vm-108-tmp
virtual_machine:
  machine:
    family: pc
    q35: { version: "6.2"}
  cpu:
    model: Host
    cores: 8
    threads: 1
    sockets: 1
  memory:
    size: 17179869184
  boot:
    uefi:
      resource: storage0
  swtpm:
    version: 2.0
    resource: storage1
  devices:
  - pcie:
      bus: 0
      device: 0
      function: 0
      type: pv_scsi
  - pcie:
      bus: 0
      device: 16
      function: 0
      type: virtio_net
      resource: net0
  - scsi:
      bus: 0
      address:
        target: 0
        lun: 0
      type: hdd
      resource: storage2
  - scsi:
      bus: 0
      address:
        target: 0
        lun: 1
      type: hdd
      resource: storage3
"#;

    let schema = EzkvmConfigSchema::from_str(input).expect("Failed to parse VM schema from input");
    println!("ezkvm schema: {:?}", schema);

    let content = schema.to_styled_compact_yaml().unwrap();
    println!("styled compact yaml:\n{}", content);
}
