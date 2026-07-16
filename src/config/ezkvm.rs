mod chipset;
mod display;
mod guest_agent;
mod memory;
mod resources;
mod runtime_types;
mod schema;
mod tpm;
mod types;
pub use schema::EzkvmConfigSchema;

use crate::{
    config::ezkvm::{
        chipset::{Chipset, EzkvmChipsetHandler}, memory::{EzkvmMemoryHandler, Memory},
    }, runtime::{BusDevice, BusDeviceRegistry, RootDevice, Runtime, RuntimeBuilder},
};
use std::{any::TypeId, collections::HashMap, fmt::Debug, sync::{Arc, Mutex}};

pub trait RootDeviceHandler: Debug + Send + Sync + 'static {
    fn handle(&self, builder: &mut EzkvmSchemaBuilder, device: &dyn RootDevice) -> Result<(), ()>;
}

pub trait BusDeviceHandler: Debug + Send + Sync + 'static {
    fn handle(&self, builder: &mut EzkvmSchemaBuilder, device: &dyn BusDevice) -> Result<(), ()>;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EzkvmSchemaBuilder {
    root_device_handlers: HashMap<TypeId, Arc<dyn RootDeviceHandler>>,
    bus_device_handlers: HashMap<TypeId, Arc<dyn BusDeviceHandler>>,
    memory: Option<memory::Memory>,
    chipset: Option<chipset::Chipset>,
    bus_devices: Arc<Mutex<BusDeviceRegistry>>
}
impl EzkvmSchemaBuilder {
    pub fn new() -> Self {
        EzkvmSchemaBuilder {
            root_device_handlers: HashMap::from([
                (TypeId::of::<crate::runtime::Memory>(), Arc::new(EzkvmMemoryHandler) as Arc<dyn RootDeviceHandler>),
                (TypeId::of::<crate::runtime::Chipset>(), Arc::new(EzkvmChipsetHandler) as Arc<dyn RootDeviceHandler>),
            ]),
            bus_device_handlers: HashMap::from([
            ]),
            memory: None,
            chipset: None,
            bus_devices: Arc::new(Mutex::new(BusDeviceRegistry(HashMap::new()))),
        }
    }

    pub fn build(self) -> Result<EzkvmConfigSchema, ()> {
        Ok(EzkvmConfigSchema {
            virtual_machine: schema::VirtualMachine {
                memory: self.memory.ok_or(())?,
                machine: schema::Machine {
                    chipset: self.chipset.ok_or(())?,
                    version: None,
                },
                cpu: None,
                boot: schema::Boot::default(),
                smbios_uuid: None,
                vmgenid: None,
                tpm: None,
                display: None,
                audio: None,
                guest_agent: None,
                devices: vec![],
            },
            metadata: schema::Metadata {
                schema_version: "1.0".to_string(),
                vm_name: "vm".to_string(),
            },
            host: schema::HostSchema {
                display: None,
                audio: None,
                resources: vec![],
            },
        })
    }

    pub fn with_device(&mut self, device: &dyn RootDevice) -> Result<(), ()> {
        let device_type = device.get_type();
        if let Some(handler) = self.root_device_handlers.get(&device_type) {
            handler.clone().handle(self, device)
        } else {
            println!(
                "No ezkvm handler for device '{}', type: {:?}",
                device.get_name(),
                device_type
            );
            Err(())
        }
    }

    pub fn with_memory(&mut self, memory: Memory) -> &mut Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_chipset(&mut self, chipset: Chipset) -> &mut Self {
        self.chipset = Some(chipset);
        self
    }
}

impl TryFrom<Runtime> for EzkvmConfigSchema {
    type Error = ();

    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let mut builder = EzkvmSchemaBuilder::new();

        for device in value.root_devices() {
            builder.with_device(device.as_ref())?;
        }

        builder.build()
    }
}

impl TryFrom<EzkvmConfigSchema> for Runtime {
    type Error = ();

    fn try_from(value: EzkvmConfigSchema) -> Result<Self, Self::Error> {
        let builder = RuntimeBuilder::new();
        let bus_devices = builder.bus_devices();

        builder.with_memory(value.virtual_machine.memory.into());
        match value.virtual_machine.machine.chipset {
            Chipset::Q35 { q35: _ } => builder.with_chipset(
                crate::runtime::Chipset::Q35(crate::runtime::Q35Chipset::new(bus_devices.clone())),
            ),
            Chipset::I440FX { i440fx: _ } => builder.with_chipset(
                crate::runtime::Chipset::I440FX,
            ),
        };

        builder.build()
    }
}
