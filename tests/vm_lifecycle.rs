use std::{path::PathBuf, str::FromStr, time::Duration};

use thiserror::Error;

use ezkvm::{
    config::{
        qemu::{QemuCommandLine, QemuContext, QemuConversionError},
        EzkvmConfigSchema,
    },
    lifecycle::{
        host_config::{HostConfig, HostConfigError, InvalidVmNameError},
        process, readiness,
        start::{start, StartError},
        status::{status, StatusReport},
        stop,
        ui_client::resolve_ui_client,
        vm_handle::VmHandle,
    },
    runtime::{Chipset, Memory, Q35ChipsetBuilder, Runtime, RuntimeBuilder, SpiceDisplay, TpmState},
};

#[derive(Debug, Error)]
pub enum StartErrorMirror {
    #[error(transparent)]
    InvalidVmName(#[from] InvalidVmNameError),
    #[error(transparent)]
    HostConfig(#[from] HostConfigError),
    #[error(transparent)]
    Start(#[from] StartError),
    #[error("failed to render qemu commandline: {0}")]
    Qemu(#[from] QemuConversionError),
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let short_name: String = name.chars().take(8).collect();
        let path = PathBuf::from("target/test-artifacts")
            .join(format!("{short_name}-{:x}-{:x}", std::process::id(), unique));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write_host_yaml(
    config_dir: &std::path::Path,
    vm_dir: &std::path::Path,
    state_dir: &std::path::Path,
    qemu_path: &str,
    swtpm_path: &str,
    remote_viewer_path: &str,
    looking_glass_client_path: &str,
) {
    let content = format!(
        "qemu_path: {qemu_path}\nqemu_default_args: []\nswtpm_path: {swtpm_path}\nremote_viewer_path: {remote_viewer_path}\nremote_viewer_default_args: []\nlooking_glass_client_path: {looking_glass_client_path}\nlooking_glass_client_default_args: []\nhost_resources: []\nvm_dir: {}\nstate_dir: {}\n",
        vm_dir.display(),
        state_dir.display(),
    );
    std::fs::write(config_dir.join("host.yaml"), content).unwrap();
}

fn write_vm_yaml(vm_dir: &std::path::Path, vm_name: &str, runtime: Runtime) {
    std::fs::create_dir_all(vm_dir).unwrap();
    let schema = EzkvmConfigSchema::try_from(runtime).unwrap();
    let yaml = schema.to_styled_compact_yaml().unwrap();
    std::fs::write(vm_dir.join(format!("{vm_name}.yaml")), yaml).unwrap();
}

fn base_runtime() -> Runtime {
    RuntimeBuilder::new()
        .with_memory(Memory::new(1024))
        .with_chipset(Chipset::Q35(Q35ChipsetBuilder::new().build()))
        .build()
        .unwrap()
}

fn tpm_runtime() -> Runtime {
    RuntimeBuilder::new()
        .with_memory(Memory::new(1024))
        .with_chipset(Chipset::Q35(Q35ChipsetBuilder::new().build()))
        .with_tpmstate(TpmState::new(
            "vm1-pool:vm-108-tpmstate".to_string(),
            "v2.0".to_string(),
        ))
        .build()
        .unwrap()
}

fn spice_tpm_runtime() -> Runtime {
    RuntimeBuilder::new()
        .with_memory(Memory::new(1024))
        .with_chipset(Chipset::Q35(Q35ChipsetBuilder::new().build()))
        .with_tpmstate(TpmState::new(
            "vm1-pool:vm-108-tpmstate".to_string(),
            "v2.0".to_string(),
        ))
        .with_spice_display(SpiceDisplay::new(
            Some(5903),
            Some("127.0.0.1".to_string()),
            false,
            false,
            None,
            false,
        ))
        .build()
        .unwrap()
}

fn wait_until(predicate: impl Fn() -> bool) {
    for _ in 0..50 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(predicate(), "condition not met in time");
}

fn kill_pid(pid: u32) {
    let _ = process::terminate_pid(pid, true);
    wait_until(|| !process::is_pid_alive(pid));
}

fn read_cmdline(pid: u32) -> String {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap();
    String::from_utf8_lossy(&bytes).replace('\0', " ")
}

fn pids_for_program(program: &str) -> std::collections::BTreeSet<u32> {
    std::fs::read_dir("/proc")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .ok()
                .and_then(|bytes| bytes.split(|b| *b == 0).next().map(|arg| arg.to_vec()))
                .map(|arg0| String::from_utf8_lossy(&arg0) == program)
                .unwrap_or(false)
        })
        .collect()
}

fn pid_with_cmdline_fragment(program: &str, fragment: &str) -> Option<u32> {
    pids_for_program(program)
        .into_iter()
        .find(|pid| read_cmdline(*pid).contains(fragment))
}

#[test]
fn host_config_load_missing_returns_missing_variant() {
    let config_dir = TestDir::new("host-config-missing");
    let result = HostConfig::load(config_dir.path());
    assert!(matches!(result, Err(HostConfigError::Missing { .. })));
}

#[test]
fn host_config_load_minimal_yaml_populates_fields() {
    let config_dir = TestDir::new("host-config-valid");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_qemu"),
        "/usr/bin/swtpm",
        "/usr/bin/remote-viewer",
        "/usr/bin/looking-glass-client",
    );

    let host = HostConfig::load(config_dir.path()).unwrap();
    assert_eq!(host.vm_dir(), &vm_dir);
    assert_eq!(host.state_dir(), &state_dir);
    assert_eq!(host.qemu_path(), env!("CARGO_BIN_EXE_fake_qemu"));
    assert_eq!(host.swtpm_path(), "/usr/bin/swtpm");
    assert_eq!(host.remote_viewer_path(), "/usr/bin/remote-viewer");
    assert_eq!(
        host.looking_glass_client_path(),
        "/usr/bin/looking-glass-client"
    );
    assert!(host.host_resources().is_empty());
}

#[test]
fn resolve_vm_name_rejects_traversal_inputs() {
    assert!(HostConfig::resolve_vm_name("../../etc/passwd").is_err());
    assert!(HostConfig::resolve_vm_name("/etc/passwd").is_err());
    assert!(HostConfig::resolve_vm_name("a/b").is_err());
    assert_eq!(HostConfig::resolve_vm_name("wakiza"), Ok("wakiza"));
}

#[test]
fn wait_for_socket_returns_when_socket_becomes_ready() {
    let dir = TestDir::new("readiness-success");
    let socket_path = dir.path().join("ready.sock");
    let socket_for_thread = socket_path.clone();

    let server = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        let listener = std::os::unix::net::UnixListener::bind(&socket_for_thread).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        drop(listener);
    });

    let started = std::time::Instant::now();
    readiness::wait_for_socket(socket_path.to_str().unwrap(), Duration::from_secs(5)).unwrap();
    let elapsed = started.elapsed();

    assert!(elapsed >= Duration::from_millis(150), "elapsed: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(5), "elapsed: {elapsed:?}");
    server.join().unwrap();
}

#[test]
fn wait_for_socket_times_out_when_socket_never_appears() {
    let dir = TestDir::new("readiness-timeout");
    let socket_path = dir.path().join("missing.sock");

    let started = std::time::Instant::now();
    let error = readiness::wait_for_socket(socket_path.to_str().unwrap(), Duration::from_millis(200))
        .expect_err("socket should never become ready");
    let elapsed = started.elapsed();

    assert!(matches!(
        error,
        readiness::ReadinessError::Timeout { ref path, .. } if path == socket_path.to_str().unwrap()
    ));
    assert!(elapsed >= Duration::from_millis(150), "elapsed: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(2), "elapsed: {elapsed:?}");
}

#[test]
fn start_persists_handle_and_status_detects_staleness() {
    let config_dir = TestDir::new("vm-lifecycle");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_qemu"),
        "/usr/bin/swtpm",
        "/usr/bin/remote-viewer",
        "/usr/bin/looking-glass-client",
    );
    write_vm_yaml(&vm_dir, "wakiza", base_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    start(&host, "wakiza").unwrap();

    let handle = VmHandle::read(&state_dir, "wakiza").unwrap().unwrap();
    let pid = *handle.qemu_pid();
    wait_until(|| PathBuf::from(format!("/proc/{pid}")).exists());
    assert!(process::is_pid_alive(pid));
    assert_eq!(handle.swtpm_pid(), &None);
    assert_eq!(handle.ui_client_pid(), &None);
    assert!(!handle.qmp_socket_path().is_empty());

    assert_eq!(
        status(&state_dir, "wakiza").unwrap(),
        StatusReport::Running {
            qemu_pid: pid,
            swtpm_pid: None,
            ui_client_pid: None,
        }
    );

    let mode = std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&state_dir).unwrap().permissions()) & 0o777;
    assert_eq!(mode, 0o700);

    kill_pid(pid);

    assert_eq!(status(&state_dir, "wakiza").unwrap(), StatusReport::NotRunning);
    assert!(VmHandle::read(&state_dir, "wakiza").unwrap().is_none());
}

#[test]
fn start_with_tpm_records_swtpm_pid_and_rejects_double_start() {
    let config_dir = TestDir::new("vm-lifecycle-tpm");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_qemu"),
        env!("CARGO_BIN_EXE_fake_swtpm"),
        "/usr/bin/remote-viewer",
        "/usr/bin/looking-glass-client",
    );
    write_vm_yaml(&vm_dir, "felucia", tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    start(&host, "felucia").unwrap();

    let handle = VmHandle::read(&state_dir, "felucia").unwrap().unwrap();
    let qemu_pid = *handle.qemu_pid();
    let swtpm_pid = handle.swtpm_pid().expect("swtpm pid should be recorded");
    let tpm_socket_path = handle
        .tpm_socket_path()
        .as_deref()
        .expect("tpm socket path should be recorded")
        .to_string();

    wait_until(|| process::is_pid_alive(qemu_pid));
    wait_until(|| process::is_pid_alive(swtpm_pid));
    assert!(read_cmdline(qemu_pid).contains(&format!("path={tpm_socket_path}")));
    assert!(read_cmdline(swtpm_pid).contains(&format!("path={tpm_socket_path}")));

    assert_eq!(
        status(&state_dir, "felucia").unwrap(),
        StatusReport::Running {
            qemu_pid,
            swtpm_pid: Some(swtpm_pid),
            ui_client_pid: None,
        }
    );

    let error = start(&host, "felucia").expect_err("second start should fail");
    assert!(matches!(
        error,
        StartError::AlreadyRunning { ref vm_name, pid } if vm_name == "felucia" && pid == qemu_pid
    ));

    kill_pid(qemu_pid);
    kill_pid(swtpm_pid);
    let _ = status(&state_dir, "felucia");
}

#[test]
fn qemu_spawn_failure_cleans_up_swtpm_and_state_file() {
    let config_dir = TestDir::new("vm-lifecycle-orphan-cleanup");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        "/definitely/missing/qemu-system-x86_64",
        env!("CARGO_BIN_EXE_fake_swtpm"),
        "/usr/bin/remote-viewer",
        "/usr/bin/looking-glass-client",
    );
    write_vm_yaml(&vm_dir, "orphan-check", tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    let tpm_socket_path = state_dir.join("orphan-check.tpm.sock");

    let error = start(&host, "orphan-check").expect_err("qemu spawn should fail");
    assert!(matches!(error, StartError::QemuSpawnFailed { .. }));

    wait_until(|| {
        pid_with_cmdline_fragment(
            env!("CARGO_BIN_EXE_fake_swtpm"),
            tpm_socket_path.to_string_lossy().as_ref(),
        )
        .is_none()
    });
    assert!(VmHandle::read(&state_dir, "orphan-check").unwrap().is_none());
}

#[test]
fn test_full_lifecycle_start_and_stop() {
    let config_dir = TestDir::new("vm-lifecycle-full-roundtrip");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_qemu"),
        env!("CARGO_BIN_EXE_fake_swtpm"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
    );
    write_vm_yaml(&vm_dir, "full-cycle", spice_tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    start(&host, "full-cycle").unwrap();

    let handle = VmHandle::read(&state_dir, "full-cycle").unwrap().unwrap();
    let qemu_pid = *handle.qemu_pid();
    let swtpm_pid = handle.swtpm_pid().expect("swtpm pid should be recorded");
    let ui_client_pid = handle.ui_client_pid().expect("ui client pid should be recorded");
    let qmp_socket_path = PathBuf::from(handle.qmp_socket_path());
    let tpm_socket_path = PathBuf::from(
        handle
            .tpm_socket_path()
            .as_deref()
            .expect("tpm socket path should be recorded"),
    );

    wait_until(|| process::is_pid_alive(qemu_pid));
    wait_until(|| process::is_pid_alive(swtpm_pid));
    wait_until(|| process::is_pid_alive(ui_client_pid));
    assert!(qmp_socket_path.exists());
    assert!(tpm_socket_path.exists());
    assert_eq!(
        VmHandle::read(&state_dir, "full-cycle").unwrap().unwrap().ui_client_pid(),
        &Some(ui_client_pid)
    );

    stop::stop(&host, "full-cycle").unwrap();

    wait_until(|| {
        !process::is_pid_alive(qemu_pid)
            && !process::is_pid_alive(swtpm_pid)
            && !process::is_pid_alive(ui_client_pid)
    });
    assert!(!qmp_socket_path.exists());
    assert!(!tpm_socket_path.exists());
    assert!(VmHandle::read(&state_dir, "full-cycle").unwrap().is_none());
}

#[test]
fn test_qemu_failure_kills_orphaned_swtpm_with_full_config() {
    let config_dir = TestDir::new("vm-lifecycle-orphan-cleanup-full-config");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        "/definitely/missing/qemu-system-x86_64",
        env!("CARGO_BIN_EXE_fake_swtpm"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
    );
    write_vm_yaml(&vm_dir, "full-config-orphan", spice_tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    let tpm_socket_path = state_dir.join("full-config-orphan.tpm.sock");

    let error = start(&host, "full-config-orphan").expect_err("qemu spawn should fail");
    assert!(matches!(error, StartError::QemuSpawnFailed { .. }));

    wait_until(|| {
        pid_with_cmdline_fragment(
            env!("CARGO_BIN_EXE_fake_swtpm"),
            tpm_socket_path.to_string_lossy().as_ref(),
        )
        .is_none()
    });
    assert!(VmHandle::read(&state_dir, "full-config-orphan").unwrap().is_none());
}

#[test]
fn test_start_already_running_fails_with_full_lifecycle() {
    let config_dir = TestDir::new("vm-lifecycle-already-running-full");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_qemu"),
        env!("CARGO_BIN_EXE_fake_swtpm"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
    );
    write_vm_yaml(&vm_dir, "already-running-full", spice_tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    start(&host, "already-running-full").unwrap();

    let handle = VmHandle::read(&state_dir, "already-running-full")
        .unwrap()
        .unwrap();
    let qemu_pid = *handle.qemu_pid();
    let swtpm_pid = handle.swtpm_pid().expect("swtpm pid should be recorded");
    let ui_client_pid = handle.ui_client_pid().expect("ui client pid should be recorded");

    let error = start(&host, "already-running-full").expect_err("second start should fail");
    assert!(matches!(
        error,
        StartError::AlreadyRunning { ref vm_name, pid }
            if vm_name == "already-running-full" && pid == qemu_pid
    ));

    stop::stop(&host, "already-running-full").unwrap();

    wait_until(|| {
        !process::is_pid_alive(qemu_pid)
            && !process::is_pid_alive(swtpm_pid)
            && !process::is_pid_alive(ui_client_pid)
    });
    assert!(VmHandle::read(&state_dir, "already-running-full").unwrap().is_none());
}

pub fn render_qemu_commandline(host_config: &HostConfig, vm_name: &str) -> Result<String, StartErrorMirror> {
    let vm_path = host_config.vm_yaml_path(vm_name)?;
    let vm_yaml = std::fs::read_to_string(&vm_path).unwrap();
    let config = EzkvmConfigSchema::from_str(&vm_yaml).unwrap();
    let runtime = Runtime::try_from(config).unwrap();
    let tpm_socket_path = runtime
        .root_devices()
        .iter()
        .any(|device| device.device_kind() == ezkvm::runtime::RootDeviceKind::TpmState)
        .then(|| {
            host_config
                .state_dir()
                .join(format!("{vm_name}.tpm.sock"))
                .to_string_lossy()
                .into_owned()
        });
    let ctx = QemuContext::new(vm_name.to_string(), String::new(), tpm_socket_path);
    Ok(QemuCommandLine::try_from((runtime, ctx))?.to_string())
}

#[test]
fn ui_client_mapping_resolves_spice_viewer_for_runtime_config() {
    let config_dir = TestDir::new("vm-lifecycle-spice-mapping");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_qemu"),
        env!("CARGO_BIN_EXE_fake_swtpm"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
    );
    write_vm_yaml(&vm_dir, "spice-ui", spice_tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    let vm_yaml = std::fs::read_to_string(vm_dir.join("spice-ui.yaml")).unwrap();
    let config = EzkvmConfigSchema::from_str(&vm_yaml).unwrap();
    let runtime = Runtime::try_from(config.clone()).unwrap();

    let resolved = resolve_ui_client(config.host().display().as_ref(), &runtime, &host).unwrap();
    assert_eq!(resolved.0, env!("CARGO_BIN_EXE_fake_ui_client"));
    assert!(resolved.1.iter().any(|arg| arg.starts_with("spice://127.0.0.1:5903")));

    let qemu_cmd = render_qemu_commandline(&host, "spice-ui").unwrap();
    assert!(qemu_cmd.contains("-spice"), "qemu_cmd was: {qemu_cmd}");
    assert!(qemu_cmd.contains("port=5903"), "qemu_cmd was: {qemu_cmd}");
    assert!(qemu_cmd.contains("addr=127.0.0.1"), "qemu_cmd was: {qemu_cmd}");
}
