use super::*;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ezkvm-runtime-preflight-{}-{}", label, nanos))
}

fn write_file(path: &Path, content: &str) {
    let mut file = std::fs::File::create(path).expect("file should be creatable");
    file.write_all(content.as_bytes())
        .expect("file should be writable");
}

fn write_fake_qemu(bin_dir: &Path) {
    let qemu_path = bin_dir.join("qemu-system-x86_64");
    write_file(&qemu_path, "#!/bin/sh\nexit 0\n");

    let mut perms = std::fs::metadata(&qemu_path)
        .expect("qemu stub should exist")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&qemu_path, perms).expect("qemu stub should be executable");
}

fn run_start_dry_run(config_path: &Path, bin_dir: &Path) -> std::process::Output {
    let base_path = std::env::var("PATH").unwrap_or_default();
    let path_env = format!("{}:{}", bin_dir.display(), base_path);

    Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .arg("start")
        .arg(config_path)
        .arg("--dry-run")
        .env("PATH", path_env)
        .env("EZKVM_CONFIG", "/tmp/ezkvm-config-does-not-exist.yaml")
        .output()
        .expect("command should run")
}

#[test]
fn dry_run_preflight_success_path() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp_dir = unique_temp_dir("success");
    std::fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    write_fake_qemu(&temp_dir);

    let vm_path = temp_dir.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: preflight-success
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
    );

    let output = run_start_dry_run(&vm_path, &temp_dir);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Runtime preflight checks passed"));
    assert!(stdout.contains("Dry run mode - would execute:"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn dry_run_preflight_required_tpm_capability_failure() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp_dir = unique_temp_dir("required-failure");
    std::fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    write_fake_qemu(&temp_dir);

    let vm_path = temp_dir.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: preflight-required-failure
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
  tpm:
    version: "2.0"
    backend: emulator
    model: tpm-tis
devices: {}
"#,
    );

    let output = run_start_dry_run(&vm_path, &temp_dir);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("preflight failed: TPM emulator backend requires --swtpm-binary"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn dry_run_preflight_optional_remote_viewer_downgrade() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp_dir = unique_temp_dir("optional-warning");
    std::fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    write_fake_qemu(&temp_dir);

    let vm_path = temp_dir.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: preflight-optional-warning
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
spice:
  enabled: true
  port: 5903
  addr: 0.0.0.0
"#,
    );

    let output = run_start_dry_run(&vm_path, &temp_dir);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Preflight warning: remote-viewer integration disabled"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
