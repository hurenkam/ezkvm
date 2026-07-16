use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::ide::IdeDeviceSchema;
use crate::config::ezkvm::schema::pci::PciDeviceSchema;
use crate::config::ezkvm::schema::pcie::PcieDeviceSchema;
use crate::config::ezkvm::schema::sata::SataDeviceSchema;
use crate::config::ezkvm::schema::scsi::ScsiDeviceSchema;
use crate::config::ezkvm::schema::usb::UsbDeviceSchema;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DeviceSchema {
    Pcie { pcie: PcieDeviceSchema },
    Pci { pci: PciDeviceSchema },
    Usb { usb: UsbDeviceSchema },
    Sata { sata: SataDeviceSchema },
    Ide { ide: IdeDeviceSchema },
    Scsi { scsi: ScsiDeviceSchema },
}
