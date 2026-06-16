use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

use super::{
    BusRegistrationApi, ControllerApi, PcieAddress, PcieControllerApi, PcieDeviceApi, SataAddress,
    SataControllerApi, SataDeviceApi, UsbAddress, UsbControllerApi, UsbDeviceApi,
};

pub struct Q35Chipset {}
impl Q35Chipset {
    pub fn new(api: &mut dyn BusRegistrationApi) -> Self {
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
#[derive(Default)]
pub struct Q35RootPortController {
    pcie_devices: Mutex<HashMap<PcieAddress, Arc<dyn PcieDeviceApi>>>,
}
impl ControllerApi for Q35RootPortController {}
impl PcieControllerApi for Q35RootPortController {
    fn register_pcie_device(
        &self,
        device: Arc<dyn PcieDeviceApi>,
        preferred: Option<PcieAddress>,
    ) -> Result<(), String> {
        let address = self.select_pcie_address(preferred);
        let mut devices = self.pcie_devices.lock().unwrap();
        devices.insert(address, device);
        Ok(())
    }
    
    fn devices(&self) -> HashMap<PcieAddress, Arc<dyn PcieDeviceApi>> {
        let devices = self.pcie_devices.lock().unwrap();
        devices.clone()
    }
}
impl Q35RootPortController {
    fn select_pcie_address(&self, preferred: Option<PcieAddress>) -> PcieAddress {
        let devices = self.pcie_devices.lock().unwrap();
        if let Some(addr) = preferred {
            if !devices.contains_key(&addr) {
                return addr;
            }
        }
        // Simple allocation strategy: find the first available address
        for device_num in 0..32 {
            for function_num in 0..8 {
                let addr = PcieAddress::new(device_num, function_num);
                if !devices.contains_key(&addr) {
                    return addr;
                }
            }
        }
        panic!("No available PCIe addresses");
    }
}
impl Display for Q35RootPortController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //write!(f, "        Q35 Root Port Controller\n");
        let devices = self.pcie_devices.lock().unwrap();
        for (addr, device) in devices.iter() {
            write!(f, "          {addr}: {device}\n")?;
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct Q35SataController {
    sata_devices: Mutex<HashMap<SataAddress, Arc<dyn SataDeviceApi>>>,
}
impl ControllerApi for Q35SataController {}
impl SataControllerApi for Q35SataController {
    fn register_sata_device(
        &self,
        device: Arc<dyn SataDeviceApi>,
        preferred: Option<SataAddress>,
    ) -> Result<(), String> {
        let address = self.select_sata_address(preferred);
        let mut devices = self.sata_devices.lock().unwrap();
        devices.insert(address, device);
        Ok(())
    }
    
    fn devices(&self) -> HashMap<SataAddress, Arc<dyn SataDeviceApi>> {
        let devices = self.sata_devices.lock().unwrap();
        devices.clone()
    }
}
impl Display for Q35SataController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let devices = self.sata_devices.lock().unwrap();
        for (addr, device) in devices.iter() {
            write!(f, "          {addr}: {device}\n")?;
        }
        Ok(())
    }
}
impl Q35SataController {
    fn select_sata_address(&self, preferred: Option<SataAddress>) -> SataAddress {
        let devices = self.sata_devices.lock().unwrap();
        if let Some(addr) = preferred {
            if !devices.contains_key(&addr) {
                return addr;
            }
        }
        // Simple allocation strategy: find the first available address
        for port in 0..4 {
            let addr = SataAddress::new(port);
            if !devices.contains_key(&addr) {
                return addr;
            }
        }
        panic!("No available SATA addresses");
    }
}
#[derive(Default)]
pub struct Q35UsbController {
    usb_devices: Mutex<HashMap<UsbAddress, Arc<dyn UsbDeviceApi>>>,
}
impl ControllerApi for Q35UsbController {}
impl UsbControllerApi for Q35UsbController {
    fn register_usb_device(
        &self,
        device: Arc<dyn UsbDeviceApi>,
        preferred: Option<UsbAddress>,
    ) -> Result<(), String> {
        let address = self.select_usb_address(preferred);
        let mut devices = self.usb_devices.lock().unwrap();
        devices.insert(address, device);
        Ok(())
    }
    fn devices(&self) -> HashMap<UsbAddress, Arc<dyn UsbDeviceApi>> {
        let devices = self.usb_devices.lock().unwrap();
        devices.clone()
    }
}
impl Display for Q35UsbController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let devices = self.usb_devices.lock().unwrap();
        for (addr, device) in devices.iter() {
            write!(f, "          {addr}: {device}\n")?;
        }
        Ok(())
    }
}
impl Q35UsbController {
    fn select_usb_address(&self, preferred: Option<UsbAddress>) -> UsbAddress {
        let devices = self.usb_devices.lock().unwrap();
        if let Some(addr) = preferred {
            if !devices.contains_key(&addr) {
                return addr;
            }
        }
        // Simple allocation strategy: find the first available port
        for port_num in 1..=4 {
            let port = format!("usb{port_num}");
            let addr = UsbAddress::new(port);
            if !devices.contains_key(&addr) {
                return addr;
            }
        }
        panic!("No available USB addresses");
    }
}
