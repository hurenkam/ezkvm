use std::{collections::HashMap, sync::Arc};

use super::{
    Cpu, I440fxChipset, IdeAddress, IdeControllerApi, IdeDeviceApi, PciAddress, PciControllerApi,
    PciDeviceApi, PcieAddress, PcieControllerApi, PcieDeviceApi, Q35Chipset, SataAddress,
    SataControllerApi, SataDeviceApi, ScsiAddress, ScsiControllerApi, ScsiDeviceApi, UsbAddress,
    UsbControllerApi, UsbDeviceApi,
};
use crate::{config_format::RuntimeConfig, runtime_model::Memory};

pub trait ControllerApi {}

pub trait BusRegistrationApi {
    fn register_pcie_bus(&self, controller: Arc<dyn PcieControllerApi>) -> Result<String, String>;
    fn register_pci_bus(&self, controller: Arc<dyn PciControllerApi>) -> Result<String, String>;
    fn register_usb_bus(&self, controller: Arc<dyn UsbControllerApi>) -> Result<String, String>;
    fn register_sata_bus(&self, controller: Arc<dyn SataControllerApi>) -> Result<String, String>;
    fn register_ide_bus(&self, controller: Arc<dyn IdeControllerApi>) -> Result<String, String>;
    fn register_scsi_bus(&self, controller: Arc<dyn ScsiControllerApi>) -> Result<String, String>;
}
pub enum Chipset {
    Q35(Q35Chipset),
    I440FX(I440fxChipset),
}

pub struct BusRegister {
    pcie_buses: HashMap<String, Arc<dyn PcieControllerApi>>,
    pci_buses: HashMap<String, Arc<dyn PciControllerApi>>,
    usb_buses: HashMap<String, Arc<dyn UsbControllerApi>>,
    sata_buses: HashMap<String, Arc<dyn SataControllerApi>>,
    ide_buses: HashMap<String, Arc<dyn IdeControllerApi>>,
    scsi_buses: HashMap<String, Arc<dyn ScsiControllerApi>>,
}
impl BusRegister {
    pub fn new() -> Self {
        Self {
            pcie_buses: HashMap::new(),
            pci_buses: HashMap::new(),
            usb_buses: HashMap::new(),
            sata_buses: HashMap::new(),
            ide_buses: HashMap::new(),
            scsi_buses: HashMap::new(),
        }
    }
}
impl BusRegistrationApi for BusRegister {
    fn register_pcie_bus(&self, _controller: Arc<dyn PcieControllerApi>) -> Result<String, String> {
        Err("Unable to register PCIe bus: not implemented".to_string())
    }
    fn register_pci_bus(&self, _controller: Arc<dyn PciControllerApi>) -> Result<String, String> {
        Err("Unable to register PCI bus: not implemented".to_string())
    }
    fn register_usb_bus(&self, _controller: Arc<dyn UsbControllerApi>) -> Result<String, String> {
        Err("Unable to register USB bus: not implemented".to_string())
    }
    fn register_sata_bus(&self, _controller: Arc<dyn SataControllerApi>) -> Result<String, String> {
        Err("Unable to register SATA bus: not implemented".to_string())
    }
    fn register_ide_bus(&self, _controller: Arc<dyn IdeControllerApi>) -> Result<String, String> {
        Err("Unable to register IDE bus: not implemented".to_string())
    }
    fn register_scsi_bus(&self, _controller: Arc<dyn ScsiControllerApi>) -> Result<String, String> {
        Err("Unable to register SCSI bus: not implemented".to_string())
    }
}
#[allow(dead_code)]
pub struct RuntimeModel {
    name: String,
    cpu: Cpu,
    memory: Memory,
    chipset: Chipset,
    busses: BusRegister,
}
impl RuntimeModel {
    pub fn register_pcie_device(
        &self,
        bus_id: String,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: PcieAddress,
    ) -> Result<(), String> {
        match self.busses.pcie_buses.get(&bus_id) {
            Some(controller) => {
                controller.register_pcie_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("PCIe bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_pci_device(
        &self,
        bus_id: String,
        device: Arc<dyn PciDeviceApi>,
        preferred_address: PciAddress,
    ) -> Result<(), String> {
        match self.busses.pci_buses.get(&bus_id) {
            Some(controller) => {
                controller.register_pci_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("PCI bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_usb_device(
        &self,
        bus_id: String,
        device: Arc<dyn UsbDeviceApi>,
        preferred_address: UsbAddress,
    ) -> Result<(), String> {
        match self.busses.usb_buses.get(&bus_id) {
            Some(controller) => {
                controller.register_usb_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("USB bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_sata_device(
        &self,
        bus_id: String,
        device: Arc<dyn SataDeviceApi>,
        preferred_address: SataAddress,
    ) -> Result<(), String> {
        match self.busses.sata_buses.get(&bus_id) {
            Some(controller) => {
                controller.register_sata_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("SATA bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_ide_device(
        &self,
        bus_id: String,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: IdeAddress,
    ) -> Result<(), String> {
        match self.busses.ide_buses.get(&bus_id) {
            Some(controller) => {
                controller.register_ide_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("IDE bus with id {} does not exist", bus_id)),
        }
    }
    pub fn register_scsi_device(
        &self,
        bus_id: String,
        device: Arc<dyn ScsiDeviceApi>,
        preferred_address: ScsiAddress,
    ) -> Result<(), String> {
        match self.busses.scsi_buses.get(&bus_id) {
            Some(controller) => {
                controller.register_scsi_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("SCSI bus with id {} does not exist", bus_id)),
        }
    }
    pub fn show(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'show' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn start(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'start' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn stop(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'stop' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn reset(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'reset' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
    pub fn shutdown(&self) -> Result<(), String> {
        println!(
            "lifecycle action 'shutdown' requested for vm '{}'; execution is not implemented yet",
            self.name
        );
        Ok(())
    }
}
impl TryFrom<RuntimeConfig> for RuntimeModel {
    type Error = String;

    fn try_from(value: RuntimeConfig) -> Result<Self, Self::Error> {
        let vm = value.virtual_machine;
        let md = value.metadata;
        let cpu = vm.cpu.unwrap_or_default();
        let chipset = match vm.machine.chipset.as_str() {
            "q35" => Chipset::Q35(Q35Chipset::new(&BusRegister::new())),
            "i440fx" => Chipset::I440FX(I440fxChipset::new(&BusRegister::new())),
            other => return Err(format!("Unsupported chipset: {}", other)),
        };
        let model = RuntimeModel {
            name: md.vm_name,
            cpu,
            memory: vm.memory,
            chipset,
            busses: BusRegister::new(),
        };

        // TODO:
        //   - uefi/bios
        //   - tpm
        //   - spice/vnc
        //   - serial ports
        //   - audio

        for device in vm.devices {
            println!("handling for device not implemented yet:\n'{:?}'", device);
        }

        Ok(model)
    }
}
