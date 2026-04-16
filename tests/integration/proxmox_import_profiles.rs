use super::*;
use ezkvm::import::proxmox::{ImportRunOptions, run_import_from_files};
use serde_yaml::Value;
use std::path::Path;

fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
    let old = std::env::var_os("EZKVM_CONFIG");
    let central_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_path);
    }

    let result = run();

    unsafe {
        match old {
            Some(value) => std::env::set_var("EZKVM_CONFIG", value),
            None => std::env::remove_var("EZKVM_CONFIG"),
        }
    }

    result
}

fn imported_profiles(conf_path: &str, storage_path: Option<&str>) -> Vec<String> {
    let result = with_repo_profiles(|| {
        run_import_from_files(
            conf_path,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage_path.map(str::to_string),
                strict: false,
                dry_run: true,
                compact_lists: false,
            },
        )
        .expect("import should succeed")
    });

    let root: Value = serde_yaml::from_str(&result.yaml).expect("yaml should parse");
    let profiles = root
        .as_mapping()
        .and_then(|map| map.get(Value::String("profiles".to_string())))
        .and_then(Value::as_sequence)
        .expect("profiles should be present");

    profiles
        .iter()
        .map(|item| item.as_str().expect("profile should be string").to_string())
        .collect()
}

#[test]
fn wakiza_import_emits_expected_profile_stack() {
    let _guard = env_lock().lock().unwrap();

    let profiles = imported_profiles("input/felucia/108.conf", Some("input/felucia/storage.cfg"));

    assert_eq!(
        profiles,
        vec![
            "proxmox-q35-uefi",
            "windows-common",
            "windows-11",
            "looking-glass",
            "gpu-passthrough",
        ]
    );
}

#[test]
fn linux_desktop_import_emits_expected_profile_stack() {
    let _guard = env_lock().lock().unwrap();

    let profiles = imported_profiles(
        "input/zbp-server-mh2/301.conf",
        Some("input/zbp-server-mh2/storage.cfg"),
    );

    assert!(profiles.contains(&"proxmox-q35-uefi".to_string()));
    assert!(profiles.contains(&"linux-l26-common".to_string()));
    assert!(profiles.contains(&"gpu-passthrough".to_string()));
}

#[test]
fn macos_import_emits_expected_profile_stack() {
    let _guard = env_lock().lock().unwrap();

    let profiles = imported_profiles(
        "input/coruscant/401.conf",
        Some("input/coruscant/storage.cfg"),
    );

    assert!(profiles.contains(&"proxmox-q35-uefi".to_string()));
    assert!(profiles.contains(&"macos-kvm".to_string()));
    assert!(profiles.contains(&"gpu-passthrough".to_string()));
}
