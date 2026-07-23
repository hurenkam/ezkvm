mod conf;
mod error;
mod importer;
mod parser;
mod storage;

pub use conf::{ProxmoxVmConf, ProxmoxDiskConf, ProxmoxAudioConf,
               ProxmoxEfiDiskConf, ProxmoxHostPciConf, ProxmoxTpmConf};
pub use error::{ProxmoxParseError, ProxmoxImportError};
pub use importer::ProxmoxImporter;
pub use storage::{ProxmoxStorageConf, ProxmoxStorageEntry, StorageResolver, StorageType};

use crate::runtime::{RootDevice, Runtime, RuntimeBuilder};

use std::{any::TypeId, collections::HashMap};

#[derive(Debug, thiserror::Error)]
pub enum ProxmoxConversionError {
    #[error("no proxmox handler for device '{name}'")]
    NoHandler { name: String },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProxmoxVmSchema {}
#[derive(Debug, Clone, Copy, Default)]
pub struct ProxmoxHostSchema {}

type ProxmoxSchemaHandler =
    fn(&mut ProxmoxSchemaBuilder, &dyn RootDevice) -> Result<(), ProxmoxConversionError>;

#[allow(dead_code)]
pub trait ProxmoxDeviceHandler {
    fn handlers() -> HashMap<TypeId, ProxmoxSchemaHandler>;
}

#[derive(Debug)]
pub struct ProxmoxSchemaBuilder {
    schema: ProxmoxVmSchema,
    handlers: HashMap<TypeId, ProxmoxSchemaHandler>,
}

impl ProxmoxSchemaBuilder {
    pub fn new(handlers: HashMap<TypeId, ProxmoxSchemaHandler>) -> Self {
        ProxmoxSchemaBuilder {
            schema: ProxmoxVmSchema::default(),
            handlers,
        }
    }

    pub fn build(self) -> Result<ProxmoxVmSchema, ProxmoxConversionError> {
        Ok(self.schema)
    }

    pub fn with_device(&mut self, device: &dyn RootDevice) -> Result<(), ProxmoxConversionError> {
        let device_type = device.get_type();
        if let Some(handler) = self.handlers.get(&device_type) {
            handler(self, device)
        } else {
            Err(ProxmoxConversionError::NoHandler { name: device.get_name().to_string() })
        }
    }
}

impl TryFrom<(Runtime, ProxmoxHostSchema)> for ProxmoxVmSchema {
    type Error = ProxmoxConversionError;

    fn try_from(value: (Runtime, ProxmoxHostSchema)) -> Result<Self, Self::Error> {
        let (runtime, _host_schema) = value;
        let handlers = HashMap::new();
        let mut builder = ProxmoxSchemaBuilder::new(handlers);

        for device in runtime.root_devices() {
            builder.with_device(device.as_ref())?;
        }

        builder.build()
    }
}

impl TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime {
    type Error = ProxmoxConversionError;

    fn try_from(value: (ProxmoxVmSchema, ProxmoxHostSchema)) -> Result<Self, Self::Error> {
        let (_vm_schema, _host_schema) = value;
        let builder = RuntimeBuilder::new();

        builder.build().map_err(|_| unreachable!("RuntimeBuilder::build() never fails"))
    }
}
