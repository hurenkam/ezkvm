//! SchemaBuilder stage: `RuntimeModel` → `EzkvmConfigSchema`.

use crate::{
    config_format::{
        ezkvm::{
            EzkvmConfigSchema, renderer::EzkvmRuntimeModelRenderer,
            runtime_builder::EzkvmHostSchema,
        },
        stages::SchemaBuilder,
    },
    runtime_model::RuntimeModel,
};

/// Builds an `EzkvmConfigSchema` from a `RuntimeModel`.
///
/// `host_path` provides the host context required during rendering.
pub struct EzkvmSchemaBuilder {
    pub host_path: String,
}

impl SchemaBuilder for EzkvmSchemaBuilder {
    type Schema = EzkvmConfigSchema;

    fn build(&self, runtime: RuntimeModel) -> Result<EzkvmConfigSchema, String> {
        EzkvmRuntimeModelRenderer::new()
            .with_runtime_model(runtime)
            .with_host_schema(EzkvmHostSchema::new(self.host_path.clone()))
            .render()
    }
}
