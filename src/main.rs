use std::sync::Arc;

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
}
