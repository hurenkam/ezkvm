//! libvirt XML importer placeholder.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

use crate::config_format::{ImportError, RuntimeModelLoader};
use crate::runtime_model::RuntimeModel;

/// Arguments required to import a libvirt XML file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibvirtInputArgs {
    /// Path to the libvirt XML file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

/// Imports libvirt XML into the canonical runtime model.
#[allow(dead_code)] // TODO: wire to CLI
pub struct LibvirtLoader;

impl RuntimeModelLoader for LibvirtLoader {
    type Args = LibvirtInputArgs;
    type Error = ImportError;

    fn load(&self, _args: Self::Args) -> Result<RuntimeModel, Self::Error> {
        Err(ImportError::UnsupportedImporter("libvirt".to_string()))
    }
}
