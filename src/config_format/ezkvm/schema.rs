use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::runtime_model::{
    BiosModel, BootModel, Resource, SeaBiosModel, StorageResource, UefiModel,
};

use derive_getters::Getters;
use derive_new::new;

use crate::runtime_model::{
    Audio, Cpu, Display, GuestAgent, IdeDevice, Memory, PciDevice, PcieDevice, SataDevice,
    ScsiDevice, Tpm, UsbDevice,
};

/// Schema version for ezkvm runtime config specification.
pub const EZKVM_CONFIG_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub schema_version: String,
    pub vm_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HostSchema {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub display: Option<DisplaySchema>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioSchema>,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DisplaySchema {
    Vnc { vnc: VncSchema },
    Spice { spice: SpiceSchema },
    LookingGlass { looking_glass: LookingGlassSchema },
    Gtk { gtk: GtkSchema },
    Sdl { sdl: SdlSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VncSchema {
    pub port: u16,
    pub listen: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpiceSchema {
    pub port: u16,
    pub listen: String,
    pub disable_ticketing: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LookingGlassSchema {
    pub port: u16,
    pub listen: String,
    pub disable_ticketing: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GtkSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SdlSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AudioSchema {
    Alsa { alsa: AlsaSchema },
    PulseAudio { pulse_audio: PulseAudioSchema },
    PipeWire { pipe_wire: PipeWireSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlsaSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PulseAudioSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipeWireSchema {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EzkvmConfigSchema {
    pub metadata: Metadata,
    pub host: HostSchema,
    pub virtual_machine: VirtualMachine,
}

impl std::fmt::Display for EzkvmConfigSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rendered = serde_yaml::to_string(self).map_err(|_| std::fmt::Error)?;
        f.write_str(&rendered)
    }
}

pub struct BootModelBuilder {}
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub machine: Machine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Cpu>,
    pub memory: Memory,
    #[serde(default)]
    pub boot: Boot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smbios_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vmgenid: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tpm: Option<Tpm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Display>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<Audio>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_agent: Option<GuestAgent>,
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Device {
    Pcie { pcie: PcieDevice },
    Pci { pci: PciDevice },
    Usb { usb: UsbDevice },
    Sata { sata: SataDevice },
    Ide { ide: IdeDevice },
    Scsi { scsi: ScsiDevice },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Getters, new)]
pub struct Boot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    secure: Option<bool>,
    #[serde(flatten)]
    bios: Bios,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bios {
    SeaBios { seabios: SeaBios },
    Uefi { uefi: Uefi },
}
impl Default for Bios {
    fn default() -> Self {
        Bios::SeaBios {
            seabios: SeaBios::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SeaBios {
    firmware: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct Uefi {
    resource: String,
}
