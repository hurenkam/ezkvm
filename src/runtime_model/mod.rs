mod audio;
mod balloon;
mod boot;
mod bus_register;
mod cpu;
mod devices;
mod display;
mod gpu;
mod guest_agent;
mod i440fx;
mod ide;
mod lifecycle;
mod memory;
mod model;
mod pci;
mod pcie;
mod q35;
mod qmp;
mod resources;
mod sata;
mod scsi;
mod serial;
mod tpm;
mod usb;

pub use audio::{Audio, AudioApi, AudioBackend, AudioController, AudioModelBuilder};
pub use balloon::BalloonConfig;
#[allow(unused_imports)] // TODO: wire to CLI
pub use display::{
    Display, DisplayApi, DisplayModelBuilder, EglHeadless, Gtk, LookingGlass, Sdl, Spice, Vnc,
};
#[allow(unused_imports)] // TODO: wire to CLI
pub use gpu::{Gpu, GpuApi, GpuModelBuilder};
pub use guest_agent::{GuestAgent, GuestAgentApi, GuestAgentModelBuilder};
pub use lifecycle::LifecycleConfig;
pub use model::{PowerManagementConfig, RuntimeModel};
pub use resources::{
    NetworkResource, PciDeviceResource, PcieDeviceResource, Resource, StorageResource,
    UsbDeviceResource,
};
pub use serial::SerialConfig;

pub use boot::{BiosModel, BootModel, SeaBiosModel, UefiModel};
pub use bus_register::{BusRegister, BusRegistrationApi};
pub use cpu::{Cpu, CpuModel};
pub use i440fx::I440fxChipset;
pub use ide::{
    IdeAddress, IdeBus, IdeControllerApi, IdeDevice, IdeDeviceApi, IdeDeviceBuilder, IdeDeviceType,
};
pub use memory::Memory;
pub use pci::{
    Ac97Controller, PciAddress, PciBus, PciControllerApi, PciDevice, PciDeviceApi, PciDeviceType,
    QxlGpuController,
};
#[allow(unused_imports)] // TODO: wire to CLI
pub use pcie::{
    Ich9IntelHdaController, IvshmemPlainController, PassthroughGpuController,
    PassthroughPcieController, PcieAddress, PcieBus, PcieControllerApi, PcieDevice, PcieDeviceApi,
    PcieDeviceType, StandardGpuController, VirtioGpuController,
};
#[allow(unused_imports)]
// Used by renderer tests and external callers when USB bus rendering is enabled.
pub use q35::{Q35Chipset, Q35UsbController};
pub use sata::{
    SataAddress, SataBus, SataControllerApi, SataDevice, SataDeviceApi, SataDeviceBuilder,
    SataDeviceType,
};
#[allow(unused_imports)] // TODO: wire to CLI
pub use scsi::{
    ScsiAddress, ScsiBus, ScsiControllerApi, ScsiDevice, ScsiDeviceApi, ScsiDeviceBuilder,
    ScsiDeviceType, ScsiDisk,
};
#[allow(unused_imports)] // TODO: wire to CLI
pub use tpm::{Swtpm, Tpm, TpmApi, TpmModel, TpmModelBuilder};
#[allow(unused_imports)] // TODO: wire to CLI
pub use usb::{
    UsbAddress, UsbBus, UsbControllerApi, UsbDevice, UsbDeviceApi, UsbDeviceBuilder, UsbDeviceType,
    UsbHostByBusPortController, UsbHostByIdController, UsbTabletController,
};

pub use devices::{
    Cdrom,
    // Storage devices:
    Hdd,
    // PCI & PCIe devices:
    PvScsiController,
    Ssd,
    StorageCachePolicy,
    StorageDeviceKind,
    StorageDeviceOptions,
    VirtioNetController,
};

pub enum Chipset {
    Q35(Q35Chipset),
    I440FX(I440fxChipset),
}
