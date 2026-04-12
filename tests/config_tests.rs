//! Unit tests for configuration parsing and validation

use ezkvm::config::VmConfig;
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
mod tests {
    use super::*;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_basic_config_parsing() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 2
  cpu_model: "host"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.name, "test-vm");
        assert_eq!(config.backend, "qemu");
        assert_eq!(config.system.architecture, "x86_64");
        assert_eq!(config.system.machine, "q35");
        assert_eq!(config.system.memory, 1024);
        assert_eq!(config.system.vcpus, 2);
        assert_eq!(config.system.cpu_model, "host");
    }

    #[test]
    fn test_config_with_devices() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

devices:
  drives:
    - id: "root"
      path: "/path/to/disk.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
      cache: "none"
      aio: "io_uring"
      detect_zeroes: "unmap"

  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "user"
      mac: "52:54:00:12:34:56"
      rx_queue_size: 1024
      tx_queue_size: 256
      boot_index: 102
      bus: "pci.0"
      addr: "0x12"

  displays:
    - type: "virtio-gpu"
      vram: 256
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.devices.drives.len(), 1);
        assert_eq!(config.devices.networks.len(), 1);
        assert_eq!(config.devices.displays.len(), 1);

        let drive = &config.devices.drives[0];
        assert_eq!(drive.id, "root");
        assert_eq!(drive.path, "/path/to/disk.qcow2");
        assert_eq!(drive.interface, "virtio");
        assert_eq!(drive.cache.as_deref(), Some("none"));
        assert_eq!(drive.aio.as_deref(), Some("io_uring"));
        assert_eq!(drive.detect_zeroes.as_deref(), Some("unmap"));

        let network = &config.devices.networks[0];
        assert_eq!(network.id, "net0");
        assert_eq!(network.model, "virtio-net");
        assert_eq!(network.mac.as_ref().unwrap(), "52:54:00:12:34:56");
        assert_eq!(network.rx_queue_size, Some(1024));
        assert_eq!(network.tx_queue_size, Some(256));
        assert_eq!(network.boot_index, Some(102));
        assert_eq!(network.bus.as_deref(), Some("pci.0"));
        assert_eq!(network.addr.as_deref(), Some("0x12"));

        let display = &config.devices.displays[0];
        assert_eq!(display.r#type, "virtio-gpu");
        assert_eq!(display.vram.unwrap(), 256);
    }

    #[test]
    fn test_empty_cdrom_path_is_allowed() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

devices:
  drives:
    - id: "ide2"
      path: ""
      interface: "ide"
      type: "cdrom"
      format: "raw"
      readonly: true
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.devices.drives[0].r#type, "cdrom");
        assert!(config.devices.drives[0].path.is_empty());
    }

    #[test]
    fn test_empty_disk_path_is_rejected() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

devices:
  drives:
    - id: "disk0"
      path: ""
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#;

        let err = VmConfig::from_str(yaml).unwrap_err();
        assert!(err.to_string().contains("Drive path cannot be empty unless drive type is cdrom"));
    }

    #[test]
    fn test_env_var_substitution() {
        let _guard = env_lock().lock().unwrap();

        unsafe {
            std::env::set_var("TEST_MEMORY", "4096");
            std::env::set_var("TEST_CPUS", "4");
        }

        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: ${TEST_MEMORY}
  vcpus: ${TEST_CPUS}
  cpu_model: "host"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.system.memory, 4096);
        assert_eq!(config.system.vcpus, 4);

        // Clean up
        unsafe {
            std::env::remove_var("TEST_MEMORY");
            std::env::remove_var("TEST_CPUS");
        }
    }

    #[test]
    fn test_simple_env_var_substitution() {
        let _guard = env_lock().lock().unwrap();

        unsafe {
            std::env::set_var("HOME", "/home/test");
        }

        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

devices:
  drives:
    - id: "root"
      path: "$HOME/disk.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.devices.drives[0].path, "/home/test/disk.qcow2");

        // Clean up
        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_missing_env_var() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: ${MISSING_VAR}
  vcpus: 2
  cpu_model: "host"
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MISSING_VAR"));
    }

    #[test]
    fn test_invalid_config_missing_required_fields() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  # missing machine, memory, vcpus, cpu_model
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_with_boot_options() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

boot:
  firmware: "uefi"
  boot_order: ["disk", "cdrom"]
  kernel: "/boot/vmlinuz"
  initrd: "/boot/initrd.img"
  cmdline: "console=ttyS0 root=/dev/vda1"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.boot.firmware.as_ref().unwrap(), "uefi");
        assert_eq!(config.boot.boot_order, vec!["disk", "cdrom"]);
        assert_eq!(config.boot.kernel.as_ref().unwrap(), "/boot/vmlinuz");
        assert_eq!(config.boot.initrd.as_ref().unwrap(), "/boot/initrd.img");
        assert_eq!(config.boot.cmdline.as_ref().unwrap(), "console=ttyS0 root=/dev/vda1");
    }

    #[test]
    fn test_config_with_options() {
        let yaml = r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

options:
  enable_kvm: false
  daemonize: true
  uefi_vars: "/path/to/vars.fd"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.options.enable_kvm, false);
        assert_eq!(config.options.daemonize, true);
        assert_eq!(config.options.uefi_vars.as_ref().unwrap(), "/path/to/vars.fd");
    }

    #[test]
    fn test_config_with_advanced_qemu_options() {
        let temp_dir = std::env::temp_dir();
        let pid_file = temp_dir.join("test-vm-advanced.pid");
        let log_dir = temp_dir.join("test-vm-advanced-logs");
        
        let yaml = format!(r#"
name: "test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "pc-q35-8.1+pve0"
  machine_options:
    - "hpet=off"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

boot:
  boot_order: ["disk", "network"]
  menu: true
  strict: true
  reboot_timeout: 1000
  splash: "/usr/share/qemu-server/bootsplash.jpg"

options:
  enable_kvm: true
  daemonize: false
  nodefaults: true
  pid_file: "{}"
  log_dir: "{}"
  log_keep: 5
  global_options:
    - "kvm-pit.lost_tick_policy=discard"
  rtc:
    base: "localtime"
    driftfix: "slew"
"#, pid_file.display(), log_dir.display());

        let config = VmConfig::from_str(&yaml).unwrap();
        assert_eq!(config.system.machine, "pc-q35-8.1+pve0");
        assert_eq!(config.system.machine_options, vec!["hpet=off"]);
        assert_eq!(config.boot.boot_order, vec!["disk", "network"]);
        assert!(config.boot.menu);
        assert!(config.boot.strict);
        assert_eq!(config.boot.reboot_timeout, Some(1000));
        assert_eq!(config.boot.splash.as_deref(), Some("/usr/share/qemu-server/bootsplash.jpg"));
        assert!(config.options.nodefaults);
        assert_eq!(config.options.pid_file.as_deref(), Some(pid_file.to_str().unwrap()));
        assert_eq!(config.options.log_dir.as_deref(), Some(log_dir.to_str().unwrap()));
        assert_eq!(config.options.log_keep, Some(5));
        assert_eq!(config.options.global_options, vec!["kvm-pit.lost_tick_policy=discard"]);
        let rtc = config.options.rtc.as_ref().unwrap();
        assert_eq!(rtc.base.as_deref(), Some("localtime"));
        assert_eq!(rtc.driftfix.as_deref(), Some("slew"));
    }

    #[test]
    fn test_config_with_iscsi_initiator_and_auth() {
        let yaml = r#"
name: "iscsi-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

iscsi_disks:
  - id: "iscsi0"
    portal: "10.0.0.1:3260"
    target: "iqn.2024-01.example:storage.vm0"
    lun: 1
    initiator: "iqn.1993-08.org.debian:01:622fd71731a1"
    username: "chap-user"
    password: "chap-pass"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        let disk = &config.iscsi_disks[0];
        assert_eq!(disk.initiator.as_deref(), Some("iqn.1993-08.org.debian:01:622fd71731a1"));
        assert_eq!(disk.username.as_deref(), Some("chap-user"));
        assert_eq!(disk.password.as_deref(), Some("chap-pass"));
    }

    #[test]
    fn test_config_with_hostpci_guest_placement() {
        let yaml = r#"
name: "gpu-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 4
  cpu_model: "host"

hostpci:
  - device: "0000:03:00.0"
    id: "hostpci0.0"
    pcie: true
    x_vga: true
    bus: "ich9-pcie-port-1"
    addr: "0x0.0"
    multifunction: true
  - device: "0000:03:00.1"
    id: "hostpci0.1"
    pcie: true
    bus: "ich9-pcie-port-1"
    addr: "0x0.1"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.hostpci[0].bus.as_deref(), Some("ich9-pcie-port-1"));
        assert_eq!(config.hostpci[0].addr.as_deref(), Some("0x0.0"));
        assert!(config.hostpci[0].multifunction);
        assert_eq!(config.hostpci[1].addr.as_deref(), Some("0x0.1"));
    }

    #[test]
    fn test_config_with_spice_audio_devices() {
        let yaml = r#"
name: "audio-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 4
  cpu_model: "host"

spice:
  enabled: true
  port: 5903
  addr: "0.0.0.0"
  disable_ticketing: true
  audio: true

audio_devices:
  - type: "ich9-intel-hda"
    id: "audiodev0"
    bus: "pci.2"
    addr: "0xc"
  - type: "hda-micro"
    id: "audiodev0-codec0"
    bus: "audiodev0.0"
    cad: 0
    audiodev: "spice-backend0"
  - type: "hda-duplex"
    id: "audiodev0-codec1"
    bus: "audiodev0.0"
    cad: 1
    audiodev: "spice-backend0"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.audio_devices.len(), 3);
        assert_eq!(config.audio_devices[0].r#type, "ich9-intel-hda");
        assert_eq!(config.audio_devices[0].bus.as_deref(), Some("pci.2"));
        assert_eq!(config.audio_devices[0].addr.as_deref(), Some("0xc"));
        assert_eq!(config.audio_devices[1].cad, Some(0));
        assert_eq!(config.audio_devices[1].audiodev.as_deref(), Some("spice-backend0"));
    }

    #[test]
    fn test_spice_audio_requires_audio_devices() {
        let yaml = r#"
name: "audio-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 4
  cpu_model: "host"

spice:
  enabled: true
  audio: true
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SPICE audio requires at least one configured audio device"));
    }

    #[test]
    fn test_config_with_input_devices() {
        let yaml = r#"
name: "input-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

spice:
  enabled: true
  vdagent: true

input_devices:
  - type: "virtio-mouse"
  - type: "virtio-keyboard"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.input_devices.len(), 2);
        assert_eq!(config.input_devices[0].r#type, "virtio-mouse");
        assert_eq!(config.input_devices[1].r#type, "virtio-keyboard");
    }

    #[test]
    fn test_duplicate_input_devices_are_rejected() {
        let yaml = r#"
name: "input-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

input_devices:
  - type: "virtio-mouse"
  - type: "virtio-mouse"
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate input device type configured"));
    }

    #[test]
    fn test_config_with_ivshmem_bus_and_mem_path() {
        let yaml = r#"
name: "ivshmem-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

ivshmem:
  enabled: true
  size: 128
  vectors: 1
  id: "ivshmem0"
  bus: "pcie.0"
  mem_path: "/dev/kvmfr0"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        let ivshmem = config.ivshmem.as_ref().unwrap();
        assert_eq!(ivshmem.id, "ivshmem0");
        assert_eq!(ivshmem.bus.as_deref(), Some("pcie.0"));
        assert_eq!(ivshmem.mem_path, "/dev/kvmfr0");
    }

    #[test]
    fn test_ivshmem_mem_path_must_be_absolute() {
        let yaml = r#"
name: "ivshmem-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

ivshmem:
  enabled: true
  mem_path: "dev/kvmfr0"
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ivshmem mem_path must be an absolute path"));
    }

    #[test]
    fn test_config_with_xhci_controller_and_usb_hostport() {
        let yaml = r#"
name: "usb-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

xhci_controllers:
  - id: "xhci"
    p2: 15
    p3: 15
    bus: "pci.1"
    addr: "0x1b"

usb_devices:
  - id: "usb0"
    hostbus: "1"
    hostport: "2.2"
    bus: "xhci.0"
    port: "1"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.xhci_controllers.len(), 1);
        assert_eq!(config.xhci_controllers[0].p2, Some(15));
        assert_eq!(config.xhci_controllers[0].p3, Some(15));
        assert_eq!(config.xhci_controllers[0].bus.as_deref(), Some("pci.1"));
        assert_eq!(config.xhci_controllers[0].addr.as_deref(), Some("0x1b"));
        assert_eq!(config.usb_devices[0].hostbus.as_deref(), Some("1"));
        assert_eq!(config.usb_devices[0].hostport.as_deref(), Some("2.2"));
    }

    #[test]
    fn test_usb_device_accepts_proxmox_host_form() {
        let yaml = r#"
name: "usb-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

usb_devices:
  - id: "usb0"
    host: "1-2.2"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.usb_devices[0].host, "1-2.2");
    }

    #[test]
    fn test_config_with_non_vfio_device_placement() {
        let yaml = r#"
name: "placement-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 4
  cpu_model: "host"

devices:
  drives:
    - id: "scsi0"
      path: "/dev/vm1/vm-108-boot"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
      scsi_id: 0
      boot_index: 100
    - id: "ide2"
      path: ""
      interface: "ide"
      type: "cdrom"
      format: "raw"
      readonly: true
      bus: "ide.1"
      unit: 0
      boot_index: 101

guest_agent:
  enabled: true
  socket_path: "/var/run/qemu-server/108.qga"
  bus: "pci.0"
  addr: "0x8"

ballooning:
  enabled: true
  model: "virtio-balloon-pci"
  id: "balloon0"
  bus: "pci.0"
  addr: "0x3"

scsi_controllers:
  - id: "scsihw0"
    type: "pvscsi"
    bus: "pci.0"
    addr: "0x5"
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.devices.drives[0].scsi_id, Some(0));
        assert_eq!(config.devices.drives[0].boot_index, Some(100));
        assert_eq!(config.devices.drives[1].bus.as_deref(), Some("ide.1"));
        assert_eq!(config.devices.drives[1].unit, Some(0));
        assert_eq!(config.guest_agent.as_ref().unwrap().bus.as_deref(), Some("pci.0"));
        assert_eq!(config.ballooning.as_ref().unwrap().addr.as_deref(), Some("0x3"));
        assert_eq!(config.scsi_controllers[0].addr.as_deref(), Some("0x5"));
    }

    #[test]
    fn test_config_with_real_serial_backends() {
        let yaml = r#"
name: "serial-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 1
  cpu_model: "host"

devices:
  serials:
    - type: "file"
      path: "/tmp/serial.log"
    - type: "socket"
      host: "127.0.0.1"
      socket_port: 4444
      server: true
      wait: false
"#;

        let config = VmConfig::from_str(yaml).unwrap();
        assert_eq!(config.devices.serials.len(), 2);
        assert_eq!(config.devices.serials[0].path.as_deref(), Some("/tmp/serial.log"));
        assert_eq!(config.devices.serials[1].host.as_deref(), Some("127.0.0.1"));
        assert_eq!(config.devices.serials[1].socket_port, Some(4444));
    }

    #[test]
    fn test_serial_file_backend_requires_path() {
        let yaml = r#"
name: "serial-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 1
  cpu_model: "host"

devices:
  serials:
    - type: "file"
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Serial file backend requires a non-empty path"));
    }

    #[test]
    fn test_serial_socket_backend_requires_host_and_port() {
        let yaml = r#"
name: "serial-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 1
  cpu_model: "host"

devices:
  serials:
    - type: "socket"
      host: ""
      socket_port: 0
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Serial socket backend requires a non-empty host") || error.contains("Serial socket backend requires a TCP port between 1 and 65535"));
    }

    #[test]
    fn test_unsupported_display_vram_is_rejected() {
        let yaml = r#"
name: "display-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 1
  cpu_model: "host"

devices:
  displays:
    - type: "cirrus"
      vram: 64
"#;

        let result = VmConfig::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not support configurable VRAM"));
    }
}