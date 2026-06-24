//! QEMU command-file importer.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/qemu/

use crate::config_format::qemu_cmd::{QemuParser, QemuRuntimeBuilder};
use crate::config_format::stages::{Parser, RuntimeBuilder};
use crate::config_format::{ImportError, ImportOptions, Importer, QemuImporter};
use crate::runtime_model::RuntimeModel;

impl Importer for QemuImporter {
    /// Imports a QEMU command-file into the canonical runtime model.
    fn import(&self, args: ImportOptions) -> Result<RuntimeModel, ImportError> {
        let source_path = match args {
            ImportOptions::Qemu { vm } => vm,
            _ => return Err(ImportError::InvalidFormat),
        };

        let source_text = std::fs::read_to_string(&source_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", source_path, e)))?;

        let schema = QemuParser
            .parse(&source_text)
            .map_err(ImportError::ImportFailed)?;

        QemuRuntimeBuilder
            .build(schema)
            .map_err(ImportError::ImportFailed)
    }
}

#[cfg(test)]
mod tests {
    use crate::config_format::{ImportOptions, Importer, QemuImporter};

    #[test]
    fn imports_basic_qemu_command_file() {
        let path = "/tmp/ezkvm-test-qemu-import-basic.cmd";
        std::fs::write(
            path,
            "qemu-system-x86_64 -name vm1 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1",
        )
        .expect("test qemu command should be written");

        let runtime = QemuImporter
            .import(ImportOptions::Qemu {
                vm: path.to_string(),
            })
            .expect("import should succeed");

        assert_eq!(runtime.name(), "vm1");
        assert_eq!(runtime.memory().qemu_args(), vec!["-m", "2048M"]);
    }
}
