use derive_getters::Getters;
use serde::{Deserialize, Serialize};

use crate::runtime_model::{
    Audio, Cpu, Display, GuestAgent, IdeDevice, Memory, PciDevice, PcieDevice, SataDevice,
    ScsiDevice, Tpm, UsbDevice,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub machine: Machine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Cpu>,
    pub memory: Memory,
    #[serde(default)]
    pub boot: Boot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smbios_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vmgenid: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tpm: Option<Tpm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Display>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<Audio>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_agent: Option<GuestAgent>,
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Device {
    Pcie { pcie: PcieDevice },
    Pci { pci: PciDevice },
    Usb { usb: UsbDevice },
    Sata { sata: SataDevice },
    Ide { ide: IdeDevice },
    Scsi { scsi: ScsiDevice },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Getters)]
pub struct Boot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    secure: Option<bool>,
    #[serde(flatten)]
    bios: Bios,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bios {
    SeaBios { seabios: SeaBios },
    Uefi { uefi: Uefi },
}
impl Default for Bios {
    fn default() -> Self {
        Bios::SeaBios {
            seabios: SeaBios::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SeaBios {
    firmware: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct Uefi {
    resource: String,
}
