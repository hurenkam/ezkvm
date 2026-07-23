use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub type PciBusSchema = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PciAddressSchema {
    pub device: u8,
    pub function: u8,
}
impl Display for PciAddressSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct PciDeviceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PciBusSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<PciAddressSchema>,
    device: PciDeviceTypeSchema,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PciDeviceTypeSchema {
    NetworkController,
    QxlGpu,
    Ac97,
    HostPci {
        resource: String,
        #[serde(default)]
        x_vga: bool,
    },
}
