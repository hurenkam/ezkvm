use crate::config_format::{ImportError, ImportOptions, Importer, LibvirtImporter, RuntimeConfig};

impl Importer for LibvirtImporter {
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError> {
        let source_path = match args {
            ImportOptions::Libvirt { vm } => vm,
            _ => return Err(ImportError::InvalidFormat),
        };

        let _source_path = source_path;
        Err(ImportError::UnsupportedImporter("libvirt".to_string()))
    }
}
