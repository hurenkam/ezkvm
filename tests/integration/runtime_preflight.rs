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

    let base_path = std::env::var("PATH").unwrap_or_default();
    let path_env = format!("{}:{}", temp_dir.display(), base_path);
    let output = Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .arg("start")
        .arg(&vm_path)
        .arg("--dry-run")
        .arg("--swtpm-binary")
        .arg("/definitely/missing/swtpm")
        .env("PATH", path_env)
        .env("EZKVM_CONFIG", "/tmp/ezkvm-config-does-not-exist.yaml")
        .output()
        .expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(stdout.contains("Runtime preflight checks passed"));
    assert!(stdout.contains("Dry run mode - would execute:"));
    assert!(stdout.contains("Capability resolution diagnostics:"));
    assert!(stdout.contains("runtime_root: source="));

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

    let base_path = std::env::var("PATH").unwrap_or_default();
    let path_env = format!("{}:{}", temp_dir.display(), base_path);
    let output = Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .arg("start")
        .arg(&vm_path)
        .arg("--dry-run")
        .arg("--swtpm-binary")
        .arg("/definitely/missing/swtpm")
        .env("PATH", path_env)
        .env("EZKVM_CONFIG", "/tmp/ezkvm-config-does-not-exist.yaml")
        .output()
        .expect("command should run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains(
        "preflight failed: required swtpm binary '/definitely/missing/swtpm' is not available"
    ));

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

#[test]
fn dry_run_network_bridge_falls_back_to_user_when_helper_missing() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp_dir = unique_temp_dir("network-fallback");
    std::fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    write_fake_qemu(&temp_dir);

    let vm_path = temp_dir.join("vm.yaml");
    write_file(
        &vm_path,
        "name: preflight-network-fallback\nbackend: qemu\nsystem:\n  architecture: x86_64\n  machine: q35\n  memory:\n    size: 1024\n  cpu:\n    model: host\n    vcpus: 2\ndevices:\n  networks:\n    - id: net0\n      model: virtio-net-pci\n      backend:\n        type: bridge\n        bridge: vmbr0\n        helper: /definitely/missing/qemu-bridge-helper\n",
    );

    let output = run_start_dry_run(&vm_path, &temp_dir);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        stdout,
        stderr
    );

    let helper_available = std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| dir.join("qemu-bridge-helper").is_file())
    }) || [
        "/usr/lib/qemu/qemu-bridge-helper",
        "/usr/libexec/qemu-bridge-helper",
        "/usr/lib64/qemu-bridge-helper",
    ]
    .iter()
    .any(|candidate| Path::new(candidate).is_file());

    if helper_available {
        assert!(!stdout.contains("network 'net0' downgraded to user-mode"));
        assert!(stdout.contains("type=bridge,id=net0,br=vmbr0"));
    } else {
        assert!(stdout.contains("network 'net0' downgraded to user-mode"));
        assert!(stdout.contains("type=user,id=net0,hostname=preflight-network-fallback"));
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn dry_run_proxmox_parity_skips_portable_capability_gates() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp_dir = unique_temp_dir("parity-bypass");
    std::fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    write_fake_qemu(&temp_dir);

    let vm_path = temp_dir.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: preflight-parity-bypass
backend: qemu
profiles:
    - proxmox-parity-runtime
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(stdout.contains("Capability resolution diagnostics:"));
    assert!(stdout.contains("runtime capabilities: source=parity-bypass"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
