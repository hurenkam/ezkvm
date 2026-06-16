use std::{collections::HashMap, fmt::Display, sync::Arc};

use super::{
    Cpu, I440fxChipset, IdeAddress, IdeBus, IdeControllerApi, IdeDeviceApi, Memory, PciAddress,
    PciBus, PciControllerApi, PciDeviceApi, PcieAddress, PcieBus, PcieControllerApi, PcieDeviceApi,
    Q35Chipset, SataAddress, SataBus, SataControllerApi, SataDeviceApi, ScsiAddress, ScsiBus,
    ScsiControllerApi, ScsiDeviceApi, UsbAddress, UsbBus, UsbControllerApi, UsbDeviceApi,
};
use crate::{
    config_format::RuntimeConfig,
    runtime_config::{Device, NetworkResource, Resource, StorageResource, UsbDeviceResource},
    runtime_model::{
        BootModel, PcieDeviceType, UsbDeviceBuilder, boot::BootModelBuilder, ide::IdeDeviceBuilder, sata::SataDeviceBuilder, scsi::ScsiDeviceBuilder
    },
};

pub trait ControllerApi {}

pub trait BusRegistrationApi {
    fn register_pcie_bus(&mut self, controller: Arc<dyn PcieControllerApi>) -> Result<u8, String>;
    fn register_pci_bus(&mut self, controller: Arc<dyn PciControllerApi>) -> Result<u8, String>;
    fn register_usb_bus(&mut self, controller: Arc<dyn UsbControllerApi>) -> Result<u8, String>;
    fn register_sata_bus(&mut self, controller: Arc<dyn SataControllerApi>) -> Result<u8, String>;
    fn register_ide_bus(&mut self, controller: Arc<dyn IdeControllerApi>) -> Result<u8, String>;
    fn register_scsi_bus(&mut self, controller: Arc<dyn ScsiControllerApi>) -> Result<u8, String>;
}
pub enum Chipset {
    Q35(Q35Chipset),
    I440FX(I440fxChipset),
}

pub struct BusRegister {
    pcie_busses: HashMap<PcieBus, Arc<dyn PcieControllerApi>>,
    pci_busses: HashMap<PciBus, Arc<dyn PciControllerApi>>,
    usb_busses: HashMap<UsbBus, Arc<dyn UsbControllerApi>>,
    sata_busses: HashMap<SataBus, Arc<dyn SataControllerApi>>,
    ide_busses: HashMap<IdeBus, Arc<dyn IdeControllerApi>>,
    scsi_busses: HashMap<ScsiBus, Arc<dyn ScsiControllerApi>>,
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
        for (id, value) in &self.pcie_busses {
            for (address, device) in value.devices() {
                write!(f, "    pcie bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    pcie bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.pci_busses {
            for (address, device) in value.devices() {
                write!(f, "    pci  bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    pci  bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.usb_busses {
            for (address, device) in value.devices() {
                write!(f, "    usb  bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    usb  bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.sata_busses {
            for (address, device) in value.devices() {
                write!(f, "    sata bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    sata bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.ide_busses {
            for (address, device) in value.devices() {
                write!(f, "    ide  bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    ide  bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.scsi_busses {
            for (address, device) in value.devices() {
                write!(f, "    scsi bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    scsi bus {id}, (no devices)\n")?;
            }
        }
        Ok(())
    }
}
impl BusRegistrationApi for BusRegister {
    fn register_pcie_bus(&mut self, controller: Arc<dyn PcieControllerApi>) -> Result<u8, String> {
        let bus_id = self.pcie_busses.len() as u8;
        self.pcie_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_pci_bus(&mut self, controller: Arc<dyn PciControllerApi>) -> Result<u8, String> {
        let bus_id = self.pci_busses.len() as u8;
        self.pci_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_usb_bus(&mut self, controller: Arc<dyn UsbControllerApi>) -> Result<u8, String> {
        let bus_id = self.usb_busses.len() as u8;
        self.usb_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_sata_bus(&mut self, controller: Arc<dyn SataControllerApi>) -> Result<u8, String> {
        let bus_id = self.sata_busses.len() as u8;
        self.sata_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_ide_bus(&mut self, controller: Arc<dyn IdeControllerApi>) -> Result<u8, String> {
        let bus_id = self.ide_busses.len() as u8;
        self.ide_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_scsi_bus(&mut self, controller: Arc<dyn ScsiControllerApi>) -> Result<u8, String> {
        let bus_id = self.scsi_busses.len() as u8;
        self.scsi_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
}
#[allow(dead_code)]
pub struct RuntimeModel {
    name: String,
    cpu: Cpu,
    memory: Memory,
    chipset: Chipset,
    boot: BootModel,
    busses: BusRegister,
    //resources: Vec<Resource>,
}
impl RuntimeModel {
    pub fn get_pcie_bus(&self, id: PcieBus) -> Arc<dyn PcieControllerApi> {
        self.busses
            .pcie_busses
            .get(&id)
            .expect(&format!("PCIe bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_pci_bus(&self, id: PciBus) -> Arc<dyn PciControllerApi> {
        self.busses
            .pci_busses
            .get(&id)
            .expect(&format!("PCI bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_usb_bus(&self, id: UsbBus) -> Arc<dyn UsbControllerApi> {
        self.busses
            .usb_busses
            .get(&id)
            .expect(&format!("USB bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_sata_bus(&self, id: SataBus) -> Arc<dyn SataControllerApi> {
        self.busses
            .sata_busses
            .get(&id)
            .expect(&format!("SATA bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_ide_bus(&self, id: IdeBus) -> Arc<dyn IdeControllerApi> {
        self.busses
            .ide_busses
            .get(&id)
            .expect(&format!("IDE bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_scsi_bus(&self, id: ScsiBus) -> Arc<dyn ScsiControllerApi> {
        self.busses
            .scsi_busses
            .get(&id)
            .expect(&format!("SCSI bus with id {} does not exist", id))
            .clone()
    }
    pub fn register_pcie_device(
        &self,
        bus_id: PcieBus,
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
        bus_id: PciBus,
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
        bus_id: UsbBus,
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
        bus_id: SataBus,
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
        bus_id: IdeBus,
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
        bus_id: ScsiBus,
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
        let RuntimeConfig {
            metadata: md,
            virtual_machine: vm,
            resources,
        } = value;

        let mut storage_resources: HashMap<String, StorageResource> = HashMap::new();
        let mut network_resources: HashMap<String, NetworkResource> = HashMap::new();
        let mut usb_resources: HashMap<String, UsbDeviceResource> = HashMap::new();
        for resource in resources {
            match resource {
                Resource::Storage { id, storage } => {
                    storage_resources.insert(id, storage);
                }
                Resource::Network { id, network } => {
                    network_resources.insert(id, network);
                }
                Resource::UsbDevice { id, usb_device } => {
                    usb_resources.insert(id, usb_device);
                }
                Resource::PciDevice {
                    id: _,
                    pci_device: _,
                } => {
                    // Handle PCI device resources if needed
                }
                Resource::PcieDevice {
                    id: _,
                    pcie_device: _,
                } => {
                    // Handle PCIe device resources if needed
                }
            }
        }

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
            boot: BootModelBuilder::build(&vm.boot, &storage_resources)?,
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
                Device::Pcie { pcie } => {
                    let pcie_api: Arc<dyn PcieDeviceApi> = match pcie.device() {
                        PcieDeviceType::PvScsi => Arc::new(super::PvScsiController::default()),
                        PcieDeviceType::VirtioNet { resource } => {
                            let resolved = match resource {
                                Some(id) => Some(
                                    network_resources
                                        .get(id)
                                        .cloned()
                                        .ok_or_else(|| {
                                            format!(
                                                "missing network resource '{}' referenced by PCIe virtio_net device",
                                                id
                                            )
                                        })?,
                                ),
                                None => None,
                            };
                            Arc::new(super::VirtioNetController::new(resolved))
                        }
                    };
                    model.register_pcie_device(
                        pcie.bus().unwrap_or_default(),
                        pcie_api,
                        pcie.address().clone(),
                    )?;
                }
                Device::Pci { pci } => {
                    model.register_pci_device(
                        pci.bus().unwrap_or_default(),
                        pci.device().into(),
                        pci.address().clone(),
                    )?;
                }
                Device::Usb { usb } => {
                    model.register_usb_device(
                        usb.bus().unwrap_or_default(),
                        UsbDeviceBuilder::build(usb.device(), &usb_resources),
                        usb.address().clone(),
                    )?;
                }
                Device::Ide { ide } => {
                    model.register_ide_device(
                        ide.bus().unwrap_or_default(),
                        IdeDeviceBuilder::build(ide.device(), &storage_resources)?,
                        ide.address().clone(),
                    )?;
                }
                Device::Sata { sata } => {
                    model.register_sata_device(
                        sata.bus().unwrap_or_default(),
                        SataDeviceBuilder::build(sata.device(), &storage_resources)?,
                        sata.address().clone(),
                    )?;
                }
                Device::Scsi { scsi } => {
                    model.register_scsi_device(
                        scsi.bus().unwrap_or_default(),
                        ScsiDeviceBuilder::build(scsi.device(), &storage_resources)?,
                        scsi.address().clone(),
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
        writeln!(f, "  Boot: {}", self.boot)?;
        writeln!(f, "  Busses: {}", self.busses)?;
        Ok(())
    }
}
