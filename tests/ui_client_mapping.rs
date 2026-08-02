use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ezkvm::{
    config::{
        ezkvm::schema::{
            DisplaySchema, EglHeadlessSchema, LookingGlassSchema, SpiceSchema, VncSchema,
        },
        EzkvmConfigSchema,
    },
    lifecycle::{
        host_config::HostConfig,
        process,
        start::start,
        ui_client::resolve_ui_client,
        vm_handle::VmHandle,
    },
    runtime::{
        Chipset, Ivshmem, Memory, PcieAddress, Q35ChipsetBuilder, Runtime, RuntimeBuilder,
        SpiceDisplay,
    },
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

fn write_host_yaml(
    config_dir: &Path,
    vm_dir: &Path,
    state_dir: &Path,
    remote_viewer_path: &str,
    looking_glass_client_path: &str,
) {
    let content = format!(
        "qemu_path: {}\nqemu_default_args: []\nswtpm_path: /usr/bin/swtpm\nremote_viewer_path: {remote_viewer_path}\nremote_viewer_default_args: []\nlooking_glass_client_path: {looking_glass_client_path}\nlooking_glass_client_default_args: []\nhost_resources: []\nvm_dir: {}\nstate_dir: {}\n",
        env!("CARGO_BIN_EXE_fake_qemu"),
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

fn host_config_for_mapping() -> HostConfig {
    HostConfig::new(
        PathBuf::from("/example/vm.d"),
        PathBuf::from("/example/state"),
        env!("CARGO_BIN_EXE_fake_qemu").to_string(),
        vec![],
        "/usr/bin/swtpm".to_string(),
        "/usr/bin/remote-viewer".to_string(),
        vec!["--full-screen".to_string()],
        "/usr/bin/looking-glass-client".to_string(),
        vec!["--borderless".to_string()],
        vec![],
        None,
    )
}

fn runtime_with_ivshmem(mem_path: &str) -> Runtime {
    let q35 = Q35ChipsetBuilder::new()
        .with_pcie_device(
            Some(PcieAddress::new(32, 0)),
            Arc::new(Ivshmem::new(
                "ivshmem0".to_string(),
                mem_path.to_string(),
                "128M".to_string(),
            )),
        )
        .build();

    RuntimeBuilder::new()
        .with_memory(Memory::new(1024))
        .with_chipset(Chipset::Q35(q35))
        .build()
        .unwrap()
}

fn spice_runtime() -> Runtime {
    RuntimeBuilder::new()
        .with_memory(Memory::new(1024))
        .with_chipset(Chipset::Q35(Q35ChipsetBuilder::new().build()))
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

fn kill_pid(pid: u32) {
    let _ = process::terminate_pid(pid, true);
    for _ in 0..50 {
        if !process::is_pid_alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(!process::is_pid_alive(pid), "pid {pid} still alive");
}

#[test]
fn resolve_ui_client_maps_spice_to_remote_viewer() {
    let host = host_config_for_mapping();
    let display = DisplaySchema::Spice {
        spice: SpiceSchema::new(
            5900,
            "127.0.0.1".to_string(),
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
        ),
    };
    let runtime = RuntimeBuilder::new().build().unwrap();

    let (binary, args) = resolve_ui_client(Some(&display), &runtime, &host).unwrap();
    assert_eq!(binary, "/usr/bin/remote-viewer");
    assert_eq!(args, vec!["--full-screen", "spice://127.0.0.1:5900"]);
}

#[test]
fn resolve_ui_client_maps_vnc_to_remote_viewer() {
    let host = host_config_for_mapping();
    let display = DisplaySchema::Vnc {
        vnc: VncSchema::new(5901, "0.0.0.0".to_string(), false, None, false),
    };
    let runtime = RuntimeBuilder::new().build().unwrap();

    let (binary, args) = resolve_ui_client(Some(&display), &runtime, &host).unwrap();
    assert_eq!(binary, "/usr/bin/remote-viewer");
    assert_eq!(args, vec!["--full-screen", "vnc://0.0.0.0:5901"]);
}

#[test]
fn resolve_ui_client_maps_looking_glass_from_ivshmem_mem_path() {
    let host = host_config_for_mapping();
    let display = DisplaySchema::LookingGlass {
        looking_glass: LookingGlassSchema::new(0, "127.0.0.1".to_string(), false),
    };
    let runtime = runtime_with_ivshmem("/dev/shm/looking-glass");

    let (binary, args) = resolve_ui_client(Some(&display), &runtime, &host).unwrap();
    assert_eq!(binary, "/usr/bin/looking-glass-client");
    assert_eq!(
        args,
        vec!["--borderless", "-f", "/dev/shm/looking-glass"]
    );
}

#[test]
fn resolve_ui_client_returns_none_for_headless_displays() {
    let host = host_config_for_mapping();
    let display = DisplaySchema::EglHeadless {
        egl_headless: EglHeadlessSchema {},
    };
    let runtime = RuntimeBuilder::new().build().unwrap();

    assert!(resolve_ui_client(Some(&display), &runtime, &host).is_none());
    assert!(resolve_ui_client(None, &runtime, &host).is_none());
}

#[test]
fn start_keeps_vm_running_when_ui_client_launch_fails() {
    let config_dir = TestDir::new("ui-client-warning");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        "/definitely/missing/remote-viewer",
        "/usr/bin/looking-glass-client",
    );
    write_vm_yaml(&vm_dir, "spice-warning", spice_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    start(&host, "spice-warning").unwrap();

    let handle = VmHandle::read(&state_dir, "spice-warning").unwrap().unwrap();
    let qemu_pid = *handle.qemu_pid();
    assert!(process::is_pid_alive(qemu_pid));
    assert_eq!(handle.ui_client_pid(), &None);

    kill_pid(qemu_pid);
}

#[test]
fn start_records_ui_client_pid_after_fixed_delay() {
    let config_dir = TestDir::new("ui-client-success");
    let vm_dir = config_dir.path().join("vm.d");
    let state_dir = config_dir.path().join("state");
    write_host_yaml(
        config_dir.path(),
        &vm_dir,
        &state_dir,
        env!("CARGO_BIN_EXE_fake_ui_client"),
        env!("CARGO_BIN_EXE_fake_ui_client"),
    );
    write_vm_yaml(&vm_dir, "spice-success", spice_runtime());

    let host = HostConfig::load(config_dir.path()).unwrap();
    let started = Instant::now();
    start(&host, "spice-success").unwrap();

    let handle = VmHandle::read(&state_dir, "spice-success").unwrap().unwrap();
    let qemu_pid = *handle.qemu_pid();
    let ui_pid = handle.ui_client_pid().expect("ui client pid should be recorded");

    assert!(started.elapsed() >= Duration::from_secs(2));
    assert!(process::is_pid_alive(qemu_pid));
    assert!(process::is_pid_alive(ui_pid));

    kill_pid(ui_pid);
    kill_pid(qemu_pid);

    let mode = std::fs::metadata(&state_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}
