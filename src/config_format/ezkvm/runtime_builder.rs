//! RuntimeBuilder stage: `EzkvmConfigSchema` → `RuntimeModel`.

use crate::{
    config_format::{
        ezkvm::{EzkvmConfigSchema, builder::EzkvmRuntimeModelBuilder},
        stages::RuntimeBuilder,
    },
    runtime_model::RuntimeModel,
};

pub use crate::config_format::ezkvm::builder::EzkvmHostSchema;

/// Builds a `RuntimeModel` from an `EzkvmConfigSchema`.
///
/// `host_path` and `vm_name` provide the host context required during assembly.
pub struct EzkvmRuntimeBuilder {
    pub host_path: String,
    pub vm_name: String,
}

impl RuntimeBuilder for EzkvmRuntimeBuilder {
    type Schema = EzkvmConfigSchema;

    fn build(&self, schema: EzkvmConfigSchema) -> Result<RuntimeModel, String> {
        EzkvmRuntimeModelBuilder::new()
            .with_host_config(EzkvmHostSchema::new(self.host_path.clone()))
            .with_vm_config(schema)
            .with_name(self.vm_name.clone())
            .build()
    }
}
