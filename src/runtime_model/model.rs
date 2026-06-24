#![allow(clippy::too_many_arguments)]
use std::{fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use super::{
    Cpu, IdeAddress, IdeBus, IdeControllerApi, IdeDeviceApi, Memory, PciAddress, PciBus,
    PciControllerApi, PciDeviceApi, PcieAddress, PcieBus, PcieControllerApi, PcieDeviceApi,
    SataAddress, SataBus, SataControllerApi, SataDeviceApi, ScsiAddress, ScsiBus,
    ScsiControllerApi, ScsiDeviceApi, UsbAddress, UsbBus, UsbControllerApi, UsbDeviceApi,
};
use crate::runtime_model::{
    AudioApi, BootModel, BusRegister, Chipset, DisplayApi, GuestAgentApi, TpmApi,
};

pub trait ControllerApi {
    fn qemu_args(&self) -> Vec<String>;
}

#[allow(dead_code)]
#[derive(Getters, new)]
pub struct RuntimeModel {
    name: String,
    cpu: Cpu,
    memory: Memory,
    chipset: Chipset,
    boot: BootModel,
    tpm: Option<Arc<dyn TpmApi>>,
    display: Option<Arc<dyn DisplayApi>>,
    audio: Option<Arc<dyn AudioApi>>,
    guest_agent: Option<Arc<dyn GuestAgentApi>>,
    busses: BusRegister,
}
impl RuntimeModel {
    pub fn get_pcie_bus(&self, id: PcieBus) -> Arc<dyn PcieControllerApi> {
        self.busses
            .pcie_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("PCIe bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_pci_bus(&self, id: PciBus) -> Arc<dyn PciControllerApi> {
        self.busses
            .pci_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("PCI bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_usb_bus(&self, id: UsbBus) -> Arc<dyn UsbControllerApi> {
        self.busses
            .usb_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("USB bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_sata_bus(&self, id: SataBus) -> Arc<dyn SataControllerApi> {
        self.busses
            .sata_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("SATA bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_ide_bus(&self, id: IdeBus) -> Arc<dyn IdeControllerApi> {
        self.busses
            .ide_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("IDE bus with id {} does not exist", id))
            .clone()
    }
    pub fn get_scsi_bus(&self, id: ScsiBus) -> Arc<dyn ScsiControllerApi> {
        self.busses
            .scsi_busses()
            .get(&id)
            .unwrap_or_else(|| panic!("SCSI bus with id {} does not exist", id))
            .clone()
    }
    pub fn register_pcie_device(
        &self,
        bus_id: PcieBus,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String> {
        match self.busses.pcie_busses().get(&bus_id) {
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
        match self.busses.pci_busses().get(&bus_id) {
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
        match self.busses.usb_busses().get(&bus_id) {
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
        match self.busses.sata_busses().get(&bus_id) {
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
        match self.busses.ide_busses().get(&bus_id) {
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
        match self.busses.scsi_busses().get(&bus_id) {
            Some(controller) => {
                controller.register_scsi_device(device, preferred_address)?;
                Ok(())
            }
            None => Err(format!("SCSI bus with id {} does not exist", bus_id)),
        }
    }
    pub fn start(&self) -> Result<(), String> {
        println!("qemu command: {}", self.qemu_command_display());
        Ok(())
    }

    pub fn qemu_command(&self) -> Vec<String> {
        self.generate_qemu_command()
    }

    pub fn qemu_command_display(&self) -> String {
        self.qemu_command()
            .into_iter()
            .map(|arg| shell_escape(&arg))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn generate_qemu_command(&self) -> Vec<String> {
        let mut args = vec![
            "qemu-system-x86_64".to_string(),
            "-name".to_string(),
            self.name.clone(),
        ];
        args.extend(self.cpu.qemu_args());
        args.extend(self.memory.qemu_args());
        args.extend(self.chipset.qemu_args());
        args.extend(self.boot.qemu_args());
        if let Some(display) = &self.display {
            args.extend(display.qemu_args());
        }
        if let Some(audio) = &self.audio {
            args.extend(audio.qemu_args());
        }
        if let Some(tpm) = &self.tpm {
            args.extend(tpm.qemu_args(&self.name));
        }
        if let Some(guest_agent) = &self.guest_agent {
            args.extend(guest_agent.qemu_args(&self.name));
        }
        args.extend(self.busses.qemu_args());
        args
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

fn shell_escape(arg: &str) -> String {
    if arg.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | ',' | '=' | '+')
    }) {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

impl Display for RuntimeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "RuntimeModel for VM '{}':", self.name)?;
        writeln!(f, "  {:?}", self.cpu)?;
        writeln!(f, "  {:?}", self.memory)?;
        writeln!(
            f,
            "  Chipset: {}",
            match &self.chipset {
                Chipset::Q35(_) => "Q35",
                Chipset::I440FX(_) => "I440FX",
            }
        )?;
        writeln!(f, "  {}", self.boot)?;
        writeln!(
            f,
            "  {}",
            match &self.tpm {
                Some(tpm) => format!("{}", tpm),
                None => "None".to_string(),
            }
        )?;
        writeln!(
            f,
            "  Display: {}",
            match &self.display {
                Some(d) => format!("{}", d),
                None => "none".to_string(),
            }
        )?;
        writeln!(
            f,
            "  Audio: {}",
            match &self.audio {
                Some(a) => format!("{}", a),
                None => "none".to_string(),
            }
        )?;
        writeln!(
            f,
            "  Guest Agent: {}",
            match &self.guest_agent {
                Some(ga) => format!("{}", ga),
                None => "none".to_string(),
            }
        )?;
        writeln!(f, "  Busses: {}", self.busses)?;
        Ok(())
    }
}
