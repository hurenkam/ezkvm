use super::*;
use ezkvm::import::qemu_cmd::{ImportOutputMode, ImportRunOptions, run_import_from_files};
use serde_yaml::Value;
use std::path::Path;

fn profiles_from_yaml(yaml: &str) -> Vec<String> {
    let root: Value = serde_yaml::from_str(yaml).expect("yaml should parse");
    root.as_mapping()
        .and_then(|map| map.get(Value::String("profiles".to_string())))
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[test]
fn qemu_cmd_profile_inference_covers_windows_linux_and_macos_workloads() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let windows_input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qemu_cmd_import/01-wakiza.qemu.cmd");
    let linux_input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("input")
        .join("zbp-server-mh2/301.qemu.cmd");
    let macos_input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("input")
        .join("coruscant/401.qemu.cmd");

    let windows = run_import_from_files(
        &windows_input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
        },
    )
    .expect("windows fixture import should succeed");
    let linux = run_import_from_files(
        &linux_input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
        },
    )
    .expect("linux fixture import should succeed");
    let macos = run_import_from_files(
        &macos_input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
        },
    )
    .expect("macos fixture import should succeed");

    let windows_profiles = profiles_from_yaml(&windows.yaml);
    let linux_profiles = profiles_from_yaml(&linux.yaml);
    let macos_profiles = profiles_from_yaml(&macos.yaml);

    assert!(windows_profiles.contains(&"windows-common".to_string()));
    assert!(windows_profiles.contains(&"windows-11".to_string()));

    assert!(linux_profiles.contains(&"linux-l26-common".to_string()));
    assert!(!linux_profiles.contains(&"windows-common".to_string()));

    assert!(macos_profiles.contains(&"macos-kvm".to_string()));
    assert!(!macos_profiles.contains(&"windows-common".to_string()));
}
