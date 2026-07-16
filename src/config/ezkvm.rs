mod compact_yaml;
mod runtime;
mod schema;
mod store;

#[allow(unused_imports)]
pub use {schema::ConfigSchema, store::ConfigFileStore};

use crate::{
    config::ezkvm::{
        runtime::{EzkvmChipsetHandler, EzkvmMemoryHandler},
        schema::{ChipsetSchema, EZKVM_CONFIG_SCHEMA_VERSION, MemorySchema},
    },
    runtime::{BusDevice, BusDeviceRegistry, RootDevice, Runtime, RuntimeBuilder},
};
use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

pub trait RootDeviceHandler: Debug + Send + Sync + 'static {
    fn handle(&self, builder: &mut SchemaBuilder, device: &dyn RootDevice) -> Result<(), ()>;
}

#[allow(dead_code)]
pub trait BusDeviceHandler: Debug + Send + Sync + 'static {
    fn handle(&self, builder: &mut SchemaBuilder, device: &dyn BusDevice) -> Result<(), ()>;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SchemaBuilder {
    root_device_handlers: HashMap<TypeId, Arc<dyn RootDeviceHandler>>,
    bus_device_handlers: HashMap<TypeId, Arc<dyn BusDeviceHandler>>,
    memory: Option<MemorySchema>,
    chipset: Option<ChipsetSchema>,
    bus_devices: Arc<Mutex<BusDeviceRegistry>>,
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
            bus_device_handlers: HashMap::from([]),
            memory: None,
            chipset: None,
            bus_devices: Arc::new(Mutex::new(BusDeviceRegistry(HashMap::new()))),
        }
    }

    pub fn build(self) -> Result<ConfigSchema, ()> {
        let metadata =
            schema::Metadata::new(EZKVM_CONFIG_SCHEMA_VERSION.to_string(), "vm".to_string());

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
        //let display = None;
        //let audio = None;
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
            //display,
            //audio,
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

impl TryFrom<Runtime> for ConfigSchema {
    type Error = ();

    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let mut builder = SchemaBuilder::new();

        for device in value.root_devices() {
            builder.with_device(device.as_ref())?;
        }

        builder.build()
    }
}

impl TryFrom<ConfigSchema> for Runtime {
    type Error = ();

    fn try_from(value: ConfigSchema) -> Result<Self, Self::Error> {
        let builder = RuntimeBuilder::new();
        let bus_devices = builder.bus_devices();

        builder.with_memory(value.virtual_machine().memory().into());
        match &value.virtual_machine().machine().chipset() {
            ChipsetSchema::Q35 { q35: _ } => builder.with_chipset(crate::runtime::Chipset::Q35(
                crate::runtime::Q35Chipset::new(bus_devices.clone()),
            )),
            ChipsetSchema::I440FX { i440fx: _ } => {
                builder.with_chipset(crate::runtime::Chipset::I440FX)
            }
        };

        builder.build()
    }
}
