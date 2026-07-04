use std::fmt::{Display as FmtDisplay, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::serde_yaml;
use crate::{
    config_format::ezkvm::compact_yaml::{ToStyledYaml, emit_styled_yaml},
    runtime_model::Resource,
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
pub struct EzkvmConfigSchema {
    pub metadata: Metadata,
    pub host: HostSchema,
    pub virtual_machine: VirtualMachine,
}

impl FmtDisplay for EzkvmConfigSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_yaml::to_string(self) {
            Ok(serialized) => f.write_str(&serialized),
            Err(_) => f.write_str("--- failed to serialize ---"),
        }
    }
}

impl FromStr for EzkvmConfigSchema {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s).map_err(|e| format!("failed to parse YAML: {e}"))
    }
}

impl EzkvmConfigSchema {
    pub fn to_styled_compact_yaml(&self) -> Result<String, String> {
        let config_styled = self.to_styled_yaml();
        emit_styled_yaml(&config_styled)
    }
}

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
    EglHeadless { egl_headless: EglHeadlessSchema },
    LookingGlass { looking_glass: LookingGlassSchema },
    Gtk { gtk: GtkSchema },
    Sdl { sdl: SdlSchema },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VncSchema {
    pub port: u16,
    pub listen: String,
    #[serde(default)]
    pub gl_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,
    #[serde(default)]
    pub password_auth: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpiceSchema {
    pub port: u16,
    pub listen: String,
    pub disable_ticketing: bool,
    #[serde(default)]
    pub gl_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_ciphers: Option<String>,
    #[serde(default)]
    pub seamless_migration: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EglHeadlessSchema {}

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
