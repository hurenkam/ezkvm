use crate::runtime::{RootDevice, Runtime, RuntimeBuilder};

use std::{any::TypeId, collections::HashMap};

#[derive(Debug, thiserror::Error)]
pub enum QemuConversionError {
    #[error("no qemu handler for device '{name}'")]
    NoHandler { name: String },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QemuSchema {}

type QemuSchemaHandler = fn(&mut QemuSchemaBuilder, &dyn RootDevice) -> Result<(), QemuConversionError>;

#[allow(dead_code)]
pub trait QemuDeviceHandler {
    fn handlers() -> HashMap<TypeId, QemuSchemaHandler>;
}

#[derive(Debug)]
pub struct QemuSchemaBuilder {
    schema: QemuSchema,
    handlers: HashMap<TypeId, QemuSchemaHandler>,
}
impl QemuSchemaBuilder {
    pub fn new(handlers: HashMap<TypeId, QemuSchemaHandler>) -> Self {
        QemuSchemaBuilder {
            schema: QemuSchema::default(),
            handlers,
        }
    }

    pub fn build(self) -> Result<QemuSchema, QemuConversionError> {
        Ok(self.schema)
    }

    pub fn with_device(&mut self, device: &dyn RootDevice) -> Result<(), QemuConversionError> {
        let device_type = device.get_type();
        if let Some(handler) = self.handlers.get(&device_type) {
            handler(self, device)
        } else {
            Err(QemuConversionError::NoHandler { name: device.get_name().to_string() })
        }
    }
}

impl TryFrom<Runtime> for QemuSchema {
    type Error = QemuConversionError;

    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let handlers = HashMap::new();
        let mut builder = QemuSchemaBuilder::new(handlers);

        for device in value.root_devices() {
            builder.with_device(device.as_ref())?;
        }

        builder.build()
    }
}

impl TryFrom<QemuSchema> for Runtime {
    type Error = QemuConversionError;

    fn try_from(_schema: QemuSchema) -> Result<Self, Self::Error> {
        let builder = RuntimeBuilder::new();

        builder.build().map_err(|_| unreachable!("RuntimeBuilder::build() never fails"))
    }
}

#[allow(dead_code)]
pub fn build_schema(runtime: Runtime) -> Result<QemuSchema, QemuConversionError> {
    QemuSchema::try_from(runtime)
}

#[allow(dead_code)]
pub fn build_runtime(schema: QemuSchema) -> Result<Runtime, QemuConversionError> {
    Runtime::try_from(schema)
}
