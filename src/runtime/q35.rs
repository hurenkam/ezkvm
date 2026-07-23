use std::{collections::HashMap, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use crate::runtime::{
    GenericPciDevice, GenericUsbDevice, HostPci, IdeAddress, IdeDevice, Ivshmem, PciBusDeviceKind, PciDevice,
    PcieAddress, PcieBusDeviceKind, PcieDevice, PvScsi, SataAddress, SataDevice,
    StorageDeviceType, UsbBusDeviceKind, VirtioNetPcie,
    pci::PciAddress,
    usb::{UsbAddress, UsbDevice},
};

pub struct Q35ChipsetBuilder {
    pcie_bus: HashMap<PcieAddress, Arc<dyn PcieDevice>>,
    pci_bus: HashMap<PciAddress, Arc<dyn PciDevice>>,
    sata_bus: HashMap<SataAddress, Arc<dyn SataDevice>>,
    ide_bus: HashMap<IdeAddress, Arc<dyn IdeDevice>>,
    usb_bus: HashMap<UsbAddress, Arc<dyn UsbDevice>>,
}
#[allow(dead_code)]
impl Q35ChipsetBuilder {
    pub fn new() -> Self {
        Self {
            pcie_bus: HashMap::new(),
            pci_bus: HashMap::new(),
            sata_bus: HashMap::new(),
            ide_bus: HashMap::new(),
            usb_bus: HashMap::new(),
        }
    }
    pub fn with_pcie_device(
        mut self,
        address: Option<PcieAddress>,
        device: Arc<dyn PcieDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => PcieAddress::new(0, 0),
        };
        self.pcie_bus.insert(address, device);
        self
    }
    pub fn with_pci_device(
        mut self,
        address: Option<PciAddress>,
        device: Arc<dyn PciDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => PciAddress::new(0, 0),
        };
        self.pci_bus.insert(address, device);
        self
    }
    pub fn with_sata_device(
        mut self,
        address: Option<SataAddress>,
        device: Arc<dyn SataDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => SataAddress::new(0, 0),
        };
        self.sata_bus.insert(address, device);
        self
    }
    pub fn with_ide_device(
        mut self,
        address: Option<IdeAddress>,
        device: Arc<dyn IdeDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => IdeAddress::new(0, 0),
        };
        self.ide_bus.insert(address, device);
        self
    }
    pub fn with_usb_device(
        mut self,
        address: Option<UsbAddress>,
        device: Arc<dyn UsbDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => UsbAddress::new("1".to_string()),
        };
        self.usb_bus.insert(address, device);
        self
    }
    pub fn with_host_pci(mut self, idx: u8, device: Arc<dyn PcieDevice>) -> Self {
        self.pcie_bus.insert(PcieAddress::new(idx, 0), device);
        self
    }
    pub fn with_ivshmem(mut self, idx: u8, device: Arc<dyn PcieDevice>) -> Self {
        self.pcie_bus.insert(PcieAddress::new(32 + idx, 0), device);
        self
    }
    pub fn build(self) -> Q35Chipset {
        Q35Chipset::new(
            self.pcie_bus,
            self.pci_bus,
            self.sata_bus,
            self.ide_bus,
            self.usb_bus,
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct Q35Chipset {
    pcie_bus: HashMap<PcieAddress, Arc<dyn PcieDevice>>,
    pci_bus: HashMap<PciAddress, Arc<dyn PciDevice>>,
    sata_bus: HashMap<SataAddress, Arc<dyn SataDevice>>,
    ide_bus: HashMap<IdeAddress, Arc<dyn IdeDevice>>,
    usb_bus: HashMap<UsbAddress, Arc<dyn UsbDevice>>,
}

impl std::fmt::Display for Q35Chipset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Q35Chipset:")?;

        writeln!(f, "  +-pcie:")?;
        let mut pcie_entries: Vec<_> = self.pcie_bus.iter().collect();
        pcie_entries.sort_by_key(|(address, _)| (*address.device(), *address.function()));
        for (address, device) in pcie_entries {
            let rendered = format_pcie_device(device.as_ref()).replace('\n', "\n         ");
            writeln!(f, "      +-{}: {}", address, rendered)?;
        }

        writeln!(f, "  +-pci:")?;
        let mut pci_entries: Vec<_> = self.pci_bus.iter().collect();
        pci_entries.sort_by_key(|(address, _)| (*address.device(), *address.function()));
        for (address, device) in pci_entries {
            let rendered = format_pci_device(device.as_ref()).replace('\n', "\n         ");
            writeln!(f, "      +-{}: {}", address, rendered)?;
        }

        writeln!(f, "  +-sata:")?;
        let mut sata_entries: Vec<_> = self.sata_bus.iter().collect();
        sata_entries.sort_by_key(|(address, _)| (*address.port(), *address.device()));
        for (address, device) in sata_entries {
            writeln!(f, "      +-{}: {}", address, format_storage_device(device.as_ref()))?;
        }

        writeln!(f, "  +-ide:")?;
        let mut ide_entries: Vec<_> = self.ide_bus.iter().collect();
        ide_entries.sort_by_key(|(address, _)| (*address.channel(), *address.device()));
        for (address, device) in ide_entries {
            writeln!(f, "      +-{}: {:?}", address, device)?;
        }

        writeln!(f, "  +-usb:")?;
        let mut usb_entries: Vec<_> = self.usb_bus.iter().collect();
        usb_entries.sort_by_key(|(address, _)| address.port().clone());
        for (address, device) in usb_entries {
            writeln!(f, "      +-{}: {}", address.port(), format_usb_device(device.as_ref()))?;
        }

        Ok(())
    }
}

fn format_pcie_device(device: &dyn PcieDevice) -> String {
    match device.device_kind() {
        PcieBusDeviceKind::PvScsi => {
            let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
            format!("{}", pvscsi)
        }
        PcieBusDeviceKind::VirtioNet => {
            let virtio_net = device.as_any().downcast_ref::<VirtioNetPcie>().unwrap();
            format!(
                "VirtioNetPcie(resource={:?}, mac_address={:?}, rx_queue_size={:?}, tx_queue_size={:?}, vhost={:?})",
                virtio_net.resource(),
                virtio_net.mac_address(),
                virtio_net.rx_queue_size(),
                virtio_net.tx_queue_size(),
                virtio_net.vhost(),
            )
        }
        PcieBusDeviceKind::HostPci => {
            let host_pci = device.as_any().downcast_ref::<HostPci>().unwrap();
            format!(
                "HostPci(base_bdf={:?}, functions={:?}, pcie={}, x_vga={}, rombar={:?}, romfile={:?})",
                host_pci.base_bdf(),
                host_pci.functions(),
                host_pci.pcie(),
                host_pci.x_vga(),
                host_pci.rombar(),
                host_pci.romfile(),
            )
        }
        PcieBusDeviceKind::Ivshmem => {
            let ivshmem = device.as_any().downcast_ref::<Ivshmem>().unwrap();
            format!(
                "Ivshmem(id={:?}, mem_path={:?}, size={:?})",
                ivshmem.id(),
                ivshmem.mem_path(),
                ivshmem.size(),
            )
        }
    }
}

fn format_pci_device(device: &dyn PciDevice) -> String {
    match device.device_kind() {
        PciBusDeviceKind::GenericPci => {
            let generic = device.as_any().downcast_ref::<GenericPciDevice>().unwrap();
            format!("GenericPciDevice({:?})", generic.kind())
        }
        PciBusDeviceKind::PvScsi => {
            let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
            format!("{}", pvscsi)
        }
    }
}

fn format_usb_device(device: &dyn UsbDevice) -> String {
    match device.device_kind() {
        UsbBusDeviceKind::Generic => {
            let generic = device.as_any().downcast_ref::<GenericUsbDevice>().unwrap();
            format!("GenericUsbDevice({:?})", generic.kind())
        }
    }
}

fn format_storage_device(device: &dyn SataDevice) -> &'static str {
    match device.storage_options().device_type {
        StorageDeviceType::Ssd => "Ssd",
        StorageDeviceType::Hdd => "Hdd",
        StorageDeviceType::Odd => "Cdrom",
    }
}
