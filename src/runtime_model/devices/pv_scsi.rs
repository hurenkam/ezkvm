use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

use crate::runtime_model::{
    ControllerApi, PcieAddress, PcieBus, PcieDeviceApi, PcieDeviceKind, ScsiAddress,
    ScsiControllerApi, ScsiDeviceApi,
};

#[derive(Default)]
pub struct PvScsiController {
    scsi_devices: Mutex<HashMap<ScsiAddress, Arc<dyn ScsiDeviceApi>>>,
}

impl PcieDeviceApi for PvScsiController {
    fn device_kind(&self) -> PcieDeviceKind {
        PcieDeviceKind::PvScsi
    }

    fn qemu_args(&self, bus: &PcieBus, address: PcieAddress) -> Vec<String> {
        let mut args = vec![
            "-device".to_string(),
            format!(
                "pvscsi,id=scsihw0,bus=pcie.{bus},addr=0x{:x}.{:x}",
                address.device(),
                address.function()
            ),
        ];
        let devices = self.scsi_devices.lock().unwrap();
        let mut addresses: Vec<_> = devices.keys().cloned().collect();
        addresses.sort_by_key(|scsi_address| (scsi_address.target, scsi_address.lun));
        for scsi_address in addresses {
            if let Some(device) = devices.get(&scsi_address) {
                args.extend(device.qemu_args(&0, scsi_address.clone()));
            }
        }
        args
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
}
impl ControllerApi for PvScsiController {
    fn qemu_args(&self) -> Vec<String> {
        Vec::new()
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
