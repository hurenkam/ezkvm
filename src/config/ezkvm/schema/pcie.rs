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
    ScsiController {
        controller_type: ScsiControllerTypeSchema,
    },
    VirtioScsiSingle {
        resource: String,
        index: u8,
        storage_type: PcieStorageDeviceTypeSchema,
    },
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
    HostPci {
        resource: String,
        #[serde(default)]
        x_vga: bool,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScsiControllerTypeSchema {
    PvScsi,
    VirtioScsiPci,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PcieStorageDeviceTypeSchema {
    Hdd,
    Ssd,
    Cdrom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostpci_round_trips_yaml() {
        let device = PcieDeviceTypeSchema::HostPci {
            resource: "hostpci0".to_string(),
            x_vga: false,
        };
        let schema = PcieDeviceSchema::new(None, None, device);
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        assert!(yaml.contains("host_pci"), "expected type: host_pci in yaml: {yaml}");
        let decoded: PcieDeviceSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        if let PcieDeviceTypeSchema::HostPci { resource, x_vga } = decoded.device() {
            assert_eq!(resource, "hostpci0");
            assert!(!x_vga);
        } else {
            panic!("expected HostPci variant");
        }
    }

    #[test]
    fn virtio_scsi_single_round_trips_yaml() {
        let device = PcieDeviceTypeSchema::VirtioScsiSingle {
            resource: "disk0".to_string(),
            index: 3,
            storage_type: PcieStorageDeviceTypeSchema::Ssd,
        };
        let schema = PcieDeviceSchema::new(None, None, device);
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        assert!(yaml.contains("virtio_scsi_single"), "yaml was: {yaml}");
        let decoded: PcieDeviceSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        if let PcieDeviceTypeSchema::VirtioScsiSingle { resource, index, storage_type } =
            decoded.device()
        {
            assert_eq!(resource, "disk0");
            assert_eq!(index, &3);
            assert_eq!(storage_type, &PcieStorageDeviceTypeSchema::Ssd);
        } else {
            panic!("expected VirtioScsiSingle variant");
        }
    }
}
