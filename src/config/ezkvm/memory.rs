use std::{any::TypeId, collections::HashMap};

use derive_getters::Getters;
use derive_new::new;

use crate::{
    config::{EzkvmDeviceHandler, EzkvmSchemaBuilder},
    runtime::{Memory, RootDevice},
};

pub struct EzkvmMemoryHandler;
impl EzkvmDeviceHandler for EzkvmMemoryHandler {
    fn handlers() -> HashMap<TypeId, fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>>
    {
        HashMap::from([(
            TypeId::of::<Memory>(),
            memory_handler as fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>,
        )])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default, Getters, new)]
pub struct EzkvmMemory {
    size: usize,
}

impl From<Memory> for EzkvmMemory {
    fn from(memory: Memory) -> Self {
        EzkvmMemory {
            size: *memory.size(),
        }
    }
}

impl From<EzkvmMemory> for Memory {
    fn from(memory: EzkvmMemory) -> Self {
        Memory::new(memory.size)
    }
}

fn memory_handler(builder: &mut EzkvmSchemaBuilder, device: &dyn RootDevice) -> Result<(), ()> {
    if let Some(memory) = device.as_any().downcast_ref::<Memory>() {
        builder.vm_schema.memory = Some((*memory).into());
        Ok(())
    } else {
        Err(())
    }
}
