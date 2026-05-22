use super::*;
use ezkvm::import::proxmox::{ImportOutputMode, ImportRunOptions, run_import_from_files};
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

fn strip_yaml_comments(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn proxmox_import_output_modes_preserve_runtime_equivalence() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let input = "input/felucia/108.conf";
    let storage = Some("input/felucia/storage.cfg".to_string());

    let canonical = with_repo_profiles(|| {
        run_import_from_files(
            input,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage.clone(),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ImportOutputMode::Canonical,
                runtime_target: ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
            },
        )
        .expect("canonical import should succeed")
    });

    let compact = with_repo_profiles(|| {
        run_import_from_files(
            input,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage.clone(),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ImportOutputMode::Compact,
                runtime_target: ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
            },
        )
        .expect("compact import should succeed")
    });

    let debug = with_repo_profiles(|| {
        run_import_from_files(
            input,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage,
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ImportOutputMode::DebugCanonical,
                runtime_target: ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
            },
        )
        .expect("debug import should succeed")
    });

    assert_ne!(canonical.yaml, compact.yaml);
    assert!(debug.yaml.contains("# from Proxmox ostype: win11"));
    // USB device is present and references xhci (not synthesized as controller)
    assert!(debug.yaml.contains("bus: xhci.0"));

    let canonical_cfg = with_repo_profiles(|| {
        VmConfig::from_str(&canonical.yaml).expect("canonical yaml should deserialize")
    });
    let compact_cfg = with_repo_profiles(|| {
        VmConfig::from_str(&compact.yaml).expect("compact yaml should deserialize")
    });
    let debug_cfg = with_repo_profiles(|| {
        let stripped = strip_yaml_comments(&debug.yaml);
        VmConfig::from_str(&stripped).expect("debug yaml should deserialize")
    });

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
fn proxmox_compact_output_is_replay_safe_and_host_independent_for_portable_target() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let input = "input/felucia/108.conf";
    let storage = Some("input/felucia/storage.cfg".to_string());

    let first = with_repo_profiles(|| {
        run_import_from_files(
            input,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage.clone(),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ImportOutputMode::Compact,
                runtime_target: ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
            },
        )
        .expect("first compact import should succeed")
    });
    let second = with_repo_profiles(|| {
        run_import_from_files(
            input,
            &ImportRunOptions {
                output_path: None,
                storage_path: storage,
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ImportOutputMode::Compact,
                runtime_target: ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
            },
        )
        .expect("second compact import should succeed")
    });

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

    let first_cfg = with_repo_profiles(|| {
        VmConfig::from_str(&first.yaml).expect("first compact yaml should deserialize")
    });
    let second_cfg = with_repo_profiles(|| {
        VmConfig::from_str(&second.yaml).expect("second compact yaml should deserialize")
    });

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
