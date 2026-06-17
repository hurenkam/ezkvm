//! Proxmox VM configuration importer for translating `.conf` files into the runtime model.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use std::ffi::OsStr;
use std::path::Path;

use crate::config_format::{ImportError, ImportOptions, Importer, ProxmoxImporter, RuntimeConfig};
use crate::runtime_config::{
    Boot, Cpu, CpuModel, EZKVM_CONFIG_SCHEMA_VERSION, Machine, Memory, Metadata, VirtualMachine
};

/// Errors that can occur while parsing a Proxmox source file.
#[derive(Debug, thiserror::Error)]
enum ProxmoxImportError {
    /// The source path does not look like a valid Proxmox `.conf` file.
    #[error("expected a Proxmox .conf source name, got {source_name}")]
    InvalidSourceName { source_name: String },
    /// The machine value is not one of the supported chipset families.
    #[error("unsupported Proxmox machine value '{value}' in {source_name}")]
    UnsupportedMachineValue { source_name: String, value: String },
    /// A field value could not be parsed into the expected representation.
    #[error("invalid Proxmox {field} value '{value}' in {source_name}")]
    InvalidFieldValue {
        source_name: String,
        field: &'static str,
        value: String,
    },
    /// A required field was missing from the source file.
    #[error("missing required Proxmox field '{field}' in {source_name}")]
    MissingRequiredField {
        source_name: String,
        field: &'static str,
    },
    /// A line could not be parsed as a valid Proxmox key/value entry.
    #[error("malformed Proxmox line {line} in {source_name}: {content}")]
    MalformedLine {
        source_name: String,
        line: usize,
        content: String,
    },
}

/// Parses Proxmox source text into the canonical runtime configuration.
///
/// # Arguments
///
/// * `source_text` - Raw Proxmox configuration text.
/// * `source_name_path` - Path used to validate source naming conventions.
///
/// # Returns
///
/// A canonical runtime configuration or a Proxmox parsing error.
fn parse_source(
    source_text: &str,
    source_name_path: &Path,
) -> Result<RuntimeConfig, ProxmoxImportError> {
    let source_name = source_name_path.to_string_lossy().into_owned();

    validate_source_name(source_name_path)?;

    let mut vm_name: Option<String> = None;
    let mut machine: Option<Machine> = None;
    let mut cpu_model: Option<String> = None;
    let mut memory_min: Option<i64> = None;

    for (index, raw_line) in source_text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            break;
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(ProxmoxImportError::MalformedLine {
                source_name: source_name.clone(),
                line: line_number,
                content: line.to_owned(),
            });
        };

        let key = key.trim();
        let value = value.trim();

        if value.is_empty() {
            return Err(ProxmoxImportError::InvalidFieldValue {
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
            _ if is_numeric_slot_key(key, "scsi") => {}
            _ if is_numeric_slot_key(key, "net") => {}
            _ if is_numeric_slot_key(key, "hostpci") || is_numeric_slot_key(key, "usb") => {}
            _ => {}
        }
    }

    Ok(RuntimeConfig {
        metadata: Metadata {
            schema_version: EZKVM_CONFIG_SCHEMA_VERSION.to_owned(),
            vm_name: required_string(vm_name, &source_name, "name")?,
        },
        virtual_machine: VirtualMachine {
            machine: required_machine(machine, &source_name)?,
            cpu: cpu_model.map(|_| Cpu::new(CpuModel::Host, 1, 1, 1)),
            memory: Memory::megabytes(required_memory(memory_min, &source_name)?),
            boot: Boot::default(),
            tpm: None,
            devices: Vec::new(),
        },
        resources: Vec::new(),
    })
}

/// Returns true when the source path has a `.conf` extension.
fn is_proxmox_conf_source(source_name: &Path) -> bool {
    matches!(
        source_name.extension().and_then(OsStr::to_str),
        Some("conf")
    )
}

/// Validates that the source path is a Proxmox `.conf` file.
fn validate_source_name(source_name: &Path) -> Result<(), ProxmoxImportError> {
    let source_name_text = source_name.to_string_lossy().into_owned();

    let Some(file_name) = source_name.file_name().and_then(OsStr::to_str) else {
        return Err(ProxmoxImportError::InvalidSourceName {
            source_name: source_name_text,
        });
    };

    if !is_proxmox_conf_source(source_name) || file_name.is_empty() {
        return Err(ProxmoxImportError::InvalidSourceName {
            source_name: source_name_text,
        });
    }

    Ok(())
}

/// Returns a required string field or an error if it is missing.
fn required_string(
    value: Option<String>,
    source_name: &str,
    field: &'static str,
) -> Result<String, ProxmoxImportError> {
    value.ok_or(ProxmoxImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field,
    })
}

/// Returns a required machine definition or an error if it is missing.
fn required_machine(
    value: Option<Machine>,
    source_name: &str,
) -> Result<Machine, ProxmoxImportError> {
    value.ok_or(ProxmoxImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field: "machine",
    })
}

/// Returns a required memory value or an error if it is missing.
fn required_memory(value: Option<i64>, source_name: &str) -> Result<usize, ProxmoxImportError> {
    value
        .ok_or(ProxmoxImportError::MissingRequiredField {
            source_name: source_name.to_owned(),
            field: "memory",
        })
        .and_then(|size| {
            usize::try_from(size).map_err(|_| ProxmoxImportError::InvalidFieldValue {
                source_name: source_name.to_owned(),
                field: "memory",
                value: size.to_string(),
            })
        })
}

/// Parses a Proxmox machine token into the canonical machine model.
fn parse_machine_value(value: &str, source_name: &str) -> Result<Machine, ProxmoxImportError> {
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
        return Err(ProxmoxImportError::UnsupportedMachineValue {
            source_name: source_name.to_owned(),
            value: machine_token.to_owned(),
        });
    };

    Ok(Machine {
        family: "pc".to_owned(),
        chipset: chipset.to_owned(),
        version: None,
    })
}

/// Parses a non-negative memory size from a Proxmox source value.
fn parse_memory_value(value: &str, source_name: &str) -> Result<i64, ProxmoxImportError> {
    let memory = value
        .parse::<i64>()
        .map_err(|_| ProxmoxImportError::InvalidFieldValue {
            source_name: source_name.to_owned(),
            field: "memory",
            value: value.to_owned(),
        })?;

    if memory < 0 {
        return Err(ProxmoxImportError::InvalidFieldValue {
            source_name: source_name.to_owned(),
            field: "memory",
            value: value.to_owned(),
        });
    }

    Ok(memory)
}

/// Returns true when a key matches a numeric Proxmox slot pattern.
fn is_numeric_slot_key(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
    })
}

impl Importer for ProxmoxImporter {
    /// Imports a Proxmox VM configuration into the canonical runtime model.
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError> {
        let (storage_path, source_path) = match args {
            ImportOptions::Proxmox { storage, vm } => (storage, vm),
            _ => return Err(ImportError::InvalidFormat),
        };

        let _storage_path = storage_path;
        let source_text = std::fs::read_to_string(&source_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", source_path, e)))?;

        parse_source(&source_text, Path::new(&source_path))
            .map_err(|e| ImportError::ImportFailed(e.to_string()))
    }
}
