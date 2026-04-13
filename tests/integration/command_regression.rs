use super::*;

fn index_of(args: &[String], needle: &str) -> usize {
    args.iter()
        .position(|arg| arg == needle)
        .unwrap_or_else(|| panic!("missing argument flag: {needle}"))
}

#[test]
fn test_refactored_build_command_keeps_boot_platform_and_option_ordering() {
    let yaml = r#"
name: "regression-order-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  machine_options:
    - "hpet=off"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

boot:
  firmware: "uefi"
  boot_order: ["disk", "cdrom", "network"]
  menu: true
  strict: true
  reboot_timeout: 1000
  splash: "/bootsplash.jpg"
  kernel: "/boot/vmlinuz"
  initrd: "/boot/initrd.img"
  cmdline: "console=ttyS0"

tpm:
  version: "2.0"
  backend: "emulator"
  model: "tpm-crb"

spice:
  enabled: true
  port: 5903
  addr: "0.0.0.0"
  disable_ticketing: true
  audio: true
  vdagent: true

audio_devices:
  - type: "ich9-intel-hda"
    id: "audiodev0"
  - type: "hda-duplex"
    id: "audiocodec0"
    bus: "audiodev0.0"
    cad: 1
    audiodev: "spice-backend0"

options:
  enable_kvm: true
  daemonize: false
  nodefaults: true
  global_options:
    - "kvm-pit.lost_tick_policy=discard"
  rtc:
    base: "localtime"
    driftfix: "slew"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    let manager = QemuManager::new(config, CentralConfig::default());
    let args = manager.build_command().unwrap().into_inner();

    let boot_idx = index_of(&args, "-boot");
    let tpm_idx = index_of(&args, "-chardev");
    let spice_idx = index_of(&args, "-spice");
    let pidfile_idx = index_of(&args, "-pidfile");
    let nodefaults_idx = index_of(&args, "-nodefaults");
    let rtc_idx = index_of(&args, "-rtc");

    assert!(boot_idx < tpm_idx);
    assert!(tpm_idx < spice_idx);
    assert!(spice_idx < pidfile_idx);
    assert!(pidfile_idx < nodefaults_idx);
    assert!(nodefaults_idx < rtc_idx);

    assert!(args.contains(&"-kernel".to_string()));
    assert!(args.contains(&"-initrd".to_string()));
    assert!(args.contains(&"-append".to_string()));
    assert!(args.iter().any(|arg| {
        arg.contains("order=cdn")
            && arg.contains("menu=on")
            && arg.contains("strict=on")
            && arg.contains("reboot-timeout=1000")
            && arg.contains("splash=/bootsplash.jpg")
    }));
    assert!(args.iter().any(|arg| arg.contains("if=pflash,unit=0")));
    assert!(
        args.iter()
            .any(|arg| { arg.contains("emulator,id=tpmdev") && arg.contains("chardev=tpmchar") })
    );
    assert!(args.iter().any(|arg| arg.contains("port=5903")));
    assert!(
        args.iter()
            .any(|arg| arg.contains("kvm-pit.lost_tick_policy=discard"))
    );
}

#[test]
fn test_refactored_command_builder_keeps_device_conversion_fragments() {
    let yaml = r#"
name: "regression-device-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

devices:
  drives:
    - id: "scsi0"
      path: "/var/lib/vm/scsi0.raw"
      interface: "scsi"
      type: "disk"
      format: "raw"
      discard: true
      controller: "scsihw0"
      scsi_id: 0
      boot_index: 100
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      backend:
        type: "user"
      mac: "52:54:00:12:34:56"
      rx_queue_size: 1024
      tx_queue_size: 256
      boot_index: 102
      bus: "pci.0"
      addr: "0x12"

scsi_controllers:
  - id: "scsihw0"
    type: "pvscsi"
"#;

    let config = VmConfig::from_str(yaml).unwrap();
    let manager = QemuManager::new(config, CentralConfig::default());
    let args = manager.build_command().unwrap().into_inner();

    assert!(args.iter().any(|arg| {
        arg.contains("file=/var/lib/vm/scsi0.raw")
            && arg.contains("if=none")
            && arg.contains("id=drive-scsi0")
            && arg.contains("discard=unmap")
    }));
    assert!(args.iter().any(|arg| {
        arg.contains("scsi-hd,drive=drive-scsi0,id=scsi0")
            && arg.contains("bus=scsihw0.0")
            && arg.contains("scsi-id=0")
            && arg.contains("bootindex=100")
    }));
    assert!(args.iter().any(|arg| arg == "type=user,id=net0"));
    assert!(args.iter().any(|arg| {
        arg.contains("virtio-net-pci,netdev=net0")
            && arg.contains("mac=52:54:00:12:34:56")
            && arg.contains("bus=pci.0")
            && arg.contains("addr=0x12")
            && arg.contains("rx_queue_size=1024")
            && arg.contains("tx_queue_size=256")
            && arg.contains("bootindex=102")
    }));
}
