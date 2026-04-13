use super::*;

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
    assert_eq!(
        config.boot.cmdline.as_ref().unwrap(),
        "console=ttyS0 root=/dev/vda1"
    );
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
    assert!(!config.options.enable_kvm);
    assert!(config.options.daemonize);
    assert_eq!(
        config.options.uefi_vars.as_ref().unwrap(),
        "/path/to/vars.fd"
    );
}

#[test]
fn test_config_with_advanced_qemu_options() {
    let temp_dir = std::env::temp_dir();
    let pid_file = temp_dir.join("test-vm-advanced.pid");
    let log_dir = temp_dir.join("test-vm-advanced-logs");

    let yaml = format!(
        r#"
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
"#,
        pid_file.display(),
        log_dir.display()
    );

    let config = VmConfig::from_str(&yaml).unwrap();
    assert_eq!(config.system.machine, "pc-q35-8.1+pve0");
    assert_eq!(config.system.machine_options, vec!["hpet=off"]);
    assert_eq!(config.boot.boot_order, vec!["disk", "network"]);
    assert!(config.boot.menu);
    assert!(config.boot.strict);
    assert_eq!(config.boot.reboot_timeout, Some(1000));
    assert_eq!(
        config.boot.splash.as_deref(),
        Some("/usr/share/qemu-server/bootsplash.jpg")
    );
    assert!(config.options.nodefaults);
    assert_eq!(
        config.options.pid_file.as_deref(),
        Some(pid_file.to_str().unwrap())
    );
    assert_eq!(
        config.options.log_dir.as_deref(),
        Some(log_dir.to_str().unwrap())
    );
    assert_eq!(config.options.log_keep, Some(5));
    assert_eq!(
        config.options.global_options,
        vec!["kvm-pit.lost_tick_policy=discard"]
    );
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
    assert_eq!(
        disk.initiator.as_deref(),
        Some("iqn.1993-08.org.debian:01:622fd71731a1")
    );
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
