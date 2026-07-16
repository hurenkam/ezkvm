use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::cpu::CpuSchema;
use crate::config::ezkvm::schema::guest_agent::GuestAgent;
use crate::config::ezkvm::schema::memory::MemorySchema;
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

    // Note: These belong in host, and have been moved there.
    // Instead, there should be a vga/gpu section here.

    //#[serde(default, skip_serializing_if = "Option::is_none")]
    //display: Option<DisplaySchema>,
    //#[serde(default, skip_serializing_if = "Option::is_none")]
    //audio: Option<AudioSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guest_agent: Option<GuestAgent>,
    devices: Vec<DeviceSchema>,
}
