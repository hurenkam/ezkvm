mod chipset;
mod memory;

use crate::runtime::{RootDevice, Runtime, RuntimeBuilder};
use chipset::{EzkvmChipset, EzkvmChipsetHandler};
use memory::{EzkvmMemory, EzkvmMemoryHandler};

use std::{any::TypeId, collections::HashMap};

#[derive(Debug, Clone, Copy, Default)]
pub struct EzkvmHostSchema {}
#[derive(Debug, Clone, Copy, Default)]
pub struct EzkvmVmSchema {
    memory: Option<EzkvmMemory>,
    chipset: Option<EzkvmChipset>,
}

type EzkvmSchemaHandler = fn(&mut EzkvmSchemaBuilder, &dyn RootDevice) -> Result<(), ()>;
pub trait EzkvmDeviceHandler {
    fn handlers() -> HashMap<TypeId, EzkvmSchemaHandler>;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EzkvmSchemaBuilder {
    host_schema: EzkvmHostSchema,
    vm_schema: EzkvmVmSchema,
    handlers: HashMap<TypeId, EzkvmSchemaHandler>,
}
impl EzkvmSchemaBuilder {
    pub fn new(
        host_schema: EzkvmHostSchema,
        handlers: HashMap<TypeId, EzkvmSchemaHandler>,
    ) -> Self {
        EzkvmSchemaBuilder {
            host_schema,
            vm_schema: EzkvmVmSchema::default(),
            handlers,
        }
    }

    pub fn build(self) -> Result<EzkvmVmSchema, ()> {
        Ok(self.vm_schema)
    }

    pub fn with_device(&mut self, device: &dyn RootDevice) -> Result<(), ()> {
        let device_type = device.get_type();
        if let Some(handler) = self.handlers.get(&device_type) {
            handler(self, device)
        } else {
            println!("No ezkvm handler for device type: {:?}", device_type);
            Err(())
        }
    }
}

impl TryFrom<(Runtime, EzkvmHostSchema)> for EzkvmVmSchema {
    type Error = ();

    fn try_from(value: (Runtime, EzkvmHostSchema)) -> Result<Self, Self::Error> {
        let (device_tree, host_schema) = value;
        let mut handlers = HashMap::new();
        handlers.extend(EzkvmMemoryHandler::handlers());
        handlers.extend(EzkvmChipsetHandler::handlers());
        let mut builder = EzkvmSchemaBuilder::new(host_schema, handlers);

        for device in device_tree.root_devices() {
            builder.with_device(device.as_ref())?;
        }

        builder.build()
    }
}

impl TryFrom<(EzkvmVmSchema, EzkvmHostSchema)> for Runtime {
    type Error = ();

    fn try_from(value: (EzkvmVmSchema, EzkvmHostSchema)) -> Result<Self, Self::Error> {
        let (vm_schema, _host_schema) = value;
        let builder = RuntimeBuilder::new();

        if let Some(memory) = vm_schema.memory {
            builder.with_memory(memory.into());
        }

        builder.build()
    }
}

pub fn build_schema(runtime: Runtime, host_schema: EzkvmHostSchema) -> Result<EzkvmVmSchema, ()> {
    EzkvmVmSchema::try_from((runtime, host_schema))
}

#[allow(dead_code)]
pub fn build_runtime(
    vm_schema: EzkvmVmSchema,
    host_schema: EzkvmHostSchema,
) -> Result<Runtime, ()> {
    Runtime::try_from((vm_schema, host_schema))
}
