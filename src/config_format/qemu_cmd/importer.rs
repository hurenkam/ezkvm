//! QEMU command-file importer placeholder.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

use crate::config_format::{ImportError, ImportOptions, Importer, QemuImporter};
use crate::runtime_model::RuntimeModel;

impl Importer for QemuImporter {
    /// Rejects QEMU command-file imports until a real importer is implemented.
    fn import(&self, args: ImportOptions) -> Result<RuntimeModel, ImportError> {
        let source_path = match args {
            ImportOptions::Qemu { vm } => vm,
            _ => return Err(ImportError::InvalidFormat),
        };

        let _source_path = source_path;
        Err(ImportError::UnsupportedImporter("qemu".to_string()))
    }
}
