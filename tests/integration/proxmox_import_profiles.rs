use super::*;
use ezkvm::import::proxmox::{ImportOutputMode, ImportRunOptions, run_import_from_files};
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

fn imported_profiles(
    conf_path: &str,
    storage_path: Option<&str>,
    runtime_target: ezkvm::import::proxmox::RuntimeTarget,
    output_mode: ImportOutputMode,
) -> Vec<String> {
    let result = with_repo_profiles(|| {
        run_import_from_files(
            conf_path,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage_path.map(str::to_string),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode,
                runtime_target,
            },
        )
        .expect("import should succeed")
    });

    let yaml = if output_mode == ImportOutputMode::DebugCanonical {
        strip_yaml_comments(&result.yaml)
    } else {
        result.yaml
    };

    let root: Value = serde_yaml::from_str(&yaml).expect("yaml should parse");
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

fn strip_yaml_comments(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn wakiza_import_emits_expected_profile_stack() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "input/felucia/108.conf",
        Some("input/felucia/storage.cfg"),
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );

    assert_eq!(
        profiles,
        vec![
            "proxmox-base",
            "proxmox-q35-uefi",
            "proxmox-windows",
            "windows-common",
            "windows-11",
            "looking-glass",
            "gpu-passthrough",
        ]
    );
    assert!(!profiles.contains(&"proxmox-parity-runtime".to_string()));
}

#[test]
fn linux_desktop_import_emits_expected_profile_stack() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "input/zbp-server-mh2/301.conf",
        Some("input/zbp-server-mh2/storage.cfg"),
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );

    assert!(profiles.contains(&"proxmox-q35-uefi".to_string()));
    assert!(profiles.contains(&"linux-l26-common".to_string()));
    assert!(profiles.contains(&"gpu-passthrough".to_string()));
    assert!(profiles.contains(&"hugepages".to_string()));
}

#[test]
fn macos_import_emits_expected_profile_stack() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "input/coruscant/401.conf",
        Some("input/coruscant/storage.cfg"),
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );

    assert!(profiles.contains(&"proxmox-q35-uefi".to_string()));
    assert!(profiles.contains(&"macos-kvm".to_string()));
    assert!(profiles.contains(&"gpu-passthrough".to_string()));
}

#[test]
fn nested_vm_import_emits_viommu_and_hidden_hypervisor_profiles() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "input/coruscant/194.conf",
        Some("input/coruscant/storage.cfg"),
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );

    assert!(profiles.contains(&"proxmox-q35-uefi".to_string()));
    assert!(profiles.contains(&"linux-l26-common".to_string()));
    assert!(profiles.contains(&"viommu".to_string()));
    assert!(profiles.contains(&"hidden-hypervisor".to_string()));
}

#[test]
fn headless_vnc_fixture_emits_headless_vnc_profile() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "tests/fixtures/proxmox_import/10-headless-vnc.conf",
        None,
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );

    assert!(profiles.contains(&"headless-vnc".to_string()));
    assert!(!profiles.contains(&"headless-serial".to_string()));
}

#[test]
fn parity_target_includes_proxmox_parity_runtime_profile() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "input/felucia/108.conf",
        Some("input/felucia/storage.cfg"),
        ezkvm::import::proxmox::RuntimeTarget::ProxmoxParity,
        ImportOutputMode::Compact,
    );

    assert!(profiles.contains(&"proxmox-parity-runtime".to_string()));
}

#[test]
fn mixed_storage_fixture_keeps_scsi_profile_inference() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let profiles = imported_profiles(
        "tests/fixtures/proxmox_import/12-mixed-storage-buses.conf",
        None,
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );

    assert!(
        profiles.contains(&"storage-virtio-scsi-single".to_string()),
        "expected virtio-scsi-single profile for mixed-bus fixture: {profiles:?}"
    );
}

#[test]
fn profile_inference_is_consistent_across_output_modes() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let canonical = imported_profiles(
        "tests/fixtures/proxmox_import/11-nested-viommu-hidden.conf",
        None,
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Canonical,
    );
    let compact = imported_profiles(
        "tests/fixtures/proxmox_import/11-nested-viommu-hidden.conf",
        None,
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::Compact,
    );
    let debug = imported_profiles(
        "tests/fixtures/proxmox_import/11-nested-viommu-hidden.conf",
        None,
        ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        ImportOutputMode::DebugCanonical,
    );

    assert_eq!(canonical, compact);
    assert_eq!(canonical, debug);
    assert!(canonical.contains(&"linux-l26-common".to_string()));
    assert!(canonical.contains(&"viommu".to_string()));
    assert!(canonical.contains(&"hidden-hypervisor".to_string()));
}
