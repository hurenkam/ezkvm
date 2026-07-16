use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::{
    config::{RootDeviceHandler, EzkvmSchemaBuilder},
    runtime::RootDevice,
};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, new)]
pub struct Memory {
    size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hugepages_kb: Option<usize>,
    #[serde(default)]
    numa_enabled: bool,
}
impl From<crate::runtime::Memory> for Memory {
    fn from(value: crate::runtime::Memory) -> Self {
        Memory {
            size: value.size().clone(),
            hugepages_kb: None,
            numa_enabled: false,
        }
    }
}
impl From<Memory> for crate::runtime::Memory {
    fn from(value: Memory) -> Self {
        crate::runtime::Memory::new(value.size)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EzkvmMemoryHandler;
impl RootDeviceHandler for EzkvmMemoryHandler {
    fn handle(&self, builder: &mut EzkvmSchemaBuilder, device: &dyn RootDevice) -> Result<(), ()> {
        let memory = device
            .as_any()
            .downcast_ref::<crate::runtime::Memory>()
            .ok_or(())?;
        builder.with_memory(memory.clone().into());
        Ok(())
    }
}
