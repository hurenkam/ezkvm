//! Schema types for qemu command-file import/export.

/// Typed subset extracted from qemu command arguments.
#[derive(Debug, Clone, Default)]
pub struct QemuKnownFields {
    pub name: Option<String>,
    pub machine: Option<String>,
    pub memory_mb: Option<u64>,
    pub cpu_model: Option<String>,
    pub cores: Option<u8>,
    pub sockets: Option<u8>,
    pub threads: Option<u8>,
}

/// Parsed qemu command schema.
///
/// `args` keeps the full command argument vector (excluding executable) so unknown
/// flags are preserved in parse/marshal roundtrips.
#[derive(Debug, Clone)]
pub struct QemuCommandSchema {
    pub executable: String,
    pub args: Vec<String>,
    pub known: QemuKnownFields,
}

impl QemuCommandSchema {
    pub fn new(executable: String, args: Vec<String>, known: QemuKnownFields) -> Self {
        Self {
            executable,
            args,
            known,
        }
    }
}
