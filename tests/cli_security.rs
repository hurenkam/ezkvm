use std::{
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use ezkvm::lifecycle::{
    host_config::HostConfig,
    vm_handle::{VmHandle, VmHandleError},
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let short_name: String = name.chars().take(8).collect();
        let path = PathBuf::from("target/test-artifacts")
            .join(format!("{short_name}-{:x}-{:x}", std::process::id(), unique));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write_host_yaml(config_dir: &Path, vm_dir: &Path, state_dir: &Path) {
    let content = format!(
        "qemu_path: {}
qemu_default_args: []
swtpm_path: /usr/bin/swtpm
remote_viewer_path: /usr/bin/remote-viewer
remote_viewer_default_args: []
looking_glass_client_path: /usr/bin/looking-glass-client
looking_glass_client_default_args: []
host_resources: []
vm_dir: {}
state_dir: {}
",
        env!("CARGO_BIN_EXE_fake_qemu"),
        vm_dir.display(),
        state_dir.display(),
    );
    std::fs::write(config_dir.join("host.yaml"), content).unwrap();
}

#[test]
fn resolve_vm_name_rejects_path_traversal_identically_for_all_lifecycle_verbs() {
    let expected = HostConfig::resolve_vm_name("../escape").unwrap_err();
    for verb in ["start", "stop", "kill", "reset", "status"] {
        let actual = HostConfig::resolve_vm_name("../escape").unwrap_err();
        assert_eq!(actual, expected, "{verb} should reject identically");
    }
    for accepted in ["felucia-108", "wakiza"] {
        assert_eq!(HostConfig::resolve_vm_name(accepted), Ok(accepted));
    }
}

#[test]
fn vm_yaml_path_stays_within_vm_dir() {
    let host = HostConfig::new(
        PathBuf::from("/example/vm.d"),
        PathBuf::from("/example/state"),
        env!("CARGO_BIN_EXE_fake_qemu").to_string(),
        vec![],
        "/usr/bin/swtpm".to_string(),
        "/usr/bin/remote-viewer".to_string(),
        vec![],
        "/usr/bin/looking-glass-client".to_string(),
        vec![],
        vec![],
        None,
    );
    let path = host.vm_yaml_path("felucia-108").unwrap();
    assert_eq!(path.parent(), Some(host.vm_dir().as_path()));
}

#[test]
fn cli_surfaces_invalid_vm_name_without_panicking() {
    let config_dir = TestDir::new("cli-security");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(config_dir.path(), &vm_dir, &state_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .args([
            "--config-dir",
            config_dir.path().to_str().unwrap(),
            "start",
            "../x",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: invalid VM name '../x'"), "stderr was: {stderr}");
    assert!(!stderr.contains("panicked at"), "stderr was: {stderr}");
}

#[test]
fn stop_cli_surfaces_invalid_vm_name_with_same_error_shape() {
    let config_dir = TestDir::new("cli-security-stop");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(config_dir.path(), &vm_dir, &state_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_ezkvm"))
        .args([
            "--config-dir",
            config_dir.path().to_str().unwrap(),
            "stop",
            "../escape",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error: invalid VM name '../escape'"),
        "stderr was: {stderr}"
    );
    assert!(!stderr.contains("panicked at"), "stderr was: {stderr}");
}

#[test]
fn vm_handle_rejects_symlinked_state_files() {
    let dir = TestDir::new("cli-security-symlink-state");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let target = dir.path().join("sentinel.txt");
    std::fs::write(&target, "do-not-touch").unwrap();
    let state_path = state_dir.join("felucia.state");
    symlink(&target, &state_path).unwrap();

    let read_error = VmHandle::read(&state_dir, "felucia").unwrap_err();
    assert!(matches!(
        read_error,
        VmHandleError::UnsafePath { ref path } if path == &state_path
    ));

    let write_error = VmHandle::new(
        "felucia".to_string(),
        1234,
        None,
        None,
        state_dir.join("felucia.qmp.sock").display().to_string(),
        None,
    )
    .write(&state_dir)
    .unwrap_err();
    assert!(matches!(
        write_error,
        VmHandleError::UnsafePath { ref path } if path == &state_path
    ));
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "do-not-touch");
}
