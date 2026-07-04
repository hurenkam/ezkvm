//! RuntimeBuilder stage: `EzkvmConfigSchema` → `RuntimeModel`.

use std::{collections::HashMap, sync::Arc};

use crate::{
    config_format::{
        RuntimeBuilder,
        ezkvm::{Bios, Boot, Device, EzkvmConfigSchema, Machine, schema::DisplaySchema},
    },
    runtime_model::{
        Audio, AudioModelBuilder, BiosModel, BootModel, BusRegister, BusRegistrationApi, Chipset,
        Display, DisplayModelBuilder, GuestAgentModelBuilder, I440fxChipset, IdeDeviceBuilder,
        NetworkResource, PciDeviceResource, PcieAddress, PcieDeviceApi, PcieDeviceResource,
        PcieDeviceType, PvScsiController, Q35Chipset, RuntimeModel, SataDeviceBuilder,
        ScsiControllerApi, ScsiDeviceBuilder, SeaBiosModel, StorageResource, TpmModelBuilder,
        UefiModel, UsbDeviceBuilder, UsbDeviceResource, VirtioNetController,
    },
};

/// Builds a `RuntimeModel` from an `EzkvmConfigSchema`.
///
/// `host_path` and `vm_name` provide the host context required during assembly.
#[derive(Default)]
pub struct EzkvmRuntimeBuilder {
    pub host_path: Option<std::path::PathBuf>,
    pub vm_name: Option<String>,
    pub schema: Option<EzkvmConfigSchema>,
}
impl EzkvmRuntimeBuilder {
    pub fn with_host_path(self, host_path: std::path::PathBuf) -> Self {
        Self {
            host_path: Some(host_path),
            ..self
        }
    }
    pub fn with_vm_name(self, vm_name: String) -> Self {
        Self {
            vm_name: Some(vm_name),
            ..self
        }
    }
}

impl RuntimeBuilder for EzkvmRuntimeBuilder {
    type Schema = EzkvmConfigSchema;

    fn build(self) -> Result<RuntimeModel, String> {
        let _host_path = self
            .host_path
            .clone()
            .ok_or_else(|| "host path is required for EzkvmRuntimeBuilder".to_string())?;
        let vm_name = self
            .vm_name
            .clone()
            .ok_or_else(|| "vm name is required for EzkvmRuntimeBuilder".to_string())?;
        let schema = self
            .schema
            .clone()
            .ok_or_else(|| "schema is required for EzkvmRuntimeBuilder".to_string())?;

        build_runtime_model(&schema, vm_name)
    }

    fn with_schema(self, schema: Self::Schema) -> Self {
        Self {
            schema: Some(schema),
            ..self
        }
    }
}

struct BootModelBuilder {}
impl BootModelBuilder {
    pub fn build(
        boot: &Boot,
        storage_resources: &HashMap<String, StorageResource>,
    ) -> Result<BootModel, String> {
        let bios = match boot.bios() {
            Bios::SeaBios { seabios: _ } => BiosModel::SeaBios(SeaBiosModel {}),
            Bios::Uefi { uefi } => {
                let uefi_resource = storage_resources.get(uefi.resource()).ok_or_else(|| {
                    format!(
                        "missing storage resource '{}' referenced by UEFI firmware",
                        uefi.resource()
                    )
                })?;
                BiosModel::Uefi(UefiModel::new(uefi_resource.clone()))
            }
        };
        Ok(BootModel::new(bios))
    }
}

struct ResourceMaps {
    storage: HashMap<String, StorageResource>,
    network: HashMap<String, NetworkResource>,
    pcie: HashMap<String, PcieDeviceResource>,
    usb: HashMap<String, UsbDeviceResource>,
    _pci: HashMap<String, PciDeviceResource>,
}

fn build_runtime_model(
    schema: &EzkvmConfigSchema,
    runtime_name: String,
) -> Result<RuntimeModel, String> {
    let resources = collect_resources(&schema.host.resources);

    let mut bus_register = BusRegister::new();
    let chipset = build_chipset(&schema.virtual_machine.machine, &mut bus_register)?;
    let cpu = schema.virtual_machine.cpu.clone().unwrap_or_default();
    let memory = schema.virtual_machine.memory.clone();
    let boot = BootModelBuilder::build(&schema.virtual_machine.boot, &resources.storage)?;

    let tpm = match schema.virtual_machine.tpm.clone() {
        Some(tpm) => Some(TpmModelBuilder::build(tpm, &resources.storage)?),
        None => None,
    };

    let display = select_display(schema).map(DisplayModelBuilder::build);
    let audio = select_audio(schema).map(AudioModelBuilder::build);
    let guest_agent = schema
        .virtual_machine
        .guest_agent
        .clone()
        .map(GuestAgentModelBuilder::build);

    register_devices(&mut bus_register, schema, &resources)?;

    Ok(RuntimeModel::new(
        runtime_name,
        cpu,
        memory,
        chipset,
        boot,
        schema.virtual_machine.smbios_uuid.clone(),
        schema.virtual_machine.vmgenid.clone(),
        tpm,
        display,
        audio,
        guest_agent,
        bus_register,
    ))
}

fn build_chipset(machine: &Machine, bus_register: &mut BusRegister) -> Result<Chipset, String> {
    match machine.chipset.as_str() {
        "q35" => Ok(Chipset::Q35(Q35Chipset::new(bus_register))),
        "i440fx" => Ok(Chipset::I440FX(I440fxChipset::new(bus_register))),
        other => Err(format!("unsupported chipset '{other}'")),
    }
}

fn select_display(schema: &EzkvmConfigSchema) -> Option<Display> {
    schema
        .virtual_machine
        .display
        .clone()
        .or_else(|| schema.host.display.as_ref().and_then(convert_host_display))
}

fn select_audio(schema: &EzkvmConfigSchema) -> Option<Audio> {
    schema.virtual_machine.audio.clone()
}

fn convert_host_display(display: &DisplaySchema) -> Option<Display> {
    Some(match display {
        DisplaySchema::Vnc { vnc } => Display::Vnc {
            vnc: crate::runtime_model::Vnc::new(vnc.listen.clone(), vnc.port),
        },
        DisplaySchema::Spice { spice } => Display::Spice {
            spice: crate::runtime_model::Spice::new(
                spice.listen.clone(),
                spice.port,
                spice.disable_ticketing,
            ),
        },
        DisplaySchema::Gtk { .. } => Display::Gtk {
            gtk: crate::runtime_model::Gtk::default(),
        },
        DisplaySchema::Sdl { .. } => Display::Sdl {
            sdl: crate::runtime_model::Sdl::default(),
        },
        DisplaySchema::LookingGlass { .. } => return None,
    })
}

fn collect_resources(resources: &[crate::runtime_model::Resource]) -> ResourceMaps {
    let mut storage = HashMap::new();
    let mut network = HashMap::new();
    let mut pcie = HashMap::new();
    let mut usb = HashMap::new();
    let mut pci = HashMap::new();

    for resource in resources {
        match resource {
            crate::runtime_model::Resource::Storage { id, storage: value } => {
                storage.insert(id.clone(), value.clone());
            }
            crate::runtime_model::Resource::Network { id, network: value } => {
                network.insert(id.clone(), value.clone());
            }
            crate::runtime_model::Resource::PciDevice { id, pci_device } => {
                pci.insert(id.clone(), pci_device.clone());
            }
            crate::runtime_model::Resource::PcieDevice { id, pcie: value } => {
                pcie.insert(id.clone(), value.clone());
            }
            crate::runtime_model::Resource::UsbDevice { id, usb_device } => {
                usb.insert(id.clone(), usb_device.clone());
            }
        }
    }

    ResourceMaps {
        storage,
        network,
        pcie,
        usb,
        _pci: pci,
    }
}

fn register_devices(
    bus_register: &mut BusRegister,
    schema: &EzkvmConfigSchema,
    resources: &ResourceMaps,
) -> Result<(), String> {
    let mut scsi_controllers: HashMap<u8, Arc<dyn ScsiControllerApi>> = HashMap::new();

    for device in &schema.virtual_machine.devices {
        match device {
            Device::Pcie { pcie } => {
                let bus_id = pcie.bus().unwrap_or(0);
                let root = bus_register
                    .pcie_busses()
                    .get(&bus_id)
                    .ok_or_else(|| format!("PCIe bus with id {bus_id} does not exist"))?;
                let preferred = pcie.address().clone();

                match pcie.device() {
                    PcieDeviceType::PvScsi => {
                        let controller = Arc::new(PvScsiController::default());
                        let pcie_device: Arc<dyn PcieDeviceApi> = controller.clone();
                        root.register_pcie_device(pcie_device, preferred)?;
                        let scsi_bus_id = bus_register.register_scsi_bus(controller.clone())?;
                        scsi_controllers.insert(scsi_bus_id, controller);
                    }
                    other => {
                        let built = build_pcie_device(other, resources)?;
                        root.register_pcie_device(built, preferred)?;
                    }
                }
            }
            Device::Pci { pci } => {
                let bus_id = pci.bus().unwrap_or(0);
                let root = bus_register
                    .pci_busses()
                    .get(&bus_id)
                    .ok_or_else(|| format!("PCI bus with id {bus_id} does not exist"))?;
                let runtime: Arc<dyn crate::runtime_model::PciDeviceApi> = pci.device().into();
                root.register_pci_device(runtime, pci.address().clone())?;
            }
            Device::Usb { usb } => {
                let bus_id = usb.bus().unwrap_or(0);
                let root = bus_register
                    .usb_busses()
                    .get(&bus_id)
                    .ok_or_else(|| format!("USB bus with id {bus_id} does not exist"))?;
                let runtime = UsbDeviceBuilder::build(usb.device(), &resources.usb)?;
                root.register_usb_device(runtime, usb.address().clone())?;
            }
            Device::Sata { sata } => {
                let bus_id = sata.bus().unwrap_or(0);
                let root = bus_register
                    .sata_busses()
                    .get(&bus_id)
                    .ok_or_else(|| format!("SATA bus with id {bus_id} does not exist"))?;
                let runtime = SataDeviceBuilder::build(sata.device(), &resources.storage)?;
                root.register_sata_device(runtime, sata.address().clone())?;
            }
            Device::Ide { ide } => {
                let bus_id = ide.bus().unwrap_or(0);
                let root = bus_register
                    .ide_busses()
                    .get(&bus_id)
                    .ok_or_else(|| format!("IDE bus with id {bus_id} does not exist"))?;
                let runtime = IdeDeviceBuilder::build(ide.device(), &resources.storage)?;
                root.register_ide_device(runtime, ide.address().clone())?;
            }
            Device::Scsi { scsi } => {
                let bus_id = scsi.bus().unwrap_or(0);
                let controller = match scsi_controllers.get(&bus_id) {
                    Some(controller) => controller.clone(),
                    None => {
                        let root = bus_register
                            .pcie_busses()
                            .get(&0)
                            .ok_or_else(|| "PCIe root bus with id 0 does not exist".to_string())?;
                        let controller = Arc::new(PvScsiController::default());
                        let pcie_device: Arc<dyn PcieDeviceApi> = controller.clone();
                        root.register_pcie_device(pcie_device, Some(PcieAddress::new(0, 0)))?;
                        let registered_bus = bus_register.register_scsi_bus(controller.clone())?;
                        scsi_controllers.insert(registered_bus, controller.clone());
                        controller
                    }
                };
                let runtime = ScsiDeviceBuilder::build(scsi.device(), &resources.storage)?;
                controller.register_scsi_device(runtime, scsi.address().clone())?;
            }
        }
    }

    Ok(())
}

fn build_pcie_device(
    device: &PcieDeviceType,
    resources: &ResourceMaps,
) -> Result<Arc<dyn PcieDeviceApi>, String> {
    Ok(match device {
        PcieDeviceType::VirtioNet {
            resource,
            mac_address,
            rx_queue_size,
            tx_queue_size,
            vhost,
        } => Arc::new(VirtioNetController::new(
            resource
                .as_ref()
                .and_then(|id| resources.network.get(id).cloned()),
            mac_address.clone(),
            *rx_queue_size,
            *tx_queue_size,
            *vhost,
        )),
        PcieDeviceType::Passthrough {
            resource,
            host,
            id,
            multifunction,
            rombar,
            romfile,
        } => {
            let resolved = match (resource.as_ref(), host.as_ref()) {
                (_, Some(host)) => PcieDeviceType::Passthrough {
                    resource: resource.clone(),
                    host: Some(host.clone()),
                    id: id.clone(),
                    multifunction: *multifunction,
                    rombar: *rombar,
                    romfile: romfile.clone(),
                },
                (Some(resource_id), None) => {
                    let mapped = resources.pcie.get(resource_id).ok_or_else(|| {
                        format!("missing pcie resource '{resource_id}' referenced by passthrough")
                    })?;
                    match mapped {
                        PcieDeviceResource::HostAddress {
                            address,
                            multifunction,
                            rombar,
                            romfile,
                        } => PcieDeviceType::Passthrough {
                            resource: Some(resource_id.clone()),
                            host: Some(address.clone()),
                            id: id.clone(),
                            multifunction: *multifunction,
                            rombar: *rombar,
                            romfile: romfile.clone(),
                        },
                        PcieDeviceResource::Address { .. } => {
                            return Err(format!(
                                "pcie address resource '{resource_id}' cannot back passthrough"
                            ));
                        }
                    }
                }
                (None, None) => {
                    return Err(
                        "passthrough device requires either a host address or resource".to_string(),
                    );
                }
            };

            (&resolved).into()
        }
        other => other.into(),
    })
}
