mod audio;
mod boot;
mod chipset;
mod config;
mod cpu;
mod device;
mod display;
mod guest_agent;
mod host;
mod ide;
mod machine;
mod memory;
mod meta;
mod pci;
mod pcie;
mod resources;
mod sata;
mod scsi;
mod tpm;
mod usb;
mod virtual_machine;

#[allow(unused_imports)]
pub use {
    audio::*, boot::*, chipset::*, config::*, cpu::*, device::*, display::*, guest_agent::*,
    host::*, ide::*, machine::*, memory::*, meta::*, pci::*, pcie::*, resources::*, sata::*,
    scsi::*, tpm::*, usb::*, virtual_machine::*,
};

/// Schema version for ezkvm runtime config specification.
pub const EZKVM_CONFIG_SCHEMA_VERSION: &str = "1.0.0";
