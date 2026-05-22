use super::*;
use ezkvm::import::qemu_cmd::{
    ImportOutputMode, ImportRunOptions, RuntimeTarget, run_import_from_files,
};
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

fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
    let old = std::env::var_os("EZKVM_CONFIG");
    let central_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_path);
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));

    unsafe {
        match old {
            Some(value) => std::env::set_var("EZKVM_CONFIG", value),
            None => std::env::remove_var("EZKVM_CONFIG"),
        }
    }

    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
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
                runtime_target: RuntimeTarget::PortableLinux,
            },
        )
        .unwrap_or_else(|err| panic!("fixture '{}' import failed: {err}", case.name));

        let config = VmConfig::from_str(&result.yaml)
            .unwrap_or_else(|err| panic!("fixture '{}' yaml invalid: {err}", case.name));

        if case.name == "wakiza" {
            assert!(config.host.pci.iter().any(|device| {
                device.device == "0000:03:00.0"
                    && device.bus.as_deref() == Some("ich9-pcie-port-1")
                    && device.addr.as_deref() == Some("0x0.0")
            }));
            assert!(
                config.controllers.scsi.iter().any(|controller| {
                    controller.id == "scsihw0" && controller.r#type == "pvscsi"
                })
            );
            assert!(config.devices.drives.iter().any(|drive| {
                drive.id == "scsi0" && drive.interface == "scsi" && drive.boot_index == Some(100)
            }));
        }

        if case.name == "felucia-505" {
            assert!(
                config
                    .host
                    .pci
                    .iter()
                    .any(|device| device.device == "0000:43:00.0")
            );
            assert!(config.controllers.scsi.iter().any(|controller| {
                controller.id == "virtioscsi0" && controller.r#type == "virtio-scsi-pci"
            }));
            assert!(
                config
                    .devices
                    .drives
                    .iter()
                    .any(|drive| drive.id == "scsi0" && drive.interface == "scsi")
            );
        }

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
            runtime_target: RuntimeTarget::PortableLinux,
        },
    )
    .expect_err("strict mode should fail when fixture emits mapping warnings");

    assert!(err.to_string().contains("strict import failed"));
}

#[test]
fn qemu_cmd_import_runtime_target_branches_netdev_helper_mapping() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let input_path = fixture_path("qemu_cmd_import/01-wakiza.qemu.cmd");

    let portable = run_import_from_files(
        &input_path.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
            runtime_target: RuntimeTarget::PortableLinux,
        },
    )
    .expect("portable target import should succeed");

    let parity = run_import_from_files(
        &input_path.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
            runtime_target: RuntimeTarget::ProxmoxParity,
        },
    )
    .expect("parity target import should succeed");

    let portable_cfg =
        VmConfig::from_str(&portable.yaml).expect("portable yaml should deserialize");
    let parity_cfg = VmConfig::from_str(&parity.yaml).expect("parity yaml should deserialize");

    let portable_backend = portable_cfg
        .devices
        .networks
        .first()
        .and_then(|network| network.backend.as_ref())
        .expect("portable backend must exist");
    let parity_backend = parity_cfg
        .devices
        .networks
        .first()
        .and_then(|network| network.backend.as_ref())
        .expect("parity backend must exist");

    assert_eq!(portable_backend.script, None);
    assert_eq!(portable_backend.downscript, None);
    assert_eq!(
        parity_backend.script.as_deref(),
        Some("/usr/libexec/qemu-server/pve-bridge")
    );
    assert_eq!(
        parity_backend.downscript.as_deref(),
        Some("/usr/libexec/qemu-server/pve-bridgedown")
    );
}

#[test]
fn q35_hostpci_placement_parity_matches_between_proxmox_and_qemu_cmd_imports() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let storage_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("input")
        .join("felucia")
        .join("storage.cfg");
    let proxmox_input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("input")
        .join("felucia")
        .join("108.conf");
    let qemu_cmd_input = fixture_path("qemu_cmd_import/01-wakiza.qemu.cmd");

    let proxmox_result = ezkvm::import::proxmox::run_import_from_files(
        &proxmox_input.to_string_lossy(),
        &ezkvm::import::proxmox::ImportRunOptions {
            output_path: None,
            storage_path: Some(storage_path.to_string_lossy().to_string()),
            strict: false,
            dry_run: true,
            compact_lists: false,
            output_mode: ezkvm::import::proxmox::ImportOutputMode::Canonical,
            runtime_target: ezkvm::import::proxmox::RuntimeTarget::PortableLinux,
        },
    )
    .expect("proxmox import should succeed");

    let qemu_cmd_result = run_import_from_files(
        &qemu_cmd_input.to_string_lossy(),
        &ImportRunOptions {
            output_path: None,
            strict: false,
            dry_run: true,
            output_mode: ImportOutputMode::Canonical,
            runtime_target: RuntimeTarget::PortableLinux,
        },
    )
    .expect("qemu-cmd import should succeed");

    let proxmox_cfg = with_repo_profiles(|| {
        VmConfig::from_str(&proxmox_result.yaml).expect("proxmox yaml should parse")
    });
    let qemu_cmd_cfg = with_repo_profiles(|| {
        VmConfig::from_str(&qemu_cmd_result.yaml).expect("qemu-cmd yaml should parse")
    });

    let proxmox_gpu = proxmox_cfg
        .host
        .pci
        .iter()
        .find(|entry| entry.device == "0000:03:00.0")
        .expect("proxmox import should include gpu function 0");
    let qemu_cmd_gpu = qemu_cmd_cfg
        .host
        .pci
        .iter()
        .find(|entry| entry.device == "0000:03:00.0")
        .expect("qemu-cmd import should include gpu function 0");

    assert_eq!(proxmox_gpu.bus.as_deref(), Some("ich9-pcie-port-1"));
    assert_eq!(qemu_cmd_gpu.bus.as_deref(), Some("ich9-pcie-port-1"));
}
