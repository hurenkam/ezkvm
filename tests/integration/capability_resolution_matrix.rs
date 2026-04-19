use super::*;
use ezkvm::import::proxmox::{ImportRunOptions, RuntimeTarget, run_import_from_files};
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
    std::env::temp_dir().join(format!("ezkvm-capability-matrix-{}-{}", label, nanos))
}

fn write_file(path: &Path, content: &str) {
    let mut file = std::fs::File::create(path).expect("file should be creatable");
    file.write_all(content.as_bytes())
        .expect("file should be writable");
}

fn write_executable(path: &Path, script: &str) {
    write_file(path, script);
    let mut perms = std::fs::metadata(path)
        .expect("executable should exist")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("executable should be chmodded");
}

fn run_start_dry_run(
    vm_path: &Path,
    bin_dir: &Path,
    central_config: &Path,
    extra_args: &[&str],
) -> std::process::Output {
    let path_env = format!("{}:/usr/bin:/bin", bin_dir.display());

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ezkvm"));
    cmd.arg("start")
        .arg(vm_path)
        .arg("--dry-run")
        .env("PATH", path_env)
        .env("EZKVM_CONFIG", central_config);

    for arg in extra_args {
        cmd.arg(arg);
    }

    cmd.output().expect("command should run")
}

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

#[test]
fn import_dry_run_reports_capability_precedence_for_portable_and_parity_targets() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let portable = Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .arg("import-proxmox")
        .arg("input/felucia/108.conf")
        .arg("--proxmox-storage")
        .arg("input/felucia/storage.cfg")
        .arg("--runtime-target")
        .arg("portable-linux")
        .arg("--dry-run")
        .env("EZKVM_CONFIG", "etc/ezkvm.yaml")
        .output()
        .expect("portable import command should run");
    let portable_stdout = String::from_utf8_lossy(&portable.stdout);
    assert!(portable.status.success());
    assert!(portable_stdout.contains("# runtime target: PortableLinux"));
    assert!(portable_stdout.contains(
        "# capability precedence: cli > vm-override > profile-default > central-config > platform-default"
    ));

    let parity = Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .arg("import-proxmox")
        .arg("input/felucia/108.conf")
        .arg("--proxmox-storage")
        .arg("input/felucia/storage.cfg")
        .arg("--runtime-target")
        .arg("proxmox-parity")
        .arg("--dry-run")
        .env("EZKVM_CONFIG", "etc/ezkvm.yaml")
        .output()
        .expect("parity import command should run");
    let parity_stdout = String::from_utf8_lossy(&parity.stdout);
    assert!(parity.status.success());
    assert!(parity_stdout.contains("# runtime target: ProxmoxParity"));
    assert!(parity_stdout.contains(
        "# capability precedence: bypassed (proxmox-parity target preserves parity defaults)"
    ));
}

#[test]
fn portable_import_generated_args_omit_proxmox_host_literals() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let result = with_repo_profiles(|| {
        run_import_from_files(
            "input/felucia/108.conf",
            &ImportRunOptions {
                output_path: None,
                storage_path: Some("input/felucia/storage.cfg".to_string()),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: ezkvm::import::proxmox::ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            },
        )
        .expect("portable import should succeed")
    });

    let config = with_repo_profiles(|| {
        VmConfig::from_str(&result.yaml).expect("imported yaml should deserialize")
    });
    let args = QemuManager::new(config, CentralConfig::default())
        .build_command()
        .expect("portable command should build")
        .into_inner()
        .join(" ");

    assert!(!args.contains("/var/run/qemu-server/"));
    assert!(!args.contains("/usr/libexec/qemu-server/pve-bridge"));
    assert!(!args.contains("tap108i0"));
}

#[test]
fn runtime_diagnostics_show_cli_override_precedence_over_central_defaults() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp = unique_temp_dir("runtime-cli-overrides");
    std::fs::create_dir_all(&temp).expect("temp dir should exist");
    let bin_dir = temp.join("bin");
    let firmware_dir = temp.join("ovmf");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should exist");
    std::fs::create_dir_all(&firmware_dir).expect("firmware dir should exist");

    let qemu_path = bin_dir.join("qemu-system-x86_64");
    let swtpm_path = bin_dir.join("swtpm");
    write_executable(&qemu_path, "#!/bin/sh\nexit 0\n");
    write_executable(&swtpm_path, "#!/bin/sh\nexit 0\n");
    write_file(&firmware_dir.join("OVMF.fd"), "mock");

    let central_path = temp.join("central.yaml");
    write_file(
        &central_path,
        &format!(
            "host_capabilities:\n  runtime:\n    run_dir: {}/central-run\n  tpm:\n    swtpm_binary: {}/central-swtpm\n",
            temp.display(),
            temp.display(),
        ),
    );

    let vm_path = temp.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: matrix-cli-wins
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
    boot:
        firmware: uefi
    tpm:
        version: "2.0"
        backend: emulator
        model: tpm-tis
devices: {}
"#,
    );

    let run_dir = temp.join("cli-run");
    let run_dir_arg = run_dir.to_string_lossy().to_string();
    let swtpm_arg = swtpm_path.to_string_lossy().to_string();
    let ovmf_arg = firmware_dir.to_string_lossy().to_string();

    let output = run_start_dry_run(
        &vm_path,
        &bin_dir,
        &central_path,
        &[
            "--run-dir",
            &run_dir_arg,
            "--swtpm-binary",
            &swtpm_arg,
            "--ovmf-dir",
            &ovmf_arg,
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(stdout.contains("runtime_root: source=cli-override"));
    assert!(stdout.contains("swtpm_binary: source=cli-override"));
    assert!(stdout.contains("ovmf_code: source=cli-override"));
}

#[test]
fn runtime_diagnostics_show_central_defaults_when_cli_is_absent() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp = unique_temp_dir("runtime-central-defaults");
    std::fs::create_dir_all(&temp).expect("temp dir should exist");
    let bin_dir = temp.join("bin");
    let firmware_dir = temp.join("ovmf");
    let run_dir = temp.join("central-run");
    let swtpm_path = temp.join("central-swtpm");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should exist");
    std::fs::create_dir_all(&firmware_dir).expect("firmware dir should exist");

    let qemu_path = bin_dir.join("qemu-system-x86_64");
    write_executable(&qemu_path, "#!/bin/sh\nexit 0\n");
    write_executable(&swtpm_path, "#!/bin/sh\nexit 0\n");
    write_file(&firmware_dir.join("OVMF.fd"), "mock");

    let central_path = temp.join("central.yaml");
    write_file(
        &central_path,
        &format!(
            "host_capabilities:\n  runtime:\n    run_dir: {}\n  tpm:\n    swtpm_binary: {}\n  firmware:\n    search_paths:\n      - {}\n",
            run_dir.display(),
            swtpm_path.display(),
            firmware_dir.display(),
        ),
    );

    let vm_path = temp.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: matrix-central-wins
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
    boot:
        firmware: uefi
    tpm:
        version: "2.0"
        backend: emulator
        model: tpm-tis
devices: {}
"#,
    );

    let output = run_start_dry_run(&vm_path, &bin_dir, &central_path, &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        stdout,
        stderr
    );

    assert!(stdout.contains("runtime_root: source=central-config"));
    assert!(stdout.contains("swtpm_binary: source=central-config"));
    assert!(stdout.contains("ovmf_code: source=central-config"));
}

#[test]
fn runtime_diagnostics_show_optional_looking_glass_auto_downgrade() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let temp = unique_temp_dir("runtime-lg-auto");
    std::fs::create_dir_all(&temp).expect("temp dir should exist");
    let bin_dir = temp.join("bin");
    std::fs::create_dir_all(&bin_dir).expect("bin dir should exist");
    let qemu_path = bin_dir.join("qemu-system-x86_64");
    write_executable(&qemu_path, "#!/bin/sh\nexit 0\n");

    let central_path = temp.join("central.yaml");
    write_file(&central_path, "host_capabilities: {}\n");

    let vm_path = temp.join("vm.yaml");
    write_file(
        &vm_path,
        r#"
name: matrix-lg-auto
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
        ivshmem:
            enabled: true
            mem_path: /definitely/missing/kvmfr0
    cpu:
        model: host
        vcpus: 2
devices: {}
host:
    pci:
        - id: hostpci0
          device: "0000:03:00.0"
options:
    looking_glass:
        mode: auto
"#,
    );

    let output = run_start_dry_run(&vm_path, &bin_dir, &central_path, &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(stdout.contains("looking_glass_program: mode=Auto"));
    assert!(stdout.contains("source=unresolved, value=<none>"));
}
