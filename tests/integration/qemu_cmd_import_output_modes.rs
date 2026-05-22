use super::*;
use ezkvm::import::qemu_cmd::{
    ImportOutputMode, ImportRunOptions, RuntimeTarget, run_import_from_files,
};
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
            runtime_target: RuntimeTarget::PortableLinux,
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
            runtime_target: RuntimeTarget::PortableLinux,
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
            runtime_target: RuntimeTarget::PortableLinux,
        },
    )
    .expect("debug import should succeed");

    assert_eq!(canonical.yaml, compact.yaml);
    assert!(debug.yaml.contains("# from qemu executable:"));

    let canonical_warning_fields = canonical
        .warnings
        .iter()
        .map(|warning| warning.source_field.clone())
        .collect::<Vec<_>>();
    let compact_warning_fields = compact
        .warnings
        .iter()
        .map(|warning| warning.source_field.clone())
        .collect::<Vec<_>>();
    let debug_warning_fields = debug
        .warnings
        .iter()
        .map(|warning| warning.source_field.clone())
        .collect::<Vec<_>>();

    assert_eq!(canonical_warning_fields, compact_warning_fields);
    assert_eq!(canonical_warning_fields, debug_warning_fields);

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

#[test]
fn qemu_cmd_compact_output_is_replay_safe_and_host_independent_for_portable_target() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qemu_cmd_import/01-wakiza.qemu.cmd");

    let first = run_import_from_files(
        &input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Compact,
            runtime_target: RuntimeTarget::PortableLinux,
        },
    )
    .expect("first compact import should succeed");

    let second = run_import_from_files(
        &input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Compact,
            runtime_target: RuntimeTarget::PortableLinux,
        },
    )
    .expect("second compact import should succeed");

    assert_eq!(
        first.yaml, second.yaml,
        "compact output must be deterministic"
    );
    assert!(
        !first.yaml.contains("/var/run/qemu-server"),
        "portable compact output must not persist proxmox runtime socket/pid paths"
    );
    assert!(
        !first.yaml.contains("/usr/libexec/qemu-server/"),
        "portable compact output must not persist proxmox network helper paths"
    );

    let first_cfg = VmConfig::from_str(&first.yaml).expect("first compact yaml should deserialize");
    let second_cfg =
        VmConfig::from_str(&second.yaml).expect("second compact yaml should deserialize");

    let first_args = QemuManager::new(first_cfg, CentralConfig::default())
        .build_command()
        .expect("first compact command build")
        .into_inner();
    let second_args = QemuManager::new(second_cfg, CentralConfig::default())
        .build_command()
        .expect("second compact command build")
        .into_inner();

    assert_eq!(
        first_args, second_args,
        "compact round-trip command args must stay deterministic"
    );
}

#[test]
fn qemu_cmd_import_selected_fixtures_have_stable_dry_run_args() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let fixtures = [
        "qemu_cmd_import/01-wakiza.qemu.cmd",
        "qemu_cmd_import/03-felucia-505.qemu.cmd",
    ];

    for fixture in fixtures {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(fixture);

        let first = run_import_from_files(
            &input.to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                strict: false,
                dry_run: true,
                output_mode: ImportOutputMode::Canonical,
                runtime_target: RuntimeTarget::PortableLinux,
            },
        )
        .expect("first import should succeed");

        let second = run_import_from_files(
            &input.to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                strict: false,
                dry_run: true,
                output_mode: ImportOutputMode::Canonical,
                runtime_target: RuntimeTarget::PortableLinux,
            },
        )
        .expect("second import should succeed");

        let first_cfg = VmConfig::from_str(&first.yaml).expect("first yaml should deserialize");
        let second_cfg = VmConfig::from_str(&second.yaml).expect("second yaml should deserialize");

        let first_args = QemuManager::new(first_cfg, CentralConfig::default())
            .build_command()
            .expect("first command build")
            .into_inner();
        let second_args = QemuManager::new(second_cfg, CentralConfig::default())
            .build_command()
            .expect("second command build")
            .into_inner();

        assert_eq!(
            first_args, second_args,
            "fixture '{}' must be stable",
            fixture
        );
    }
}
