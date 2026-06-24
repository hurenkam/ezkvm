//! Parser stage: Proxmox `.conf` text → `ProxmoxConfigSchema`.

use crate::config_format::{proxmox::schema::ProxmoxConfigSchema, stages::Parser};

/// Parses Proxmox `.conf` text into `ProxmoxConfigSchema`.
pub struct ProxmoxParser;

impl Parser for ProxmoxParser {
    type Schema = ProxmoxConfigSchema;
    type Error = String;

    fn parse(&self, source: &str) -> Result<ProxmoxConfigSchema, String> {
        ProxmoxConfigSchema::parse(source).map_err(|e| e.to_string())
    }
}
