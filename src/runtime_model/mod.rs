mod boot;
mod bus_register;
mod cpu;
mod devices;
mod i440fx;
mod ide;
mod memory;
mod model;
mod pci;
mod pcie;
mod q35;
mod resources;
mod sata;
mod scsi;
mod tpm;
mod usb;

pub use model::RuntimeModel;
pub use resources::{
    NetworkResource, PciDeviceResource, PcieDeviceResource, Resource, StorageResource,
    UsbDeviceResource,
};

pub use boot::{BiosModel, BootModel, SeaBiosModel, UefiModel};
pub use bus_register::{BusRegister, BusRegistrationApi};
pub use cpu::{Cpu, CpuModel};
pub use i440fx::I440fxChipset;
pub use ide::{
    IdeAddress, IdeBus, IdeControllerApi, IdeDevice, IdeDeviceApi, IdeDeviceBuilder, IdeDeviceType,
};
pub use memory::Memory;
pub use model::ControllerApi;
pub use pci::{PciAddress, PciBus, PciControllerApi, PciDevice, PciDeviceApi, PciDeviceType};
pub use pcie::{
    PcieAddress, PcieBus, PcieControllerApi, PcieDevice, PcieDeviceApi, PcieDeviceType,
};
pub use q35::Q35Chipset;
pub use sata::{
    SataAddress, SataBus, SataControllerApi, SataDevice, SataDeviceApi, SataDeviceBuilder,
    SataDeviceType,
};
pub use scsi::{
    ScsiAddress, ScsiBus, ScsiControllerApi, ScsiDevice, ScsiDeviceApi, ScsiDeviceBuilder,
    ScsiDeviceType, ScsiDisk,
};
pub use tpm::{Tpm, TpmApi, TpmModel, TpmModelBuilder};
pub use usb::{
    UsbAddress, UsbBus, UsbControllerApi, UsbDevice, UsbDeviceApi, UsbDeviceBuilder, UsbDeviceType,
};

pub use devices::{
    Cdrom,
    // Storage devices:
    Hdd,
    // PCI & PCIe devices:
    PvScsiController,
    Ssd,
    VirtioNetController,
};

pub struct RuntimeModelBuilder {}
pub struct RuntimeModelRenderer {}

pub enum Chipset {
    Q35(Q35Chipset),
    I440FX(I440fxChipset),
}
impl Chipset {
    pub fn qemu_args(&self) -> Vec<String> {
        match self {
            Chipset::Q35(q35) => q35.qemu_args(),
            Chipset::I440FX(i440fx) => i440fx.qemu_args(),
        }
    }
}
