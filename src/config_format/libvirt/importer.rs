//! libvirt XML importer placeholder.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/linux/

use crate::config_format::{ImportError, ImportOptions, Importer, LibvirtImporter};
use crate::runtime_model::RuntimeModel;

impl Importer for LibvirtImporter {
    /// Rejects libvirt XML imports until a real importer is implemented.
    fn import(&self, args: ImportOptions) -> Result<RuntimeModel, ImportError> {
        let source_path = match args {
            ImportOptions::Libvirt { vm } => vm,
            _ => return Err(ImportError::InvalidFormat),
        };

        let _source_path = source_path;
        Err(ImportError::UnsupportedImporter("libvirt".to_string()))
    }
}
