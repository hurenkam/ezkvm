//! RuntimeBuilder stage: `ProxmoxConfigSchema` → `RuntimeModel`.

use crate::{
    config_format::{
        proxmox::{schema::ProxmoxConfigSchema, storage_resolver::ProxmoxStorageConfig},
        stages::RuntimeBuilder,
    },
    runtime_model::{RuntimeModel, RuntimeModelBuilder},
};

/// Builds a `RuntimeModel` from a `ProxmoxConfigSchema`.
///
/// `storage_config` provides the storage token/path resolver required during assembly.
pub struct ProxmoxRuntimeBuilder {
    pub storage_config: ProxmoxStorageConfig,
}

impl RuntimeBuilder for ProxmoxRuntimeBuilder {
    type Schema = ProxmoxConfigSchema;

    fn build(&self, schema: ProxmoxConfigSchema) -> Result<RuntimeModel, String> {
        RuntimeModelBuilder::build_from_proxmox_config(&schema, &self.storage_config)
    }
}
