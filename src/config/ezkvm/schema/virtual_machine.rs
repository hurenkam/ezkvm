use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::audio_device::AudioDeviceSchema;
use crate::config::ezkvm::schema::cpu::CpuSchema;
use crate::config::ezkvm::schema::guest_agent::GuestAgentSchema;
use crate::config::ezkvm::schema::memory::MemorySchema;
use crate::config::ezkvm::schema::rawargs::RawArgsSchema;
use crate::config::ezkvm::schema::tpm::TpmSchema;
use crate::config::ezkvm::schema::{BootSchema, DeviceSchema, MachineSchema};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct VirtualMachineSchema {
    machine: MachineSchema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cpu: Option<CpuSchema>,
    memory: MemorySchema,
    #[serde(default)]
    boot: BootSchema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    smbios_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vmgenid: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    tpm: Option<TpmSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    audio_device: Option<AudioDeviceSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw_args: Option<RawArgsSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guest_agent: Option<GuestAgentSchema>,
    devices: Vec<DeviceSchema>,
}
