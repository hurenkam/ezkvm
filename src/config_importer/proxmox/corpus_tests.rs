use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config_importer::{ConfigArgs, ConfigImporter};
use crate::vm_spec::{ConformanceError, validate_runtime_config};

use super::ProxmoxConfigImporter;

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
