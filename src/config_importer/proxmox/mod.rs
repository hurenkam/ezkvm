//! Proxmox .conf file import adapter.
//!
//! Owns Proxmox-specific parsing and adaptation to runtime config specification.

use std::ffi::OsStr;
use std::path::Path;

use crate::vm_spec::model::{
    Cpu, Machine, Memory, Metadata, NetworkEntry, ResourceRef, StorageEntry, System,
};
use crate::vm_spec::model::{EZKVM_CONFIG_SCHEMA_VERSION, VirtualMachine};

use super::{ConfigArgs, ConfigImportError, ConfigImporter, RuntimeConfig, read_config_text};

#[derive(Debug, thiserror::Error)]
enum ProxmoxImportError {
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
pub struct ProxmoxConfigImporter;

impl ProxmoxConfigImporter {
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
        let mut storage: Vec<StorageEntry> = Vec::new();
        let mut network: Vec<NetworkEntry> = Vec::new();
        let mut resources: Vec<ResourceRef> = Vec::new();

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

        Ok(RuntimeConfig {
            metadata: Metadata {
                schema_version: EZKVM_CONFIG_SCHEMA_VERSION.to_owned(),
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

impl ConfigImporter for ProxmoxConfigImporter {
    type ConfigError = ConfigImportError;

    fn import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, Self::ConfigError> {
        let config_path_arg =
            config_args
                .args
                .first()
                .ok_or(ConfigImportError::MissingConfigPath {
                    importer: "proxmox",
                })?;
        let config_path = Path::new(config_path_arg);
        let source_text = read_config_text(config_path, "proxmox")?;
        let _extra_options = &config_args.args[1..];

        Self::parse_source(&source_text, config_path).map_err(|error| ConfigImportError::Importer {
            importer: "proxmox",
            reason: error.to_string(),
        })
    }
}

fn is_proxmox_conf_source(source_name: &Path) -> bool {
    matches!(
        source_name.extension().and_then(OsStr::to_str),
        Some("conf")
    )
}

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

fn required_machine(
    value: Option<Machine>,
    source_name: &str,
) -> Result<Machine, ProxmoxImportError> {
    value.ok_or(ProxmoxImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field: "machine",
    })
}

fn required_memory(value: Option<i64>, source_name: &str) -> Result<i64, ProxmoxImportError> {
    value.ok_or(ProxmoxImportError::MissingRequiredField {
        source_name: source_name.to_owned(),
        field: "memory",
    })
}

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
    })
}

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

fn is_numeric_slot_key(key: &str, prefix: &str) -> bool {
    key.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::ProxmoxConfigImporter;
    use crate::config_importer::{ConfigArgs, ConfigImportError, ConfigImporter};
    use crate::vm_spec::{ConformanceError, validate_runtime_config};

    struct CorpusExpectation {
        fixture_label: &'static str,
        source_text: &'static str,
        source_name: &'static str,
        expected_vm_name: &'static str,
        expected_chipset: &'static str,
        expected_cpu_model: &'static str,
        expected_memory_min: i64,
        expected_storage_ids: &'static [&'static str],
        expected_network_ids: &'static [&'static str],
        expected_resource_ids: &'static [&'static str],
    }

    const FELUCIA_108_CONF: &str = r#"
name: wakiza
machine: q35
cpu: host
memory: 16384
scsi0: local-lvm:vm-108-disk-0
scsi1: local-lvm:vm-108-disk-1
net0: virtio=AA:BB:CC:DD:EE:FF,bridge=vmbr0
hostpci0: 0000:65:00
usb0: host=046d:c52b
"#;

    const CORUSCANT_3101_CONF: &str = r#"
name: gyndine
machine: q35
cpu: host
memory: 65536
scsi0: local-zfs:vm-3101-disk-0
hostpci0: 0000:01:00
hostpci1: 0000:02:00
hostpci2: 0000:03:00
hostpci3: 0000:04:00
hostpci4: 0000:05:00
hostpci5: 0000:06:00
hostpci6: 0000:07:00
hostpci7: 0000:08:00
hostpci8: 0000:09:00
hostpci9: 0000:0a:00
hostpci10: 0000:0b:00
hostpci11: 0000:0c:00
"#;

    const ZBP_SERVER_MH2_103_CONF: &str = r#"
name: desktop-markh-3
machine: q35
cpu: x86-64-v2-AES
memory: 24576
scsi0: local-zfs:vm-103-disk-0
scsi1: local-zfs:vm-103-disk-1
net0: virtio=11:22:33:44:55:66,bridge=vmbr0
"#;

    static NEXT_CASE_ID: AtomicUsize = AtomicUsize::new(0);

    fn corpus_expectations() -> &'static [CorpusExpectation] {
        &[
            CorpusExpectation {
                fixture_label: "felucia/108.conf",
                source_text: FELUCIA_108_CONF,
                source_name: "/tmp/108.conf",
                expected_vm_name: "wakiza",
                expected_chipset: "q35",
                expected_cpu_model: "host",
                expected_memory_min: 16384,
                expected_storage_ids: &["scsi0", "scsi1"],
                expected_network_ids: &["net0"],
                expected_resource_ids: &["hostpci0", "usb0"],
            },
            CorpusExpectation {
                fixture_label: "coruscant/3101.conf",
                source_text: CORUSCANT_3101_CONF,
                source_name: "/tmp/3101.conf",
                expected_vm_name: "gyndine",
                expected_chipset: "q35",
                expected_cpu_model: "host",
                expected_memory_min: 65536,
                expected_storage_ids: &["scsi0"],
                expected_network_ids: &[],
                expected_resource_ids: &[
                    "hostpci0",
                    "hostpci1",
                    "hostpci2",
                    "hostpci3",
                    "hostpci4",
                    "hostpci5",
                    "hostpci6",
                    "hostpci7",
                    "hostpci8",
                    "hostpci9",
                    "hostpci10",
                    "hostpci11",
                ],
            },
            CorpusExpectation {
                fixture_label: "zbp-server-mh2/103.conf",
                source_text: ZBP_SERVER_MH2_103_CONF,
                source_name: "/tmp/103.conf",
                expected_vm_name: "desktop-markh-3",
                expected_chipset: "q35",
                expected_cpu_model: "x86-64-v2-AES",
                expected_memory_min: 24576,
                expected_storage_ids: &["scsi0", "scsi1"],
                expected_network_ids: &["net0"],
                expected_resource_ids: &[],
            },
        ]
    }

    fn import_corpus_case(case: &CorpusExpectation) -> crate::vm_spec::RuntimeConfig {
        let case_id = NEXT_CASE_ID.fetch_add(1, Ordering::Relaxed);
        let source_path = PathBuf::from(format!(
            "/tmp/ezkvm-proxmox-{case_id}-{}.conf",
            case.expected_vm_name
        ));
        fs::write(&source_path, case.source_text).expect("fixture write should succeed");
        let config_args = ConfigArgs::new(vec![source_path.to_string_lossy().into_owned()]);
        ProxmoxConfigImporter
            .import_config(config_args)
            .unwrap_or_else(|err| panic!("{} should parse: {err}", case.fixture_label))
    }

    fn runtime_output_path(vm_name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/{vm_name}.yaml"))
    }

    fn assert_storage_ids(
        entries: &[crate::vm_spec::model::StorageEntry],
        expected_ids: &[&str],
        fixture_label: &str,
    ) {
        let actual_ids: Vec<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(
            actual_ids, expected_ids,
            "unexpected storage IDs for {fixture_label}"
        );
    }

    fn assert_network_ids(
        entries: &[crate::vm_spec::model::NetworkEntry],
        expected_ids: &[&str],
        fixture_label: &str,
    ) {
        let actual_ids: Vec<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(
            actual_ids, expected_ids,
            "unexpected network IDs for {fixture_label}"
        );
    }

    fn assert_resource_ids(
        entries: &[crate::vm_spec::model::ResourceRef],
        expected_ids: &[&str],
        fixture_label: &str,
    ) {
        let actual_ids: Vec<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(
            actual_ids, expected_ids,
            "unexpected resource IDs for {fixture_label}"
        );
    }

    #[test]
    fn proxmox_importer_preserves_corpus_expectations() {
        for case in corpus_expectations() {
            let document = import_corpus_case(case);

            assert_eq!(
                document.metadata.schema_version, "1.0.0",
                "{}",
                case.fixture_label
            );
            assert_eq!(
                document.metadata.vm_name, case.expected_vm_name,
                "{}",
                case.fixture_label
            );
            assert_eq!(
                document.virtual_machine.system.machine.family, "pc",
                "{}",
                case.fixture_label
            );
            assert_eq!(
                document.virtual_machine.system.machine.chipset, case.expected_chipset,
                "{}",
                case.fixture_label
            );
            assert_eq!(
                document.virtual_machine.system.cpu.model, case.expected_cpu_model,
                "{}",
                case.fixture_label
            );
            assert_eq!(
                document.virtual_machine.system.memory.min, case.expected_memory_min,
                "{}",
                case.fixture_label
            );

            assert_storage_ids(
                &document.virtual_machine.storage,
                case.expected_storage_ids,
                case.fixture_label,
            );
            assert_network_ids(
                &document.virtual_machine.network,
                case.expected_network_ids,
                case.fixture_label,
            );
            assert_resource_ids(
                &document.virtual_machine.resources,
                case.expected_resource_ids,
                case.fixture_label,
            );

            let runtime_path = runtime_output_path(case.expected_vm_name);
            validate_runtime_config(&document, &runtime_path).unwrap_or_else(|err| {
                panic!(
                    "{} should conform when validated as {}: {err}",
                    case.fixture_label,
                    runtime_path.display()
                )
            });
        }
    }

    #[test]
    fn proxmox_imported_corpus_documents_report_source_filename_mismatch() {
        for case in corpus_expectations() {
            let document = import_corpus_case(case);
            let source_name = Path::new(case.source_name);
            let expected_stem = source_name
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("fixture source name should have a stem");

            let err = validate_runtime_config(&document, source_name).unwrap_err();

            match err {
                ConformanceError::Validation(_, issues) => {
                    assert!(issues.iter().any(|issue| {
                        issue.path == "metadata.vm_name"
                            && issue
                                .reason
                                .contains(&format!("filename stem '{expected_stem}'"))
                    }));
                }
                other => panic!(
                    "{} should fail with a filename-stem validation issue, got {other:?}",
                    case.fixture_label
                ),
            }
        }
    }

    #[test]
    fn proxmox_machine_parser_handles_pc_q35_shape() {
        let machine = super::parse_machine_value("pc-q35-8.1", "/tmp/108.conf")
            .expect("machine parser should accept pc-q35 shapes");

        assert_eq!(machine.family, "pc");
        assert_eq!(machine.chipset, "q35");
    }

    #[test]
    fn proxmox_importer_rejects_malformed_source_name() {
        let source_path = PathBuf::from("/tmp/wakiza.yaml");
        fs::write(
            &source_path,
            "name: wakiza\nmachine: pc-q35-8.1\ncpu: host\nmemory: 8192\n",
        )
        .expect("fixture write should succeed");
        let config_args = ConfigArgs::new(vec![source_path.to_string_lossy().into_owned()]);

        let err = ProxmoxConfigImporter
            .import_config(config_args)
            .expect_err("non-.conf source names should be rejected");

        assert!(matches!(
            err,
            ConfigImportError::Importer {
                importer: "proxmox",
                ..
            }
        ));
        assert!(
            err.to_string()
                .contains("expected a Proxmox .conf source name")
        );
    }
}
