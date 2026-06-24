//! SchemaBuilder stage: `RuntimeModel` -> `QemuCommandSchema`.

use crate::{
    config_format::{
        qemu_cmd::{
            parser::parse_known_fields,
            schema::{QemuCommandSchema, QemuKnownFields},
        },
        stages::SchemaBuilder,
    },
    runtime_model::{Chipset, RuntimeModel},
};

/// Builds qemu command schema from runtime model.
pub struct QemuSchemaBuilder;

impl SchemaBuilder for QemuSchemaBuilder {
    type Schema = QemuCommandSchema;

    fn build(&self, runtime: RuntimeModel) -> Result<QemuCommandSchema, String> {
        let mut args: Vec<String> = Vec::new();

        args.push("-name".to_string());
        args.push(runtime.name().clone());

        args.extend(runtime.cpu().qemu_args());
        args.extend(runtime.memory().qemu_args());

        match runtime.chipset() {
            Chipset::Q35(_) => {
                args.push("-machine".to_string());
                args.push("type=q35".to_string());
            }
            Chipset::I440FX(_) => {
                args.push("-machine".to_string());
                args.push("type=i440fx".to_string());
            }
        }

        args.extend(runtime.boot().qemu_args());

        if let Some(tpm) = runtime.tpm() {
            args.extend(tpm.qemu_args(runtime.name()));
        }

        args.extend(runtime.busses().qemu_args());

        let mut known: QemuKnownFields = parse_known_fields(&args);
        known.name = Some(runtime.name().clone());

        Ok(QemuCommandSchema::new(
            "qemu-system-x86_64".to_string(),
            args,
            known,
        ))
    }
}
