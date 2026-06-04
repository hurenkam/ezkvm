use crate::config_format::{ImportError, ImportOptions, Importer, QemuImporter, RuntimeConfig};

impl Importer for QemuImporter {
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError> {
        let source_path = match args {
            ImportOptions::Qemu { vm } => vm,
            _ => return Err(ImportError::InvalidFormat),
        };

        let _source_path = source_path;
        Err(ImportError::UnsupportedImporter("qemu".to_string()))
    }
}
