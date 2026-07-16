use crate::config::ezkvm::schema::MemorySchema;
use crate::config::ezkvm::{RootDeviceHandler, SchemaBuilder};
use crate::runtime::{Memory, RootDevice};

impl From<&Memory> for MemorySchema {
    fn from(value: &Memory) -> Self {
        MemorySchema::new(*value.size(), None, false)
    }
}

impl From<&MemorySchema> for Memory {
    fn from(value: &MemorySchema) -> Self {
        Memory::new(*value.size())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EzkvmMemoryHandler;
impl RootDeviceHandler for EzkvmMemoryHandler {
    fn handle(&self, builder: &mut SchemaBuilder, device: &dyn RootDevice) -> Result<(), ()> {
        let memory = device
            .as_any()
            .downcast_ref::<crate::runtime::Memory>()
            .ok_or(())?;
        builder.with_memory(memory.into());
        Ok(())
    }
}
