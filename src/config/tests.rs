use super::*;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

mod path_behavior {
    use super::*;

    #[test]
    fn test_central_config_load_honors_env_override() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let temp_path = std::env::temp_dir().join(format!(
            "ezkvm-central-config-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::write(&temp_path, "tools:\n  swtpm: \"/custom/swtpm\"\n").unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &temp_path);
        }

        let config = CentralConfig::load().unwrap();
        assert_eq!(config.tools.swtpm.as_deref(), Some("/custom/swtpm"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_default_central_config_search_order_prefers_directory_path() {
        assert_eq!(
            DEFAULT_CENTRAL_CONFIG_PATHS,
            &["/etc/ezkvm/ezkvm.yaml", "/etc/ezkvm.yaml"]
        );
    }
}

mod profile_merge {
    use super::*;

    #[test]
    fn test_vm_config_from_file_merges_profiles_from_profile_dir() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("windows_11.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
boot:
  firmware: "uefi"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("gpu_passthrough.yaml"),
            r#"
devices:
  displays: []
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "test-vm"
backend: "qemu"
profiles:
  - "windows_11"
  - "gpu_passthrough"
system:
  memory: 8192
  vcpus: 8
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.name, "test-vm");
        assert_eq!(config.backend, "qemu");
        assert_eq!(config.system.architecture, "x86_64");
        assert_eq!(config.system.machine, "q35");
        assert_eq!(config.system.cpu_model, "host");
        assert_eq!(config.system.memory, 8192);
        assert_eq!(config.system.vcpus, 8);
        assert!(config.devices.displays.is_empty());

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_errors_on_missing_profile_file() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-missing");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "test-vm"
backend: "qemu"
profiles:
  - "does_not_exist"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let err = VmConfig::from_file(&vm_config_path)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unknown profile 'does_not_exist'"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_errors_on_non_mapping_profile_root() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-nonmap");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("bad_profile.yaml"),
            r#"
- not
- a
- mapping
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "test-vm"
backend: "qemu"
profiles:
  - "bad_profile"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let err = VmConfig::from_file(&vm_config_path)
            .unwrap_err()
            .to_string();
        assert!(err.contains("must be a YAML mapping/object at the root"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_applies_profiles_in_listed_order() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-order");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base_cpu.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("override_machine.yaml"),
            r#"
system:
  machine: "pc"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "order-test"
backend: "qemu"
profiles:
  - "base_cpu"
  - "override_machine"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.system.machine, "pc");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_deep_merges_nested_maps() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-deep-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base_system.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
boot:
  firmware: "uefi"
  menu: true
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("secure_boot.yaml"),
            r#"
boot:
  secure_boot: true
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "deep-merge-test"
backend: "qemu"
profiles:
  - "base_system"
  - "secure_boot"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.boot.firmware.as_deref(), Some("uefi"));
        assert!(config.boot.menu);
        assert!(config.boot.secure_boot);

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_replaces_non_specialized_lists() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-list-replace");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base_system.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
devices:
  displays:
        - type: "qxl"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("replace_displays.yaml"),
            r#"
devices:
  displays:
        - type: "cirrus"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "list-replace-test"
backend: "qemu"
profiles:
  - "base_system"
  - "replace_displays"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.devices.displays.len(), 1);
        assert_eq!(config.devices.displays[0].r#type, "cirrus");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_vm_values_override_profile_values() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-vm-override");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("defaults.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "vm-override-test"
backend: "qemu"
profiles:
  - "defaults"
system:
  memory: 12288
  vcpus: 6
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.system.memory, 12288);
        assert_eq!(config.system.vcpus, 6);

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_still_runs_validation_after_profile_merge() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-validation");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("invalid_arch.yaml"),
            r#"
system:
  architecture: "invalid_arch"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "validation-test"
backend: "qemu"
profiles:
  - "invalid_arch"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let err = VmConfig::from_file(&vm_config_path)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unsupported architecture"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_hostpci_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-hostpci-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
hostpci:
  - id: "hostpci0.0"
    device: "0000:03:00.0"
    bus: "ich9-pcie-port-1"
    multifunction: true
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("gpu_overlay.yaml"),
            r#"
hostpci:
  - id: "hostpci0.0"
    addr: "0x0.0"
  - id: "hostpci0.1"
    device: "0000:03:00.1"
    bus: "ich9-pcie-port-1"
    addr: "0x0.1"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "hostpci-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "gpu_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.hostpci.len(), 2);

        let gpu0 = config
            .hostpci
            .iter()
            .find(|d| d.id == "hostpci0.0")
            .unwrap();
        assert_eq!(gpu0.device, "0000:03:00.0");
        assert_eq!(gpu0.bus.as_deref(), Some("ich9-pcie-port-1"));
        assert_eq!(gpu0.addr.as_deref(), Some("0x0.0"));
        assert!(gpu0.multifunction);

        let gpu1 = config
            .hostpci
            .iter()
            .find(|d| d.id == "hostpci0.1")
            .unwrap();
        assert_eq!(gpu1.device, "0000:03:00.1");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_devices_drives_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-drives-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        let root_disk = root.join("root.qcow2");
        let data_disk = root.join("data.raw");

        std::fs::write(
            profile_dir.join("base.yaml"),
            format!(
                r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
devices:
  drives:
    - id: "root"
      path: "{}"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#,
                root_disk.display()
            ),
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("drive_overlay.yaml"),
            format!(
                r#"
devices:
  drives:
    - id: "root"
      cache: "none"
      boot_index: 100
    - id: "data"
      path: "{}"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
"#,
                data_disk.display()
            ),
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "drive-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "drive_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.devices.drives.len(), 2);

        let root_drive = config
            .devices
            .drives
            .iter()
            .find(|d| d.id == "root")
            .unwrap();
        assert_eq!(root_drive.path, root_disk.to_str().unwrap());
        assert_eq!(root_drive.cache.as_deref(), Some("none"));
        assert_eq!(root_drive.boot_index, Some(100));

        let data_drive = config
            .devices
            .drives
            .iter()
            .find(|d| d.id == "data")
            .unwrap();
        assert_eq!(data_drive.interface, "scsi");
        assert_eq!(data_drive.format, "raw");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_devices_networks_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-networks-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
devices:
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      mode: "user"
      mac: "52:54:00:12:34:56"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("network_overlay.yaml"),
            r#"
devices:
  networks:
    - id: "net0"
      boot_index: 110
      tx_queue_size: 256
    - id: "net1"
      model: "e1000"
      mode: "user"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "network-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "network_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.devices.networks.len(), 2);

        let net0 = config
            .devices
            .networks
            .iter()
            .find(|n| n.id == "net0")
            .unwrap();
        assert_eq!(net0.model, "virtio-net-pci");
        assert_eq!(net0.mode, "user");
        assert_eq!(net0.mac.as_deref(), Some("52:54:00:12:34:56"));
        assert_eq!(net0.boot_index, Some(110));
        assert_eq!(net0.tx_queue_size, Some(256));

        let net1 = config
            .devices
            .networks
            .iter()
            .find(|n| n.id == "net1")
            .unwrap();
        assert_eq!(net1.model, "e1000");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_usb_devices_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-usb-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
usb_devices:
  - id: "usb0"
    hostbus: "1"
    hostport: "2.2"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("usb_overlay.yaml"),
            r#"
usb_devices:
  - id: "usb0"
    bus: "xhci.0"
    port: "1"
  - id: "usb1"
    hostbus: "1"
    hostport: "2.3"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "usb-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "usb_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.usb_devices.len(), 2);

        let usb0 = config.usb_devices.iter().find(|u| u.id == "usb0").unwrap();
        assert_eq!(usb0.hostbus.as_deref(), Some("1"));
        assert_eq!(usb0.hostport.as_deref(), Some("2.2"));
        assert_eq!(usb0.bus.as_deref(), Some("xhci.0"));
        assert_eq!(usb0.port.as_deref(), Some("1"));

        let usb1 = config.usb_devices.iter().find(|u| u.id == "usb1").unwrap();
        assert_eq!(usb1.hostport.as_deref(), Some("2.3"));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_scsi_controllers_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-scsi-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
scsi_controllers:
  - id: "scsihw0"
    type: "pvscsi"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("scsi_overlay.yaml"),
            r#"
scsi_controllers:
  - id: "scsihw0"
    bus: "pci.0"
    addr: "0x5"
  - id: "scsihw1"
    type: "virtio-scsi-pci"
    bus: "pci.0"
    addr: "0x6"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "scsi-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "scsi_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.scsi_controllers.len(), 2);

        let scsihw0 = config
            .scsi_controllers
            .iter()
            .find(|c| c.id == "scsihw0")
            .unwrap();
        assert_eq!(scsihw0.r#type, "pvscsi");
        assert_eq!(scsihw0.bus.as_deref(), Some("pci.0"));
        assert_eq!(scsihw0.addr.as_deref(), Some("0x5"));

        let scsihw1 = config
            .scsi_controllers
            .iter()
            .find(|c| c.id == "scsihw1")
            .unwrap();
        assert_eq!(scsihw1.r#type, "virtio-scsi-pci");

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_xhci_controllers_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-xhci-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
xhci_controllers:
  - id: "xhci"
    p2: 8
    p3: 8
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("xhci_overlay.yaml"),
            r#"
xhci_controllers:
  - id: "xhci"
    bus: "pci.1"
    addr: "0x1b"
  - id: "xhci2"
    p2: 4
    p3: 4
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "xhci-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "xhci_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.xhci_controllers.len(), 2);

        let xhci = config
            .xhci_controllers
            .iter()
            .find(|c| c.id == "xhci")
            .unwrap();
        assert_eq!(xhci.p2, Some(8));
        assert_eq!(xhci.p3, Some(8));
        assert_eq!(xhci.bus.as_deref(), Some("pci.1"));
        assert_eq!(xhci.addr.as_deref(), Some("0x1b"));

        let xhci2 = config
            .xhci_controllers
            .iter()
            .find(|c| c.id == "xhci2")
            .unwrap();
        assert_eq!(xhci2.p2, Some(4));
        assert_eq!(xhci2.p3, Some(4));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_merges_audio_devices_by_id() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-audio-id-merge");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
spice:
  enabled: true
  audio: true
audio_devices:
  - type: "ich9-intel-hda"
    id: "audiodev0"
    bus: "pci.2"
  - type: "hda-micro"
    id: "audiodev0-codec0"
    bus: "audiodev0.0"
    cad: 0
    audiodev: "spice-backend0"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("audio_overlay.yaml"),
            r#"
audio_devices:
  - type: "ich9-intel-hda"
    id: "audiodev0"
    addr: "0xc"
  - type: "hda-duplex"
    id: "audiodev0-codec1"
    bus: "audiodev0.0"
    cad: 1
    audiodev: "spice-backend0"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "audio-id-merge-test"
backend: "qemu"
profiles:
  - "base"
  - "audio_overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.audio_devices.len(), 3);

        let hda = config
            .audio_devices
            .iter()
            .find(|d| d.id == "audiodev0")
            .unwrap();
        assert_eq!(hda.r#type, "ich9-intel-hda");
        assert_eq!(hda.bus.as_deref(), Some("pci.2"));
        assert_eq!(hda.addr.as_deref(), Some("0xc"));

        let codec0 = config
            .audio_devices
            .iter()
            .find(|d| d.id == "audiodev0-codec0")
            .unwrap();
        assert_eq!(codec0.r#type, "hda-micro");

        let codec1 = config
            .audio_devices
            .iter()
            .find(|d| d.id == "audiodev0-codec1")
            .unwrap();
        assert_eq!(codec1.r#type, "hda-duplex");
        assert_eq!(codec1.cad, Some(1));

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_appends_unique_cpu_features() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-cpu-features-append");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
  cpu_features:
        - name: "hv_relaxed"
        - name: "hv_time"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("overlay.yaml"),
            r#"
system:
  cpu_features:
        - name: "hv_time"
        - name: "kvm=off"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "cpu-features-append-test"
backend: "qemu"
profiles:
  - "base"
  - "overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        let feature_names: Vec<&str> = config
            .system
            .cpu_features
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(feature_names, vec!["hv_relaxed", "hv_time", "kvm=off"]);

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_appends_unique_machine_options() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-machine-options-append");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
  machine_options:
        - "hpet=off"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("overlay.yaml"),
            r#"
system:
  machine_options:
        - "hpet=off"
        - "smm=on"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "machine-options-append-test"
backend: "qemu"
profiles:
  - "base"
  - "overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(config.system.machine_options, vec!["hpet=off", "smm=on"]);

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_vm_config_from_file_appends_unique_global_options() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = unique_test_dir("ezkvm-profile-global-options-append");
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).unwrap();

        std::fs::write(
            profile_dir.join("base.yaml"),
            r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 2
  cpu_model: "host"
options:
  enable_kvm: true
  daemonize: false
  global_options:
        - "kvm-pit.lost_tick_policy=discard"
"#,
        )
        .unwrap();

        std::fs::write(
            profile_dir.join("overlay.yaml"),
            r#"
options:
  global_options:
        - "kvm-pit.lost_tick_policy=discard"
        - "ICH9-LPC.disable_s3=1"
"#,
        )
        .unwrap();

        let central_config_path = root.join("ezkvm.yaml");
        std::fs::write(
            &central_config_path,
            format!("locations:\n  profile_dir: \"{}\"\n", profile_dir.display()),
        )
        .unwrap();

        let vm_config_path = root.join("vm.yaml");
        std::fs::write(
            &vm_config_path,
            r#"
name: "global-options-append-test"
backend: "qemu"
profiles:
  - "base"
  - "overlay"
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_config_path);
        }

        let config = VmConfig::from_file(&vm_config_path).unwrap();
        assert_eq!(
            config.options.global_options,
            vec!["kvm-pit.lost_tick_policy=discard", "ICH9-LPC.disable_s3=1"]
        );

        unsafe {
            std::env::remove_var("EZKVM_CONFIG");
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
