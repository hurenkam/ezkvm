//! SchemaBuilder stage: `RuntimeModel` → `EzkvmConfigSchema`.

use std::collections::HashMap;

use crate::{
    config_format::{
        SchemaBuilder,
        ezkvm::{
            Bios, Boot, Device, EzkvmConfigSchema, Machine, VirtualMachine,
            schema::{
                AudioSchema, DisplaySchema, EZKVM_CONFIG_SCHEMA_VERSION, HostSchema, Metadata,
                SpiceSchema, Uefi, VncSchema,
            },
        },
    },
    runtime_model::{
        Audio, AudioBackend, AudioController, BiosModel, BootModel, Chipset, Display, IdeDevice,
        IdeDeviceType, NetworkResource, PcieAddress, PcieDevice, PcieDeviceKind,
        PcieDeviceResource, PcieDeviceType, Resource, RuntimeModel, SataDevice, SataDeviceType,
        ScsiDevice, ScsiDeviceType, StorageDeviceKind, StorageResource, Tpm,
    },
};

#[derive(Default)]
struct ResourceIndex {
    resources: Vec<Resource>,
    storage: HashMap<String, String>,
    network: HashMap<String, String>,
    hostpci: HashMap<String, String>,
}

impl ResourceIndex {
    fn storage_id(&mut self, resource: StorageResource) -> String {
        let key = serde_json::to_string(&resource).expect("storage resource should serialize");
        if let Some(id) = self.storage.get(&key) {
            return id.clone();
        }

        let id = format!("storage{}", self.storage.len());
        self.resources.push(Resource::Storage {
            id: id.clone(),
            storage: resource,
        });
        self.storage.insert(key, id.clone());
        id
    }

    fn network_id(&mut self, resource: NetworkResource) -> String {
        let key = serde_json::to_string(&resource).expect("network resource should serialize");
        if let Some(id) = self.network.get(&key) {
            return id.clone();
        }

        let id = format!("net{}", self.network.len());
        self.resources.push(Resource::Network {
            id: id.clone(),
            network: resource,
        });
        self.network.insert(key, id.clone());
        id
    }

    fn hostpci_id(&mut self, resource: PcieDeviceResource) -> String {
        let key = serde_json::to_string(&resource).expect("pcie resource should serialize");
        if let Some(id) = self.hostpci.get(&key) {
            return id.clone();
        }

        let id = format!("hostpci{}", self.hostpci.len());
        self.resources.push(Resource::PcieDevice {
            id: id.clone(),
            pcie: resource,
        });
        self.hostpci.insert(key, id.clone());
        id
    }

    fn into_resources(self) -> Vec<Resource> {
        self.resources
    }
}

/// Builds an `EzkvmConfigSchema` from a `RuntimeModel`.
///
/// `host_path` provides the host context required during rendering.
#[derive(Default)]
pub struct EzkvmSchemaBuilder {
    pub host_path: Option<std::path::PathBuf>,
    pub runtime: Option<RuntimeModel>,
    #[allow(dead_code)] // TODO: wire to CLI
    pub file_name: Option<std::path::PathBuf>,
}
impl EzkvmSchemaBuilder {
    pub fn with_host_path(self, host_path: std::path::PathBuf) -> Self {
        Self {
            host_path: Some(host_path),
            ..self
        }
    }
    #[allow(dead_code)] // TODO: wire to CLI
    pub fn with_file_name(self, file_name: std::path::PathBuf) -> Self {
        Self {
            file_name: Some(file_name),
            ..self
        }
    }
}
impl SchemaBuilder for EzkvmSchemaBuilder {
    type Schema = EzkvmConfigSchema;

    fn with_runtime(self, runtime: RuntimeModel) -> Self {
        Self {
            runtime: Some(runtime),
            ..self
        }
    }

    fn build(mut self) -> Result<Self::Schema, String> {
        let _host_path = self
            .host_path
            .clone()
            .ok_or_else(|| "host path is required for EzkvmSchemaBuilder".to_string())?;
        let runtime = self
            .runtime
            .take()
            .ok_or_else(|| "runtime model is required for EzkvmSchemaBuilder".to_string())?;

        let mut resources = ResourceIndex::default();
        let host_display = runtime
            .display()
            .as_ref()
            .map(|display| render_host_display(display.config()))
            .transpose()?;
        let (host_audio, vm_audio) =
            render_audio(runtime.audio().as_ref().map(|audio| audio.config().clone()));
        let boot = render_boot(runtime.boot(), &mut resources)?;
        let tpm = render_tpm(runtime.tpm().as_ref(), &mut resources)?;
        let device_list = render_devices(&runtime, &mut resources)?;

        Ok(EzkvmConfigSchema {
            metadata: Metadata {
                schema_version: EZKVM_CONFIG_SCHEMA_VERSION.to_string(),
                vm_name: runtime.name().clone(),
            },
            host: HostSchema {
                display: host_display,
                audio: host_audio,
                resources: resources.into_resources(),
            },
            virtual_machine: VirtualMachine {
                machine: render_machine(runtime.chipset()),
                cpu: Some(runtime.cpu().clone()),
                memory: runtime.memory().clone(),
                boot,
                smbios_uuid: runtime.smbios_uuid().clone(),
                vmgenid: runtime.vmgenid().clone(),
                tpm,
                display: None,
                audio: vm_audio,
                guest_agent: runtime
                    .guest_agent()
                    .as_ref()
                    .map(|guest_agent| guest_agent.config().clone()),
                devices: device_list,
            },
        })
    }
}

fn render_machine(chipset: &Chipset) -> Machine {
    let chipset = match chipset {
        Chipset::Q35(_) => "q35",
        Chipset::I440FX(_) => "i440fx",
    };

    Machine {
        family: "pc".to_string(),
        chipset: chipset.to_string(),
        version: None,
    }
}

fn render_boot(boot: &BootModel, resources: &mut ResourceIndex) -> Result<Boot, String> {
    let bios = match boot.bios() {
        BiosModel::SeaBios(_) => Bios::SeaBios {
            seabios: crate::config_format::ezkvm::schema::SeaBios::default(),
        },
        BiosModel::Uefi(uefi) => Bios::Uefi {
            uefi: Uefi::new(resources.storage_id(uefi.storage().clone())),
        },
    };

    Ok(Boot::new(None, bios))
}

fn render_tpm(
    tpm: Option<&std::sync::Arc<dyn crate::runtime_model::TpmApi>>,
    resources: &mut ResourceIndex,
) -> Result<Option<Tpm>, String> {
    let Some(tpm) = tpm else {
        return Ok(None);
    };

    match (tpm.swtpm_version(), tpm.storage_resource()) {
        (Some(version), Some(storage)) => Ok(Some(Tpm::Emulated {
            swtpm: crate::runtime_model::Swtpm::new(version, resources.storage_id(storage.clone())),
        })),
        _ => Err("unsupported TPM shape for ezkvm rendering".to_string()),
    }
}

fn render_host_display(display: &Display) -> Result<DisplaySchema, String> {
    Ok(match display {
        Display::Gtk { .. } => DisplaySchema::Gtk {
            gtk: crate::config_format::ezkvm::schema::GtkSchema {},
        },
        Display::Sdl { .. } => DisplaySchema::Sdl {
            sdl: crate::config_format::ezkvm::schema::SdlSchema {},
        },
        Display::Vnc { vnc } => DisplaySchema::Vnc {
            vnc: VncSchema {
                port: *vnc.port(),
                listen: vnc.listen().clone(),
            },
        },
        Display::Spice { spice } => DisplaySchema::Spice {
            spice: SpiceSchema {
                port: *spice.port(),
                listen: spice.listen().clone(),
                disable_ticketing: *spice.disable_ticketing(),
            },
        },
        Display::LookingGlass { .. } => {
            return Err(
                "looking glass display is not representable in host display schema".to_string(),
            );
        }
    })
}

fn render_audio(audio: Option<Audio>) -> (Option<AudioSchema>, Option<Audio>) {
    match audio {
        Some(audio) if matches!(audio.controller, AudioController::Ich9IntelHda) => {
            let host_audio = match audio.backend {
                AudioBackend::None => None,
                AudioBackend::Alsa => Some(AudioSchema::Alsa {
                    alsa: crate::config_format::ezkvm::schema::AlsaSchema {},
                }),
                AudioBackend::PulseAudio => Some(AudioSchema::PulseAudio {
                    pulse_audio: crate::config_format::ezkvm::schema::PulseAudioSchema {},
                }),
                AudioBackend::PipeWire => Some(AudioSchema::PipeWire {
                    pipe_wire: crate::config_format::ezkvm::schema::PipeWireSchema {},
                }),
            };
            (host_audio, None)
        }
        Some(audio) => (None, Some(audio)),
        None => (None, None),
    }
}

fn render_devices(
    runtime: &RuntimeModel,
    resources: &mut ResourceIndex,
) -> Result<Vec<Device>, String> {
    let mut devices = Vec::new();

    if let Some(root) = runtime.busses().pcie_busses().get(&0) {
        let mut entries: Vec<_> = root.devices().into_iter().collect();
        entries.sort_by_key(|(address, _)| (address.device(), address.function()));

        for (address, device) in entries {
            devices.push(Device::Pcie {
                pcie: render_pcie_device(address, device.as_ref(), resources)?,
            });
        }
    }

    let mut scsi_busses: Vec<_> = runtime.busses().scsi_busses().iter().collect();
    scsi_busses.sort_by_key(|(bus_id, _)| **bus_id);
    for (bus_id, controller) in scsi_busses {
        let mut entries: Vec<_> = controller.devices().into_iter().collect();
        entries.sort_by_key(|(address, _)| (address.target, address.lun));
        for (address, device) in entries {
            let resource = resources.storage_id(device.storage_resource().clone());
            let kind = match device.storage_kind() {
                StorageDeviceKind::Hdd => ScsiDeviceType::Hdd { resource },
                StorageDeviceKind::Ssd => ScsiDeviceType::Ssd { resource },
                StorageDeviceKind::Cdrom => ScsiDeviceType::Cdrom { resource },
            };
            devices.push(Device::Scsi {
                scsi: ScsiDevice::new(Some(*bus_id), Some(address), kind),
            });
        }
    }

    let mut sata_busses: Vec<_> = runtime.busses().sata_busses().iter().collect();
    sata_busses.sort_by_key(|(bus_id, _)| **bus_id);
    for (bus_id, controller) in sata_busses {
        let mut entries: Vec<_> = controller.devices().into_iter().collect();
        entries.sort_by_key(|(address, _)| address.address);
        for (address, device) in entries {
            let resource = resources.storage_id(device.storage_resource().clone());
            let kind = match device.storage_kind() {
                StorageDeviceKind::Hdd => SataDeviceType::Hdd { resource },
                StorageDeviceKind::Ssd => SataDeviceType::Ssd { resource },
                StorageDeviceKind::Cdrom => SataDeviceType::Cdrom { resource },
            };
            devices.push(Device::Sata {
                sata: SataDevice::new(Some(*bus_id), Some(address), kind),
            });
        }
    }

    let mut ide_busses: Vec<_> = runtime.busses().ide_busses().iter().collect();
    ide_busses.sort_by_key(|(bus_id, _)| **bus_id);
    for (bus_id, controller) in ide_busses {
        let mut entries: Vec<_> = controller.devices().into_iter().collect();
        entries.sort_by_key(|(address, _)| address.address);
        for (address, device) in entries {
            let resource = resources.storage_id(device.storage_resource().clone());
            let kind = match device.storage_kind() {
                StorageDeviceKind::Hdd => IdeDeviceType::Hdd { resource },
                StorageDeviceKind::Ssd => IdeDeviceType::Ssd { resource },
                StorageDeviceKind::Cdrom => IdeDeviceType::Cdrom { resource },
            };
            devices.push(Device::Ide {
                ide: IdeDevice::new(Some(*bus_id), Some(address), kind),
            });
        }
    }

    Ok(devices)
}

fn render_pcie_device(
    address: PcieAddress,
    device: &dyn crate::runtime_model::PcieDeviceApi,
    resources: &mut ResourceIndex,
) -> Result<PcieDevice, String> {
    let device_type = match device.device_kind() {
        PcieDeviceKind::PvScsi => PcieDeviceType::PvScsi,
        PcieDeviceKind::VirtioNet => PcieDeviceType::VirtioNet {
            resource: device
                .network_resource()
                .cloned()
                .map(|network| resources.network_id(network)),
        },
        PcieDeviceKind::StandardGpu => PcieDeviceType::StandardGpu,
        PcieDeviceKind::VirtioGpu => PcieDeviceType::VirtioGpu,
        PcieDeviceKind::Passthrough => {
            let spec = device
                .passthrough_spec()
                .ok_or_else(|| "passthrough device is missing passthrough metadata".to_string())?;
            let resource = match device.resource_id() {
                Some(id) => Some(id.to_string()),
                None => Some(resources.hostpci_id(PcieDeviceResource::HostAddress {
                    address: spec.host,
                    multifunction: spec.multifunction,
                    rombar: spec.rombar,
                    romfile: spec.romfile,
                })),
            };

            PcieDeviceType::Passthrough {
                resource,
                host: None,
                id: spec.id,
                multifunction: None,
                rombar: None,
                romfile: None,
            }
        }
        PcieDeviceKind::PassthroughGpu => PcieDeviceType::PassthroughGpu {
            resource: device.resource_id().unwrap_or_default().to_string(),
        },
        PcieDeviceKind::Ich9IntelHda => PcieDeviceType::Ich9IntelHda { codec: None },
        PcieDeviceKind::IvshmemPlain => PcieDeviceType::IvshmemPlain {
            resource: device.resource_id().unwrap_or_default().to_string(),
        },
    };

    Ok(PcieDevice::new(Some(0), Some(address), device_type))
}
