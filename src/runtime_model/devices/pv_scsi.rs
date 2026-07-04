use std::{
    any::Any,
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

use crate::runtime_model::{
    PcieAddress, PcieDeviceApi, PcieDeviceType, ScsiAddress, ScsiControllerApi, ScsiDeviceApi,
};

#[derive(Default)]
pub struct PvScsiController {
    scsi_devices: Mutex<HashMap<ScsiAddress, Arc<dyn ScsiDeviceApi>>>,
}

impl PcieDeviceApi for PvScsiController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::PvScsi
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
}
impl ScsiControllerApi for PvScsiController {
    fn register_scsi_device(
        &self,
        device: Arc<dyn ScsiDeviceApi>,
        preferred_address: Option<ScsiAddress>,
    ) -> Result<(), String> {
        let address = self.select_scsi_address(preferred_address);
        let mut devices = self.scsi_devices.lock().unwrap();
        devices.insert(address, device);
        Ok(())
    }

    fn devices(&self) -> HashMap<ScsiAddress, Arc<dyn ScsiDeviceApi>> {
        let devices = self.scsi_devices.lock().unwrap();
        devices.clone()
    }
}
impl Display for PvScsiController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PV SCSI Controller")
    }
}

impl PvScsiController {
    fn select_scsi_address(&self, preferred: Option<ScsiAddress>) -> ScsiAddress {
        let devices = self.scsi_devices.lock().unwrap();
        if let Some(address) = preferred
            && !devices.contains_key(&address)
        {
            return address;
        }

        for target in 0..16 {
            for lun in 0..16 {
                let address = ScsiAddress::new(target, lun);
                if !devices.contains_key(&address) {
                    return address;
                }
            }
        }

        panic!("No available SCSI addresses");
    }
}
