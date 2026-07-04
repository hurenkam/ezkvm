use crate::{
    config_format::{
        ImportError, Parser, RuntimeBuilder, RuntimeModelLoader,
        qemu_cmd::{QemuRuntimeBuilder, parser::QemuParser},
    },
    runtime_model::RuntimeModel,
};

/// Arguments required to import a QEMU command file.
#[allow(dead_code)] // TODO: wire to CLI
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QemuInputArgs {
    /// Path to the QEMU command file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

/// Imports QEMU command files into the canonical runtime model.
#[allow(dead_code)] // TODO: wire to CLI
pub struct QemuLoader;

impl RuntimeModelLoader for QemuLoader {
    type Args = QemuInputArgs;
    type Error = ImportError;

    fn load(&self, args: Self::Args) -> Result<RuntimeModel, Self::Error> {
        let source_path = args.input_vm;
        let source_text = std::fs::read_to_string(&source_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", source_path, e)))?;

        let schema = QemuParser
            .parse(&source_text)
            .map_err(ImportError::ImportFailed)?;

        QemuRuntimeBuilder::default()
            .with_schema(schema)
            .build()
            .map_err(ImportError::ImportFailed)
    }
}

#[cfg(test)]
mod tests {

    use crate::config_format::{
        RuntimeModelLoader,
        qemu_cmd::loader::{QemuInputArgs, QemuLoader},
    };

    #[test]
    fn imports_basic_qemu_command_file() {
        let path = "/tmp/ezkvm-test-qemu-import-basic.cmd";
        std::fs::write(
            path,
            "qemu-system-x86_64 -name vm1 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1",
        )
        .expect("test qemu command should be written");

        let runtime = QemuLoader
            .load(QemuInputArgs {
                input_vm: path.to_string(),
            })
            .expect("import should succeed");

        assert_eq!(runtime.name(), "vm1");
        assert_eq!(runtime.memory().size() / 1024 / 1024, 2048);
    }
}
