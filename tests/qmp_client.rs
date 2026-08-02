use std::{
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ezkvm::{
    config::EzkvmConfigSchema,
    lifecycle::{
        host_config::HostConfig,
        kill, process,
        qmp::{QmpClient, QmpError},
        reset,
        start::start,
        stop,
        vm_handle::VmHandle,
    },
    runtime::{Chipset, Memory, Q35ChipsetBuilder, Runtime, RuntimeBuilder, SpiceDisplay, TpmState},
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

struct FakeQmpServer {
    _dir: TestDir,
    socket_path: PathBuf,
    commands: Arc<Mutex<Vec<String>>>,
}

impl FakeQmpServer {
    fn spawn<F>(name: &str, handler: F) -> Self
    where
        F: FnOnce(UnixStream, Arc<Mutex<Vec<String>>>) + Send + 'static,
    {
        let dir = TestDir::new(name);
        let socket_path = dir.path().join("qmp.sock");
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let thread_commands = Arc::clone(&commands);

        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                handler(stream, thread_commands);
            }
        });

        Self {
            _dir: dir,
            socket_path,
            commands,
        }
    }

    fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}

fn write_json_line(stream: &mut UnixStream, value: serde_json::Value) {
    let payload = serde_json::to_string(&value).unwrap();
    writeln!(stream, "{payload}").unwrap();
}

fn read_command(reader: &mut BufReader<UnixStream>, commands: &Arc<Mutex<Vec<String>>>) -> String {
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).unwrap();
    assert!(bytes > 0, "expected command line");
    let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    let command = value
        .get("execute")
        .and_then(serde_json::Value::as_str)
        .unwrap()
        .to_string();
    commands.lock().unwrap().push(command.clone());
    command
}

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(predicate(), "condition not met in time");
}

fn new_host_config(state_dir: &Path, stop_timeout: Option<u64>) -> HostConfig {
    HostConfig::new(
        PathBuf::from("target/test-artifacts/vm.d"),
        state_dir.to_path_buf(),
        env!("CARGO_BIN_EXE_fake_qemu").to_string(),
        Vec::new(),
        "/usr/bin/swtpm".to_string(),
        "/usr/bin/remote-viewer".to_string(),
        Vec::new(),
        "/usr/bin/looking-glass-client".to_string(),
        Vec::new(),
        Vec::new(),
        stop_timeout,
    )
}

fn spawn_fake_qemu(socket_path: &Path, ignore_powerdown: bool) -> Child {
    spawn_fake_qemu_with_opts(socket_path, ignore_powerdown, false)
}

fn spawn_fake_qemu_with_opts(socket_path: &Path, ignore_powerdown: bool, ignore_quit: bool) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fake_qemu"));
    command
        .arg("-qmp")
        .arg(format!("unix:{},server=on,wait=off", socket_path.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ignore_powerdown {
        command.env("FAKE_QEMU_IGNORE_POWERDOWN", "1");
    }
    if ignore_quit {
        command.env("FAKE_QEMU_IGNORE_QUIT", "1");
    }
    let child = command.spawn().unwrap();
    wait_until(Duration::from_secs(2), || {
        UnixStream::connect(socket_path).is_ok()
    });
    child
}

fn spawn_ui_client_standin() -> Child {
    Command::new(env!("CARGO_BIN_EXE_fake_qemu"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn write_vm_handle(
    state_dir: &Path,
    vm_name: &str,
    qemu_pid: u32,
    ui_client_pid: Option<u32>,
    qmp_socket_path: &Path,
) {
    VmHandle::new(
        vm_name.to_string(),
        qemu_pid,
        None,
        ui_client_pid,
        qmp_socket_path.display().to_string(),
        None,
    )
    .write(state_dir)
    .unwrap();
}

fn wait_for_exit(child: &mut Child) {
    let _ = child.wait();
}

fn write_host_yaml(
    config_dir: &Path,
    vm_dir: &Path,
    state_dir: &Path,
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

fn write_vm_yaml(vm_dir: &Path, vm_name: &str, runtime: Runtime) {
    std::fs::create_dir_all(vm_dir).unwrap();
    let schema = EzkvmConfigSchema::try_from(runtime).unwrap();
    let yaml = schema.to_styled_compact_yaml().unwrap();
    std::fs::write(vm_dir.join(format!("{vm_name}.yaml")), yaml).unwrap();
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

#[test]
fn connect_succeeds_after_greeting_and_capabilities() {
    let server = FakeQmpServer::spawn("qmp-connect", |mut stream, commands| {
        write_json_line(
            &mut stream,
            serde_json::json!({ "QMP": { "version": {}, "capabilities": [] } }),
        );
        let reader = stream.try_clone().unwrap();
        let mut reader = BufReader::new(reader);
        assert_eq!(read_command(&mut reader, &commands), "qmp_capabilities");
        write_json_line(&mut stream, serde_json::json!({ "return": {} }));
    });

    QmpClient::connect(server.socket_path().to_str().unwrap()).unwrap();
    assert_eq!(server.commands(), vec!["qmp_capabilities"]);
}

#[test]
fn connect_returns_no_greeting_when_server_closes_immediately() {
    let server = FakeQmpServer::spawn("qmp-no-greeting", |_stream, _commands| {});

    let err = match QmpClient::connect(server.socket_path().to_str().unwrap()) {
        Ok(_) => panic!("expected connect to fail without greeting"),
        Err(err) => err,
    };
    assert!(matches!(err, QmpError::NoGreeting));
}

#[test]
fn system_powerdown_always_follows_capabilities_handshake() {
    let server = FakeQmpServer::spawn("qmp-powerdown", |mut stream, commands| {
        write_json_line(
            &mut stream,
            serde_json::json!({ "QMP": { "version": {}, "capabilities": [] } }),
        );
        let reader = stream.try_clone().unwrap();
        let mut reader = BufReader::new(reader);

        assert_eq!(read_command(&mut reader, &commands), "qmp_capabilities");
        write_json_line(&mut stream, serde_json::json!({ "return": {} }));

        assert_eq!(read_command(&mut reader, &commands), "system_powerdown");
        write_json_line(&mut stream, serde_json::json!({ "return": {} }));
    });

    let mut client = QmpClient::connect(server.socket_path().to_str().unwrap()).unwrap();
    client.system_powerdown().unwrap();

    assert_eq!(
        server.commands(),
        vec!["qmp_capabilities", "system_powerdown"]
    );
}

#[test]
fn quit_tolerates_premature_eof() {
    let server = FakeQmpServer::spawn("qmp-quit-eof", |mut stream, commands| {
        write_json_line(
            &mut stream,
            serde_json::json!({ "QMP": { "version": {}, "capabilities": [] } }),
        );
        let reader = stream.try_clone().unwrap();
        let mut reader = BufReader::new(reader);

        assert_eq!(read_command(&mut reader, &commands), "qmp_capabilities");
        write_json_line(&mut stream, serde_json::json!({ "return": {} }));
        assert_eq!(read_command(&mut reader, &commands), "quit");
    });

    let mut client = QmpClient::connect(server.socket_path().to_str().unwrap()).unwrap();
    client.quit().unwrap();
    assert_eq!(server.commands(), vec!["qmp_capabilities", "quit"]);
}

#[test]
fn system_reset_is_fire_and_forget() {
    let server = FakeQmpServer::spawn("qmp-reset", |mut stream, commands| {
        write_json_line(
            &mut stream,
            serde_json::json!({ "QMP": { "version": {}, "capabilities": [] } }),
        );
        let reader = stream.try_clone().unwrap();
        let mut reader = BufReader::new(reader);

        assert_eq!(read_command(&mut reader, &commands), "qmp_capabilities");
        write_json_line(&mut stream, serde_json::json!({ "return": {} }));
        assert_eq!(read_command(&mut reader, &commands), "system_reset");
        thread::sleep(Duration::from_secs(2));
    });

    let mut client = QmpClient::connect(server.socket_path().to_str().unwrap()).unwrap();
    let started = Instant::now();
    client.system_reset_fire_and_forget().unwrap();
    assert!(started.elapsed() < Duration::from_millis(500));
    wait_until(Duration::from_secs(1), || server.commands().len() == 2);
    assert_eq!(server.commands(), vec!["qmp_capabilities", "system_reset"]);
}

#[test]
fn system_powerdown_surfaces_qmp_error_details() {
    let server = FakeQmpServer::spawn("qmp-error", |mut stream, commands| {
        write_json_line(
            &mut stream,
            serde_json::json!({ "QMP": { "version": {}, "capabilities": [] } }),
        );
        let reader = stream.try_clone().unwrap();
        let mut reader = BufReader::new(reader);

        assert_eq!(read_command(&mut reader, &commands), "qmp_capabilities");
        write_json_line(&mut stream, serde_json::json!({ "return": {} }));
        assert_eq!(read_command(&mut reader, &commands), "system_powerdown");
        write_json_line(
            &mut stream,
            serde_json::json!({ "error": { "class": "CommandNotFound", "desc": "powerdown blocked" } }),
        );
    });

    let mut client = QmpClient::connect(server.socket_path().to_str().unwrap()).unwrap();
    let err = client.system_powerdown().unwrap_err();
    assert!(matches!(
        err,
        QmpError::CommandFailed {
            command,
            desc
        } if command == "system_powerdown" && desc == "powerdown blocked"
    ));
}

#[test]
fn stop_sends_powerdown_and_waits_for_exit() {
    let dir = TestDir::new("stop-powerdown");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu(&socket_path, false);
    write_vm_handle(&state_dir, "vm", qemu.id(), None, &socket_path);

    let host = new_host_config(&state_dir, None);
    stop::stop(&host, "vm").unwrap();

    assert!(!process::is_pid_alive(qemu.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_none());
    let mode = std::fs::metadata(&state_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    wait_for_exit(&mut qemu);
}

#[test]
fn kill_sends_quit_and_waits_for_exit() {
    let dir = TestDir::new("kill-quit");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu(&socket_path, false);
    write_vm_handle(&state_dir, "vm", qemu.id(), None, &socket_path);

    let host = new_host_config(&state_dir, None);
    kill::kill(&host, "vm").unwrap();

    assert!(!process::is_pid_alive(qemu.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_none());
    wait_for_exit(&mut qemu);
}

#[test]
fn kill_falls_back_to_sigterm_when_qmp_connect_fails() {
    // VMGR-04: when the QMP socket can't even be connected to (e.g. the socket
    // path is stale/missing), `kill` must still terminate the process via a
    // SIGTERM fallback rather than erroring out or reaching straight for SIGKILL.
    let dir = TestDir::new("kill-noqmp");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let missing_socket_path = state_dir.join("vm.qmp.sock");
    let mut standin = spawn_ui_client_standin();
    write_vm_handle(&state_dir, "vm", standin.id(), None, &missing_socket_path);

    let host = new_host_config(&state_dir, None);
    kill::kill(&host, "vm").unwrap();

    assert!(!process::is_pid_alive(standin.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_none());
    wait_for_exit(&mut standin);
}

#[test]
fn kill_falls_back_to_sigterm_when_quit_does_not_exit_in_time() {
    // VMGR-04: when QMP `quit` is accepted but the process does not exit in
    // time, `kill` must escalate to SIGTERM (not go straight to SIGKILL).
    let dir = TestDir::new("kill-noresp");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu_with_opts(&socket_path, false, true);
    write_vm_handle(&state_dir, "vm", qemu.id(), None, &socket_path);

    let host = new_host_config(&state_dir, None);
    kill::kill(&host, "vm").unwrap();

    assert!(!process::is_pid_alive(qemu.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_none());
    wait_for_exit(&mut qemu);
}

#[test]
fn reset_returns_promptly_and_keeps_vm_running() {
    let dir = TestDir::new("reset-fire-and-forget");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu(&socket_path, false);
    write_vm_handle(&state_dir, "vm", qemu.id(), None, &socket_path);

    let host = new_host_config(&state_dir, None);
    let started = Instant::now();
    reset::reset(&host, "vm").unwrap();

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(process::is_pid_alive(qemu.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_some());

    process::terminate_pid(qemu.id(), true).unwrap();
    wait_until(Duration::from_secs(2), || !process::is_pid_alive(qemu.id()));
    wait_for_exit(&mut qemu);
}

#[test]
fn kill_also_terminates_tracked_ui_client_and_removes_state() {
    let dir = TestDir::new("kill-ui-client");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu(&socket_path, false);
    let mut ui_client = spawn_ui_client_standin();
    write_vm_handle(
        &state_dir,
        "vm",
        qemu.id(),
        Some(ui_client.id()),
        &socket_path,
    );

    let host = new_host_config(&state_dir, None);
    kill::kill(&host, "vm").unwrap();

    assert!(!process::is_pid_alive(qemu.id()));
    assert!(!process::is_pid_alive(ui_client.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_none());
    wait_for_exit(&mut qemu);
    wait_for_exit(&mut ui_client);
}

#[test]
fn stop_does_not_auto_escalate_without_explicit_timeout() {
    let dir = TestDir::new("stop-no-escalation");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu(&socket_path, true);
    write_vm_handle(&state_dir, "vm", qemu.id(), None, &socket_path);

    let host = new_host_config(&state_dir, None);
    let (sender, receiver) = mpsc::channel();
    let host_for_thread = host.clone();
    thread::spawn(move || {
        let result = stop::stop(&host_for_thread, "vm");
        sender.send(result).unwrap();
    });

    assert!(receiver.recv_timeout(Duration::from_millis(700)).is_err());
    assert!(process::is_pid_alive(qemu.id()));

    process::terminate_pid(qemu.id(), true).unwrap();
    wait_until(Duration::from_secs(2), || !process::is_pid_alive(qemu.id()));
    assert!(
        receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_ok()
    );
    wait_for_exit(&mut qemu);
}

#[test]
fn stop_escalates_to_quit_when_timeout_is_configured() {
    let dir = TestDir::new("stop-escalation");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let socket_path = state_dir.join("vm.qmp.sock");
    let mut qemu = spawn_fake_qemu(&socket_path, true);
    write_vm_handle(&state_dir, "vm", qemu.id(), None, &socket_path);

    let host = new_host_config(&state_dir, Some(1));
    let started = Instant::now();
    stop::stop(&host, "vm").unwrap();

    assert!(started.elapsed() >= Duration::from_secs(1));
    assert!(!process::is_pid_alive(qemu.id()));
    assert!(VmHandle::read(&state_dir, "vm").unwrap().is_none());
    wait_for_exit(&mut qemu);
}

#[test]
fn start_creates_qmp_socket_in_secure_state_dir_under_full_config() {
    let config_dir = TestDir::new("qmp-secure-state-dir");
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
    write_vm_yaml(&vm_dir, "socket-perms", spice_tpm_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    start(&host, "socket-perms").unwrap();

    let handle = VmHandle::read(&state_dir, "socket-perms").unwrap().unwrap();
    let qmp_socket_path = PathBuf::from(handle.qmp_socket_path());
    let qemu_pid = *handle.qemu_pid();
    let swtpm_pid = handle.swtpm_pid().expect("swtpm pid should be recorded");
    let ui_client_pid = handle.ui_client_pid().expect("ui client pid should be recorded");

    wait_until(Duration::from_secs(2), || qmp_socket_path.exists());
    assert_eq!(qmp_socket_path.parent(), Some(state_dir.as_path()));
    let mode = std::fs::metadata(&state_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);

    stop::stop(&host, "socket-perms").unwrap();
    wait_until(Duration::from_secs(2), || {
        !process::is_pid_alive(qemu_pid)
            && !process::is_pid_alive(swtpm_pid)
            && !process::is_pid_alive(ui_client_pid)
    });
    assert!(!qmp_socket_path.exists());
    assert!(VmHandle::read(&state_dir, "socket-perms").unwrap().is_none());
}
