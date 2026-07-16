use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

pub type PcieBusSchema = u8;
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, new)]
pub struct PcieAddressSchema {
    pub device: u8,
    pub function: u8,
}
impl std::fmt::Display for PcieAddressSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct PcieDeviceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PcieBusSchema>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    address: Option<PcieAddressSchema>,
    #[serde(flatten)]
    device: PcieDeviceTypeSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PcieDeviceTypeSchema {
    PvScsi,
    VirtioNet {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mac_address: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rx_queue_size: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tx_queue_size: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vhost: Option<bool>,
    },
    StandardGpu,
    VirtioGpu,
    Passthrough {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        multifunction: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rombar: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        romfile: Option<String>,
    },
    PassthroughGpu {
        resource: String,
    },
    Ich9IntelHda {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        codec: Option<String>,
    },
    IvshmemPlain {
        resource: String,
    },
}
