use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::VirtualMachine;
use crate::{
    config_format::ezkvm::{Bios, Boot},
    runtime_model::{BiosModel, BootModel, Resource, SeaBiosModel, StorageResource, UefiModel},
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
