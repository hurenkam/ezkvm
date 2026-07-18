use std::{str::FromStr, sync::Arc};

use crate::{
    config::EzkvmConfigSchema,
    runtime::{
        PcieAddress, PvScsiBuilder, Q35ChipsetBuilder, Runtime, SataAddress, ScsiAddress, Ssd,
    },
};

mod config;
mod runtime;
mod serde_yaml;

fn main() {
    let runtime = runtime::RuntimeBuilder::new()
        .with_memory(runtime::Memory::new(1024))
        .with_chipset(runtime::Chipset::Q35(
            Q35ChipsetBuilder::new()
                .with_sata_device(Some(SataAddress::new(0, 0)), Arc::new(Ssd::new()))
                .with_pcie_device(
                    Some(PcieAddress::new(0, 0)),
                    Arc::new(
                        PvScsiBuilder::new()
                            .with_scsi_device(Some(ScsiAddress::new(0, 0)), Arc::new(Ssd::new()))
                            .build(),
                    ),
                )
                .build(),
        ))
        .build()
        .expect("build runtime failed");

    println!("runtime: {:?}\n\n\n", runtime);

    let schema = EzkvmConfigSchema::try_from(runtime).expect("Failed to build VM schema");
    println!("ezkvm schema: {:?}\n\n\n", schema);

    let runtime = Runtime::try_from(schema).expect("Failed to build runtime from VM schema");
    println!("runtime: {:?}\n\n\n", runtime);

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
    println!("ezkvm schema: {:?}\n\n\n", schema);

    let content = schema.to_styled_compact_yaml().unwrap();
    println!("styled compact yaml:\n{:?}", content);
}
