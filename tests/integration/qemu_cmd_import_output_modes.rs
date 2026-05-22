use super::*;
use ezkvm::import::qemu_cmd::{ImportOutputMode, ImportRunOptions, run_import_from_files};
use std::path::Path;

fn strip_yaml_comments(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn qemu_cmd_import_output_modes_preserve_runtime_equivalence() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qemu_cmd_import/02-zbp-201.qemu.cmd");

    let canonical = run_import_from_files(
        &input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
        },
    )
    .expect("canonical import should succeed");

    let compact = run_import_from_files(
        &input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Compact,
        },
    )
    .expect("compact import should succeed");

    let debug = run_import_from_files(
        &input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::DebugCanonical,
        },
    )
    .expect("debug import should succeed");

    assert_eq!(canonical.yaml, compact.yaml);
    assert!(debug.yaml.contains("# from qemu executable:"));

    let canonical_cfg =
        VmConfig::from_str(&canonical.yaml).expect("canonical yaml should deserialize");
    let compact_cfg = VmConfig::from_str(&compact.yaml).expect("compact yaml should deserialize");
    let debug_cfg = VmConfig::from_str(&strip_yaml_comments(&debug.yaml))
        .expect("debug yaml should deserialize after comment strip");

    let canonical_args = QemuManager::new(canonical_cfg, CentralConfig::default())
        .build_command()
        .expect("canonical command build")
        .into_inner();
    let compact_args = QemuManager::new(compact_cfg, CentralConfig::default())
        .build_command()
        .expect("compact command build")
        .into_inner();
    let debug_args = QemuManager::new(debug_cfg, CentralConfig::default())
        .build_command()
        .expect("debug command build")
        .into_inner();

    assert_eq!(canonical_args, compact_args);
    assert_eq!(canonical_args, debug_args);
}
