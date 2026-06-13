use std::{collections::HashMap, fmt::Display, sync::Arc};

use super::{
    Cpu, I440fxChipset, IdeAddress, IdeControllerApi, IdeDeviceApi, Memory, PciAddress,
    PciControllerApi, PciDeviceApi, PcieAddress, PcieControllerApi, PcieDeviceApi, Q35Chipset,
    SataAddress, SataControllerApi, SataDeviceApi, ScsiAddress, ScsiControllerApi, ScsiDeviceApi,
    UsbAddress, UsbControllerApi, UsbDeviceApi,
};
use crate::{config_format::RuntimeConfig, runtime_config::Device};

pub trait ControllerApi {}

pub trait BusRegistrationApi {
    fn register_pcie_bus(
        &mut self,
        controller: Arc<dyn PcieControllerApi>,
    ) -> Result<String, String>;
    fn register_pci_bus(&mut self, controller: Arc<dyn PciControllerApi>)
    -> Result<String, String>;
    fn register_usb_bus(&mut self, controller: Arc<dyn UsbControllerApi>)
    -> Result<String, String>;
    fn register_sata_bus(
        &mut self,
        controller: Arc<dyn SataControllerApi>,
    ) -> Result<String, String>;
    fn register_ide_bus(&mut self, controller: Arc<dyn IdeControllerApi>)
    -> Result<String, String>;
    fn register_scsi_bus(
        &mut self,
        controller: Arc<dyn ScsiControllerApi>,
    ) -> Result<String, String>;
}
pub enum Chipset {
    Q35(Q35Chipset),
    I440FX(I440fxChipset),
}

pub struct BusRegister {
    pcie_busses: HashMap<String, Arc<dyn PcieControllerApi>>,
    pci_busses: HashMap<String, Arc<dyn PciControllerApi>>,
    usb_busses: HashMap<String, Arc<dyn UsbControllerApi>>,
    sata_busses: HashMap<String, Arc<dyn SataControllerApi>>,
    ide_busses: HashMap<String, Arc<dyn IdeControllerApi>>,
    scsi_busses: HashMap<String, Arc<dyn ScsiControllerApi>>,
}
impl BusRegister {
    pub fn new() -> Self {
        Self {
            pcie_busses: HashMap::new(),
            pci_busses: HashMap::new(),
            usb_busses: HashMap::new(),
            sata_busses: HashMap::new(),
            ide_busses: HashMap::new(),
            scsi_busses: HashMap::new(),
        }
    }
}
impl Display for BusRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n")?;
        write!(f, "    PCIe: \n")?;
        for (id, value) in &self.pcie_busses {
            write!(f, "      {id}: \n{value}")?;
        }
        write!(f, "    PCI: \n")?;
        for (id, value) in &self.pci_busses {
            write!(f, "      {id}: \n{value}")?;
        }
        write!(f, "    USB: \n")?;
        for (id, value) in &self.usb_busses {
            write!(f, "      {id}: \n{value}")?;
        }
        write!(f, "    SATA: \n")?;
        for (id, value) in &self.sata_busses {
            write!(f, "      {id}: \n{value}")?;
        }
        write!(f, "    IDE: \n")?;
        for (id, value) in &self.ide_busses {
            write!(f, "      {id}: \n{value}")?;
        }
        write!(f, "    SCSI: \n")?;
        for (id, value) in &self.scsi_busses {
            write!(f, "      {id}: \n{value}")?;
        }
        Ok(())
    }
}
impl BusRegistrationApi for BusRegister {
    fn register_pcie_bus(
        &mut self,
        controller: Arc<dyn PcieControllerApi>,
    ) -> Result<String, String> {
        let bus_id = format!("pcie.{}", self.pcie_busses.len());
        self.pcie_busses.insert(bus_id.clone(), controller);
        Ok(bus_id)
    }
    fn register_pci_bus(
        &mut self,
        controller: Arc<dyn PciControllerApi>,
    ) -> Result<String, String> {
        let bus_id = format!("pci.{}", self.pci_busses.len());
        self.pci_busses.insert(bus_id.clone(), controller);
        Ok(bus_id)
    }
    fn register_usb_bus(
        &mut self,
        controller: Arc<dyn UsbControllerApi>,
    ) -> Result<String, String> {
        let bus_id = format!("usb.{}", self.usb_busses.len());
        self.usb_busses.insert(bus_id.clone(), controller);
        Ok(bus_id)
    }
    fn register_sata_bus(
        &mut self,
        controller: Arc<dyn SataControllerApi>,
    ) -> Result<String, String> {
        let bus_id = format!("sata.{}", self.sata_busses.len());
        self.sata_busses.insert(bus_id.clone(), controller);
        Ok(bus_id)
    }
    fn register_ide_bus(
        &mut self,
        controller: Arc<dyn IdeControllerApi>,
    ) -> Result<String, String> {
        let bus_id = format!("ide.{}", self.ide_busses.len());
        self.ide_busses.insert(bus_id.clone(), controller);
        Ok(bus_id)
    }
    fn register_scsi_bus(
        &mut self,
        controller: Arc<dyn ScsiControllerApi>,
    ) -> Result<String, String> {
        let bus_id = format!("scsi.{}", self.scsi_busses.len());
        self.scsi_busses.insert(bus_id.clone(), controller);
        Ok(bus_id)
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
    pub fn get_pcie_bus(&self, id: String) -> Arc<dyn PcieControllerApi> {
        self.busses
            .pcie_busses
            .get(&id)
            .expect(&format!("PCIe bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_pci_bus(&self, id: String) -> Arc<dyn PciControllerApi> {
        self.busses
            .pci_busses
            .get(&id)
            .expect(&format!("PCI bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_usb_bus(&self, id: String) -> Arc<dyn UsbControllerApi> {
        self.busses
            .usb_busses
            .get(&id)
            .expect(&format!("USB bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_sata_bus(&self, id: String) -> Arc<dyn SataControllerApi> {
        self.busses
            .sata_busses
            .get(&id)
            .expect(&format!("SATA bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_ide_bus(&self, id: String) -> Arc<dyn IdeControllerApi> {
        self.busses
            .ide_busses
            .get(&id)
            .expect(&format!("IDE bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_scsi_bus(&self, id: String) -> Arc<dyn ScsiControllerApi> {
        self.busses
            .scsi_busses
            .get(&id)
            .expect(&format!("SCSI bus with id {} does not exist", id))
            .clone()
    }
    pub fn register_pcie_device(
        &self,
        bus_id: String,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String> {
        match self.busses.pcie_busses.get(&bus_id) {
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
        preferred_address: Option<PciAddress>,
    ) -> Result<(), String> {
        match self.busses.pci_busses.get(&bus_id) {
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
        preferred_address: Option<UsbAddress>,
    ) -> Result<(), String> {
        match self.busses.usb_busses.get(&bus_id) {
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
        preferred_address: Option<SataAddress>,
    ) -> Result<(), String> {
        match self.busses.sata_busses.get(&bus_id) {
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
        preferred_address: Option<IdeAddress>,
    ) -> Result<(), String> {
        match self.busses.ide_busses.get(&bus_id) {
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
        preferred_address: Option<ScsiAddress>,
    ) -> Result<(), String> {
        match self.busses.scsi_busses.get(&bus_id) {
            Some(controller) => {
                controller.register_scsi_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("SCSI bus with id {} does not exist", bus_id)),
        }
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
        let mut register = BusRegister::new();
        let chipset = match vm.machine.chipset.as_str() {
            "q35" => Chipset::Q35(Q35Chipset::new(&mut register)),
            "i440fx" => Chipset::I440FX(I440fxChipset::new(&BusRegister::new())),
            other => return Err(format!("Unsupported chipset: {}", other)),
        };
        let model = RuntimeModel {
            name: md.vm_name,
            cpu,
            memory: vm.memory,
            chipset,
            busses: register,
        };

        // TODO:
        //   - uefi/bios
        //   - tpm
        //   - spice/vnc/gpu
        //   - serial ports
        //   - audio
        //   - qmp/guest agent

        for device in vm.devices {
            match device {
                Device::Pcie {
                    pcie,
                    address,
                    device,
                } => {
                    model.register_pcie_device(
                        pcie.unwrap_or("pcie.0".to_string()),
                        device.into(),
                        address,
                    )?;
                }
                Device::Pci {
                    pci,
                    address,
                    device,
                } => {
                    model.register_pci_device(
                        pci.unwrap_or("pci.0".to_string()),
                        device.into(),
                        address,
                    )?;
                }
                Device::Usb { usb, port, device } => {
                    model.register_usb_device(
                        usb.unwrap_or("usb.0".to_string()),
                        device.into(),
                        port,
                    )?;
                }
                Device::Ide { ide, port, device } => {
                    model.register_ide_device(
                        ide.unwrap_or("ide.0".to_string()),
                        device.into(),
                        port,
                    )?;
                }
                Device::Sata {
                    sata,
                    address,
                    device,
                } => {
                    model.register_sata_device(
                        sata.unwrap_or("sata.0".to_string()),
                        device.into(),
                        address,
                    )?;
                }
                Device::Scsi {
                    scsi,
                    address,
                    device,
                } => {
                    model.register_scsi_device(
                        scsi.unwrap_or("scsi.0".to_string()),
                        device.into(),
                        address,
                    )?;
                }
            }
        }

        Ok(model)
    }
}

impl Display for RuntimeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RuntimeModel for VM '{}':", self.name)?;
        writeln!(f, "  CPU: {:?}", self.cpu)?;
        writeln!(f, "  Memory: {:?}", self.memory)?;
        writeln!(
            f,
            "  Chipset: {}",
            match &self.chipset {
                Chipset::Q35(_) => "Q35",
                Chipset::I440FX(_) => "I440FX",
            }
        )?;
        writeln!(f, "  Busses: {}", self.busses)?;
        Ok(())
    }
}
