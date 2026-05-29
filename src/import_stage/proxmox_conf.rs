//! Proxmox .conf file import adapter.
//!
//! Owns Proxmox-specific parsing and adaptation to canonical VM specification.

use std::ffi::OsStr;
use std::path::Path;

use crate::vm_spec::model::{VirtualMachine, CANONICAL_SCHEMA_VERSION};
use crate::vm_spec::model::{Cpu, Machine, Memory, Metadata, NetworkEntry, ResourceRef, StorageEntry, System};
use crate::vm_spec::CanonicalDocument;

use super::ImportRequest;

#[derive(Debug, thiserror::Error)]
pub enum ProxmoxConfImportError {
    #[error("expected a Proxmox .conf source name, got {source_name}")]
    InvalidSourceName { source_name: String },
    #[error("unsupported Proxmox machine value '{value}' in {source_name}")]
    UnsupportedMachineValue { source_name: String, value: String },
    #[error("invalid Proxmox {field} value '{value}' in {source_name}")]
    InvalidFieldValue {
        source_name: String,
        field: &'static str,
        value: String,
    },
    #[error("missing required Proxmox field '{field}' in {source_name}")]
    MissingRequiredField {
        source_name: String,
        field: &'static str,
    },
    #[error("malformed Proxmox line {line} in {source_name}: {content}")]
    MalformedLine {
        source_name: String,
        line: usize,
        content: String,
    },
}

#[derive(Debug, Default)]
pub struct ProxmoxConfImportStage;

impl ProxmoxConfImportStage {
    pub fn parse(request: ImportRequest<'_>) -> Result<CanonicalDocument, ProxmoxConfImportError> {
        let source_name = request.source_name.to_string_lossy().into_owned();

        validate_source_name(request.source_name)?;

        let mut vm_name: Option<String> = None;
        let mut machine: Option<Machine> = None;
        let mut cpu_model: Option<String> = None;
        let mut memory_min: Option<i64> = None;
        let mut storage: Vec<StorageEntry> = Vec::new();
        let mut network: Vec<NetworkEntry> = Vec::new();
        let mut resources: Vec<ResourceRef> = Vec::new();

        for (index, raw_line) in request.source_text.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') {
                break;
            }

            let Some((key, value)) = line.split_once(':') else {
                return Err(ProxmoxConfImportError::MalformedLine {
                    source_name: source_name.clone(),
                    line: line_number,
                    content: line.to_owned(),
                });
            };

            let key = key.trim();
            let value = value.trim();

            if value.is_empty() {
                return Err(ProxmoxConfImportError::InvalidFieldValue {
                    source_name: source_name.clone(),
                    field: "source line value",
                    value: line.to_owned(),
                });
            }

            match key {
                "name" => vm_name = Some(value.to_owned()),
                "machine" => machine = Some(parse_machine_value(value, &source_name)?),
                "cpu" => cpu_model = Some(value.to_owned()),
                "memory" => memory_min = Some(parse_memory_value(value, &source_name)?),
                _ if is_numeric_slot_key(key, "scsi") => {
                    storage.push(StorageEntry { id: key.to_owned() })
                }
                _ if is_numeric_slot_key(key, "net") => {
                    network.push(NetworkEntry { id: key.to_owned() })
                }
                _ if is_numeric_slot_key(key, "hostpci") || is_numeric_slot_key(key, "usb") => {
                    resources.push(ResourceRef { id: key.to_owned() })
                }
                _ => {}
            }
        }

        Ok(CanonicalDocument {
            metadata: Metadata {
                schema_version: CANONICAL_SCHEMA_VERSION.to_owned(),
                vm_name: required_string(vm_name, &source_name, "name")?,
            },
            virtual_machine: VirtualMachine {
                system: System {
                    machine: required_machine(machine, &source_name)?,
                    cpu: Cpu {
                        model: required_string(cpu_model, &source_name, "cpu")?,
                    },
                    memory: Memory {
                        min: required_memory(memory_min, &source_name)?,
                    },
                },
                storage,
                network,
                resources,
            },
        })
    }
}

fn is_proxmox_conf_source(source_name: &Path) -> bool {
    matches!(
        source_name.extension().and_then(OsStr::to_str),
        Some("conf")
    )
}

fn validate_source_name(source_name: &Path) -> Result<(), ProxmoxConfImportError> {
    let source_name_text = source_name.to_string_lossy().into_owned();

    let Some(file_name) = source_name.file_name().and_then(OsStr::to_str) else {
        return Err(ProxmoxConfImportError::InvalidSourceName {
            source_name: source_name_text,
        });
    };

    if !is_proxmox_conf_source(source_name) || file_name.is_empty() {
        return Err(ProxmoxConfImportError::InvalidSourceName {
            source_name: source_name_text,
        });
    }

    Ok(())
}

fn required_string(
    value: Option<String>,
    source_name: &str,
    field: &'static str,
) -> Result<String, ProxmoxConfImportError> {
    value.ok_or(ProxmoxConfImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field,
    })
}

fn required_machine(
    value: Option<Machine>,
    source_name: &str,
) -> Result<Machine, ProxmoxConfImportError> {
    value.ok_or(ProxmoxConfImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field: "machine",
    })
}

fn required_memory(value: Option<i64>, source_name: &str) -> Result<i64, ProxmoxConfImportError> {
    value.ok_or(ProxmoxConfImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field: "memory",
    })
}

pub fn parse_machine_value(value: &str, source_name: &str) -> Result<Machine, ProxmoxConfImportError> {
    let machine_token = value.split(',').next().unwrap_or(value).trim();

    let chipset = if machine_token == "q35"
        || machine_token == "pc-q35"
        || machine_token.starts_with("pc-q35-")
    {
        Some("q35")
    } else if machine_token == "i440fx"
        || machine_token == "pc-i440fx"
        || machine_token.starts_with("pc-i440fx-")
    {
        Some("i440fx")
    } else {
        None
    };

    let Some(chipset) = chipset else {
        return Err(ProxmoxConfImportError::UnsupportedMachineValue {
            source_name: source_name.to_owned(),
            value: machine_token.to_owned(),
        });
    };

    Ok(Machine {
        family: "pc".to_owned(),
        chipset: chipset.to_owned(),
    })
}

fn parse_memory_value(value: &str, source_name: &str) -> Result<i64, ProxmoxConfImportError> {
    let memory = value
        .parse::<i64>()
        .map_err(|_| ProxmoxConfImportError::InvalidFieldValue {
            source_name: source_name.to_owned(),
            field: "memory",
            value: value.to_owned(),
        })?;

    if memory < 0 {
        return Err(ProxmoxConfImportError::InvalidFieldValue {
            source_name: source_name.to_owned(),
            field: "memory",
            value: value.to_owned(),
        });
    }

    Ok(memory)
}

fn is_numeric_slot_key(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ProxmoxConfImportError, ProxmoxConfImportStage};
    use crate::import_stage::ImportRequest;

    #[test]
    fn proxmox_conf_import_stage_parses_corpus_shape() {
        let request = ImportRequest {
            source_text: include_str!("../../input/felucia/108.conf"),
            source_name: Path::new("/tmp/wakiza.conf"),
        };

        let document = ProxmoxConfImportStage::parse(request)
            .expect("corpus-shaped Proxmox config should parse");

        assert_eq!(document.metadata.schema_version, "1.0.0");
        assert_eq!(document.metadata.vm_name, "wakiza");
        assert_eq!(document.virtual_machine.system.machine.family, "pc");
        assert_eq!(document.virtual_machine.system.machine.chipset, "q35");
        assert_eq!(document.virtual_machine.system.cpu.model, "host");
        assert_eq!(document.virtual_machine.system.memory.min, 16384);
        assert_eq!(document.virtual_machine.storage.len(), 2);
        assert_eq!(document.virtual_machine.storage[0].id, "scsi0");
        assert_eq!(document.virtual_machine.storage[1].id, "scsi1");
        assert_eq!(document.virtual_machine.network.len(), 1);
        assert_eq!(document.virtual_machine.network[0].id, "net0");
        assert_eq!(document.virtual_machine.resources.len(), 2);
        assert_eq!(document.virtual_machine.resources[0].id, "hostpci0");
        assert_eq!(document.virtual_machine.resources[1].id, "usb0");
    }

    #[test]
    fn proxmox_conf_machine_parser_handles_pc_q35_shape() {
        let machine = super::parse_machine_value("pc-q35-8.1", "/tmp/108.conf")
            .expect("machine parser should accept pc-q35 shapes");

        assert_eq!(machine.family, "pc");
        assert_eq!(machine.chipset, "q35");
    }

    #[test]
    fn proxmox_conf_import_stage_rejects_malformed_source_name() {
        let request = ImportRequest {
            source_text: "name: wakiza\nmachine: pc-q35-8.1\ncpu: host\nmemory: 8192\n",
            source_name: Path::new("/tmp/wakiza.yaml"),
        };

        let err = ProxmoxConfImportStage::parse(request)
            .expect_err("non-.conf source names should be rejected");

        assert!(matches!(
            err,
            ProxmoxConfImportError::InvalidSourceName { .. }
        ));
    }
}
