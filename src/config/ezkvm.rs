mod runtime;
mod schema;
mod file;

#[allow(unused_imports)]
pub use {schema::ConfigSchema, file::ConfigFileStore};

/*
use crate::{
    config::ezkvm::{
        runtime::{EzkvmChipsetHandler, EzkvmMemoryHandler},
        schema::{ChipsetSchema, EZKVM_CONFIG_SCHEMA_VERSION, MemorySchema},
    },
    runtime::{RootDevice, Runtime, RuntimeBuilder},
};

use std::{any::TypeId, collections::HashMap, fmt::Debug, sync::Arc};

pub trait RootDeviceHandler: Debug + Send + Sync + 'static {
    fn handle(&self, builder: &mut SchemaBuilder, device: &dyn RootDevice) -> Result<(), ()>;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SchemaBuilder {
    root_device_handlers: HashMap<TypeId, Arc<dyn RootDeviceHandler>>,
    //bus_device_handlers: HashMap<TypeId, Arc<dyn BusDeviceHandler>>,
    memory: Option<MemorySchema>,
    chipset: Option<ChipsetSchema>,
    //bus_devices: Arc<Mutex<BusDeviceRegistry>>,
}
impl SchemaBuilder {
    pub fn new() -> Self {
        SchemaBuilder {
            root_device_handlers: HashMap::from([
                (
                    TypeId::of::<crate::runtime::Memory>(),
                    Arc::new(EzkvmMemoryHandler) as Arc<dyn RootDeviceHandler>,
                ),
                (
                    TypeId::of::<crate::runtime::Chipset>(),
                    Arc::new(EzkvmChipsetHandler) as Arc<dyn RootDeviceHandler>,
                ),
            ]),
            //bus_device_handlers: HashMap::from([]),
            memory: None,
            chipset: None,
        }
    }

    pub fn build(self) -> Result<ConfigSchema, ()> {
        let metadata =
            schema::MetadataSchema::new(EZKVM_CONFIG_SCHEMA_VERSION.to_string(), "vm".to_string());

        let display = None;
        let audio = None;
        let resources = vec![];
        let host = schema::HostSchema::new(display, audio, resources);

        let chipset = self.chipset.ok_or(())?;
        let version = None;
        let machine = schema::MachineSchema::new(chipset, version);

        let cpu = None;
        let memory = self.memory.ok_or(())?;
        let boot = schema::BootSchema::default();
        let smbios_uuid = None;
        let vmgenid = None;
        let tpm = None;
        let audio_device = None;
        let raw_args = None;
        let guest_agent = None;
        let devices = vec![];
        let virtual_machine = schema::VirtualMachineSchema::new(
            machine,
            cpu,
            memory,
            boot,
            smbios_uuid,
            vmgenid,
            tpm,
            audio_device,
            raw_args,
            guest_agent,
            devices,
        );
        Ok(ConfigSchema::new(metadata, host, virtual_machine))
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

    pub fn with_memory(&mut self, memory: MemorySchema) -> &mut Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_chipset(&mut self, chipset: ChipsetSchema) -> &mut Self {
        self.chipset = Some(chipset);
        self
    }
}
 */