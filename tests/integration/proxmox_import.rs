use super::*;
use ezkvm::import::proxmox::{ImportRunOptions, run_import_from_files};
use std::fs;
use std::path::{Path, PathBuf};

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

struct ImportFixtureCase {
    name: &'static str,
    conf_fixture: &'static str,
    storage_fixture: Option<&'static str>,
    snapshot_fixture: &'static str,
    warning_fields: &'static [&'static str],
}

impl ImportFixtureCase {
    fn conf_path(&self) -> PathBuf {
        fixture_path(self.conf_fixture)
    }

    fn storage_path(&self) -> Option<PathBuf> {
        self.storage_fixture.map(fixture_path)
    }

    fn snapshot_path(&self) -> PathBuf {
        fixture_path(self.snapshot_fixture)
    }
}

#[test]
fn proxmox_import_fixtures_generate_expected_dry_run_snapshots() {
    let _guard = env_lock().lock().unwrap();
    let mut mismatches = Vec::new();

    for case in fixture_cases() {
        if let Some(mismatch) = check_fixture_case(case) {
            mismatches.push(mismatch);
        }
    }

    if !mismatches.is_empty() {
        panic!("{}", mismatches.join("\n\n"));
    }
}

fn fixture_cases() -> &'static [ImportFixtureCase] {
    &[
        ImportFixtureCase {
            name: "scsi bridge tpm",
            conf_fixture: "proxmox_import/01-scsi-bridge-tpm.conf",
            storage_fixture: Some("proxmox_import/storage.cfg"),
            snapshot_fixture: "proxmox_import/01-scsi-bridge-tpm.args",
            warning_fields: &[],
        },
        ImportFixtureCase {
            name: "bios cdrom user",
            conf_fixture: "proxmox_import/02-bios-cdrom-user.conf",
            storage_fixture: None,
            snapshot_fixture: "proxmox_import/02-bios-cdrom-user.args",
            warning_fields: &[],
        },
        ImportFixtureCase {
            name: "hostpci gpu",
            conf_fixture: "proxmox_import/03-hostpci-gpu.conf",
            storage_fixture: None,
            snapshot_fixture: "proxmox_import/03-hostpci-gpu.args",
            warning_fields: &[],
        },
        ImportFixtureCase {
            name: "usb passthrough",
            conf_fixture: "proxmox_import/04-usb-passthrough.conf",
            storage_fixture: None,
            snapshot_fixture: "proxmox_import/04-usb-passthrough.args",
            warning_fields: &[],
        },
        ImportFixtureCase {
            name: "warning rich import",
            conf_fixture: "proxmox_import/05-warning-rich.conf",
            storage_fixture: Some("proxmox_import/storage.cfg"),
            snapshot_fixture: "proxmox_import/05-warning-rich.args",
            warning_fields: &["arch", "scsihw", "net0", "vga"],
        },
    ]
}

fn check_fixture_case(case: &ImportFixtureCase) -> Option<String> {
    let runtime_root = create_temp_test_dir(case.name);
    let runtime_dir = runtime_root.join("runtime");
    let home_dir = runtime_root.join("home");
    fs::create_dir_all(&runtime_dir).unwrap();
    fs::create_dir_all(&home_dir).unwrap();

    let old_home = std::env::var_os("HOME");
    let old_runtime = std::env::var_os("XDG_RUNTIME_DIR");

    unsafe {
        std::env::set_var("HOME", &home_dir);
        std::env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
    }

    let result = with_repo_profiles(|| {
        run_import_from_files(
            &case.conf_path().to_string_lossy(),
            &ImportRunOptions {
                output_path: None,
                storage_path: case
                    .storage_path()
                    .map(|path| path.to_string_lossy().to_string()),
                strict: false,
                dry_run: true,
                compact_lists: false,
            },
        )
        .unwrap_or_else(|err| panic!("fixture '{}' import failed: {err}", case.name))
    });

    let config = with_repo_profiles(|| {
        VmConfig::from_str(&result.yaml)
            .unwrap_or_else(|err| panic!("fixture '{}' yaml invalid: {err}", case.name))
    });
    let args = QemuManager::new(config, CentralConfig::default())
        .build_command()
        .unwrap_or_else(|err| panic!("fixture '{}' command build failed: {err}", case.name))
        .into_inner();

    restore_env(old_home, old_runtime);

    let actual_snapshot = render_snapshot(&args, &runtime_dir);
    let expected_snapshot = fs::read_to_string(case.snapshot_path())
        .unwrap_or_else(|err| panic!("fixture '{}' snapshot missing: {err}", case.name));

    let warning_fields = result
        .warnings
        .iter()
        .map(|warning| warning.source_field.as_str())
        .collect::<Vec<_>>();

    if case.warning_fields != warning_fields {
        return Some(format!(
            "fixture '{}' warning fields changed\nexpected: {:?}\nactual: {:?}",
            case.name, case.warning_fields, warning_fields
        ));
    }

    if expected_snapshot != actual_snapshot {
        return Some(format!(
            "fixture '{}' snapshot mismatch\nexpected:\n{}\nactual:\n{}",
            case.name, expected_snapshot, actual_snapshot
        ));
    }

    None
}

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

fn create_temp_test_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "ezkvm-proxmox-import-{}-{}-{}",
        std::process::id(),
        name.replace(' ', "-"),
        nanos
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn restore_env(old_home: Option<std::ffi::OsString>, old_runtime: Option<std::ffi::OsString>) {
    unsafe {
        match old_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match old_runtime {
            Some(value) => std::env::set_var("XDG_RUNTIME_DIR", value),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }
}

fn render_snapshot(args: &[String], runtime_dir: &Path) -> String {
    let runtime_dir = runtime_dir.to_string_lossy();
    args.iter()
        .map(|arg| arg.replace(runtime_dir.as_ref(), "<RUNTIME_DIR>"))
        .collect::<Vec<_>>()
        .join("\n")
}
