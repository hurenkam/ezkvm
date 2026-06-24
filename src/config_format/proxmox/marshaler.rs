//! Marshaler stage: `ProxmoxConfigSchema` → `.conf` text.

use crate::config_format::{proxmox::schema::ProxmoxConfigSchema, stages::Marshaler};

/// Marshals a `ProxmoxConfigSchema` into Proxmox `.conf` text.
pub struct ProxmoxMarshaler;

impl Marshaler for ProxmoxMarshaler {
    type Schema = ProxmoxConfigSchema;
    type Error = String;

    fn marshal(&self, schema: &ProxmoxConfigSchema) -> Result<String, String> {
        Ok(schema.render())
    }
}
