use std::sync::Arc;

use super::{
    BusRegistrationApi, ControllerApi, PcieAddress, PcieControllerApi, PcieDeviceApi, SataAddress,
    SataControllerApi, SataDeviceApi, UsbAddress, UsbControllerApi, UsbDeviceApi,
};

pub struct Q35Chipset {}
impl Q35Chipset {
    pub fn new(api: &dyn BusRegistrationApi) -> Self {
        let pcie_bus = Arc::new(Q35RootPortController::default());
        api.register_pcie_bus(pcie_bus.clone())
            .expect("Failed to register PCIe bus");
        let sata_bus = Arc::new(Q35SataController::default());
        api.register_sata_bus(sata_bus.clone())
            .expect("Failed to register SATA bus");
        let usb_bus = Arc::new(Q35UsbController::default());
        api.register_usb_bus(usb_bus.clone())
            .expect("Failed to register USB bus");
        Self {}
    }
    pub fn qemu_args(&self) -> Vec<String> {
        todo!()
    }
}
#[derive(Debug, Clone, Default)]
pub struct Q35RootPortController {}
impl ControllerApi for Q35RootPortController {}
impl PcieControllerApi for Q35RootPortController {
    fn register_pcie_device(
        &self,
        _device: Arc<dyn PcieDeviceApi>,
        _preferred_address: Option<PcieAddress>,
    ) -> Result<(), String> {
        Err("Unable to register PCIe device: not implemented".to_string())
    }
}
#[derive(Debug, Clone, Default)]
pub struct Q35SataController {}
impl ControllerApi for Q35SataController {}
impl SataControllerApi for Q35SataController {
    fn register_sata_device(
        &self,
        _device: Arc<dyn SataDeviceApi>,
        _preferred_address: Option<SataAddress>,
    ) -> Result<(), String> {
        Err("Unable to register SATA device: not implemented".to_string())
    }
}
#[derive(Debug, Clone, Default)]
pub struct Q35UsbController {}
impl ControllerApi for Q35UsbController {}
impl UsbControllerApi for Q35UsbController {
    fn register_usb_device(
        &self,
        _device: Arc<dyn UsbDeviceApi>,
        _preferred_address: Option<UsbAddress>,
    ) -> Result<(), String> {
        Err("Unable to register USB device: not implemented".to_string())
    }
}
