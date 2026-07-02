//! Marshaler stage: `QemuCommandSchema` → qemu command text.

use crate::config_format::{qemu_cmd::schema::QemuCommandSchema, stages::Marshaler};

/// Marshals a qemu command schema into deterministic shell command text.
#[allow(dead_code)] // TODO: wire to CLI
pub struct QemuMarshaler;

impl Marshaler for QemuMarshaler {
    type Schema = QemuCommandSchema;
    type Error = String;

    fn marshal(&self, schema: &QemuCommandSchema) -> Result<String, String> {
        let mut out = Vec::with_capacity(1 + schema.args.len());
        out.push(shell_escape(&schema.executable));
        out.extend(schema.args.iter().map(|arg| shell_escape(arg)));
        Ok(out.join(" "))
    }
}

#[allow(dead_code)] // TODO: wire to CLI
fn shell_escape(arg: &str) -> String {
    if arg.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | ',' | '=' | '+')
    }) {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use crate::config_format::{
        qemu_cmd::{parser::QemuParser, schema::QemuCommandSchema},
        stages::{Marshaler, Parser},
    };

    use super::QemuMarshaler;

    #[test]
    fn marshals_with_shell_escaping() {
        let schema = QemuCommandSchema::new(
            "qemu-system-x86_64".to_string(),
            vec!["-name".to_string(), "Debian 12".to_string()],
            Default::default(),
        );

        let text = QemuMarshaler
            .marshal(&schema)
            .expect("marshal should succeed");

        assert_eq!(text, "qemu-system-x86_64 -name 'Debian 12'");
    }

    #[test]
    fn parse_marshal_roundtrip_preserves_unknown_flags() {
        let source = "qemu-system-x86_64 -name vm1 -m 2048 -cpu host -smp 2,sockets=1,cores=2,threads=1 -global ICH9-LPC.disable_s3=1 -readconfig '/usr/share/qemu cfg.cfg'";

        let schema = QemuParser.parse(source).expect("parse should succeed");
        let text = QemuMarshaler
            .marshal(&schema)
            .expect("marshal should succeed");

        assert!(text.contains("-global ICH9-LPC.disable_s3=1"));
        assert!(text.contains("-readconfig '/usr/share/qemu cfg.cfg'"));
    }
}
