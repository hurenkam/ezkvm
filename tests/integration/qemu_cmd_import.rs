use super::*;
use ezkvm::import::qemu_cmd::{ImportOutputMode, ImportRunOptions, run_import_from_files};
use std::path::{Path, PathBuf};

struct QemuCmdFixtureCase {
    name: &'static str,
    fixture: &'static str,
}

fn fixture_cases() -> &'static [QemuCmdFixtureCase] {
    &[
        QemuCmdFixtureCase {
            name: "wakiza",
            fixture: "qemu_cmd_import/01-wakiza.qemu.cmd",
        },
        QemuCmdFixtureCase {
            name: "zbp-201",
            fixture: "qemu_cmd_import/02-zbp-201.qemu.cmd",
        },
        QemuCmdFixtureCase {
            name: "felucia-505",
            fixture: "qemu_cmd_import/03-felucia-505.qemu.cmd",
        },
    ]
}

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

#[test]
fn qemu_cmd_import_fixtures_support_validate_and_dry_run_command_build() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut failures = Vec::new();

    for case in fixture_cases() {
        let input_path = fixture_path(case.fixture);
        let result = run_import_from_files(
            &input_path.to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                strict: false,
                dry_run: true,
                output_mode: ImportOutputMode::Compact,
            },
        )
        .unwrap_or_else(|err| panic!("fixture '{}' import failed: {err}", case.name));

        let config = VmConfig::from_str(&result.yaml)
            .unwrap_or_else(|err| panic!("fixture '{}' yaml invalid: {err}", case.name));

        let command_result = QemuManager::new(config, CentralConfig::default()).build_command();
        if let Err(error) = command_result {
            failures.push(format!(
                "fixture '{}' failed command generation: {}",
                case.name, error
            ));
        }

        if result.output_path.is_empty() {
            failures.push(format!(
                "fixture '{}' produced empty output path",
                case.name
            ));
        }
    }

    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}

#[test]
fn qemu_cmd_import_strict_mode_fails_on_warning_rich_fixture() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let input_path = fixture_path("qemu_cmd_import/01-wakiza.qemu.cmd");

    let err = run_import_from_files(
        &input_path.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: true,
            dry_run: true,
            output_mode: ImportOutputMode::Compact,
        },
    )
    .expect_err("strict mode should fail when fixture emits mapping warnings");

    assert!(err.to_string().contains("strict import failed"));
}
