//! Marshaler stage: `EzkvmConfigSchema` → YAML text.

use crate::config_format::{ezkvm::EzkvmConfigSchema, stages::Marshaler};

/// Marshals an `EzkvmConfigSchema` into ezkvm YAML text.
pub struct EzkvmMarshaler;

impl Marshaler for EzkvmMarshaler {
    type Schema = EzkvmConfigSchema;
    type Error = String;

    fn marshal(&self, schema: &EzkvmConfigSchema) -> Result<String, String> {
        serde_yaml::to_string(schema).map_err(|e| format!("Failed to marshal schema: {e}"))
    }
}
