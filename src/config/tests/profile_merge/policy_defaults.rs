use super::*;

#[test]
fn test_vm_config_from_file_applies_drive_policies_by_selector() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-drive-policies");
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
policies:
  drives:
    - match:
        interface: "scsi"
        type: "disk"
      defaults:
        format: "raw"
        cache: "none"
        aio: "io_uring"
        controller: "scsihw0"
    - match:
        interface: "ide"
        type: "cdrom"
      defaults:
        format: "raw"
        readonly: true
        bus: "ide.1"
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
name: "drive-policy-test"
backend: "qemu"
profiles:
  - "base"
devices:
  drives:
    - id: "scsi0"
      path: "/tmp/scsi0.raw"
      interface: "scsi"
      type: "disk"
      cache: "writeback"
    - id: "ide0"
      path: ""
      interface: "ide"
      type: "cdrom"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();

    let scsi0 = config
        .devices
        .drives
        .iter()
        .find(|drive| drive.id == "scsi0")
        .unwrap();
    assert_eq!(scsi0.format, "raw");
    assert_eq!(scsi0.cache.as_deref(), Some("writeback"));
    assert_eq!(scsi0.aio.as_deref(), Some("io_uring"));
    assert_eq!(scsi0.controller.as_deref(), Some("scsihw0"));

    let ide0 = config
        .devices
        .drives
        .iter()
        .find(|drive| drive.id == "ide0")
        .unwrap();
    assert_eq!(ide0.format, "raw");
    assert!(ide0.readonly);
    assert_eq!(ide0.bus.as_deref(), Some("ide.1"));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_applies_network_policies_for_structured_and_legacy_backends() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-network-policies");
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
policies:
  networks:
    - match:
        backend_type: "tap"
      defaults:
        model: "virtio-net-pci"
        rx_queue_size: 1024
        backend:
          script: "/usr/libexec/qemu-server/pve-bridge"
          downscript: "/usr/libexec/qemu-server/pve-bridgedown"
          vhost: true
    - match:
        backend_type: "user"
      defaults:
        model: "e1000"
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
name: "network-policy-test"
backend: "qemu"
profiles:
  - "base"
devices:
  networks:
    - id: "net0"
      backend:
        type: "tap"
        ifname: "tap0"
      tx_queue_size: 256
    - id: "net1"
      mode: "user"
      mac: "52:54:00:12:34:56"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();

    let net0 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net0")
        .unwrap();
    assert_eq!(net0.model, "virtio-net-pci");
    assert_eq!(net0.rx_queue_size, Some(1024));
    assert_eq!(net0.tx_queue_size, Some(256));
    let net0_backend = net0.backend.as_ref().unwrap();
    assert_eq!(net0_backend.backend_type, "tap");
    assert_eq!(net0_backend.ifname.as_deref(), Some("tap0"));
    assert_eq!(
        net0_backend.script.as_deref(),
        Some("/usr/libexec/qemu-server/pve-bridge")
    );
    assert_eq!(
        net0_backend.downscript.as_deref(),
        Some("/usr/libexec/qemu-server/pve-bridgedown")
    );
    assert_eq!(net0_backend.vhost, Some(true));

    let net1 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net1")
        .unwrap();
    assert_eq!(net1.model, "e1000");
    assert_eq!(net1.mode, "user");
    assert!(net1.backend.is_none());

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_applies_drive_scsi_id_placement_policy() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-drive-placement");
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
policies:
  drives:
    - match:
        interface: "scsi"
        type: "disk"
        controller: "scsihw0"
      placement:
        scsi_id:
          scope: "controller"
          start: 0
          step: 1
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
name: "drive-placement-test"
backend: "qemu"
profiles:
  - "base"
devices:
  drives:
    - id: "scsi0"
      path: "/tmp/scsi0.raw"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
    - id: "scsi1"
      path: "/tmp/scsi1.raw"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
      scsi_id: 3
    - id: "scsi2"
      path: "/tmp/scsi2.raw"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();

    let scsi0 = config
        .devices
        .drives
        .iter()
        .find(|drive| drive.id == "scsi0")
        .unwrap();
    assert_eq!(scsi0.scsi_id, Some(0));

    let scsi1 = config
        .devices
        .drives
        .iter()
        .find(|drive| drive.id == "scsi1")
        .unwrap();
    assert_eq!(scsi1.scsi_id, Some(3));

    let scsi2 = config
        .devices
        .drives
        .iter()
        .find(|drive| drive.id == "scsi2")
        .unwrap();
    assert_eq!(scsi2.scsi_id, Some(1));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_applies_network_addr_placement_policy() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-network-placement");
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
policies:
  networks:
    - match:
        model: "virtio-net-pci"
      placement:
        addr:
          scope: "bus"
          bus: "pci.0"
          start: "0x12"
          step: 1
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
name: "network-placement-test"
backend: "qemu"
profiles:
  - "base"
devices:
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      mode: "user"
      bus: "pci.0"
    - id: "net1"
      model: "virtio-net-pci"
      mode: "user"
      bus: "pci.0"
      addr: "0x14"
    - id: "net2"
      model: "virtio-net-pci"
      mode: "user"
      bus: "pci.0"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();

    let net0 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net0")
        .unwrap();
    assert_eq!(net0.addr.as_deref(), Some("0x12"));

    let net1 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net1")
        .unwrap();
    assert_eq!(net1.addr.as_deref(), Some("0x14"));

    let net2 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net2")
        .unwrap();
    assert_eq!(net2.addr.as_deref(), Some("0x13"));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_rejects_duplicate_drive_scsi_id_in_same_scope() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-drive-scsi-collision");
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
name: "drive-scsi-collision-test"
backend: "qemu"
profiles:
  - "base"
devices:
  drives:
    - id: "scsi0"
      path: "/tmp/scsi0.raw"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
      scsi_id: 0
    - id: "scsi1"
      path: "/tmp/scsi1.raw"
      interface: "scsi"
      type: "disk"
      format: "raw"
      controller: "scsihw0"
      scsi_id: 0
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let err = VmConfig::from_file(&vm_config_path).unwrap_err();
    assert!(
        err.to_string()
            .contains("Duplicate drive scsi_id 0 in scope 'controller:scsihw0'")
    );

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_rejects_duplicate_network_addr_in_same_bus_scope() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-network-addr-collision");
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
name: "network-addr-collision-test"
backend: "qemu"
profiles:
  - "base"
devices:
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      mode: "user"
      bus: "pci.0"
      addr: "0x12"
    - id: "net1"
      model: "virtio-net-pci"
      mode: "user"
      bus: "pci.0"
      addr: "18"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let err = VmConfig::from_file(&vm_config_path).unwrap_err();
    assert!(
        err.to_string()
            .contains("Duplicate network addr 0x12 in scope 'bus:pci.0'")
    );

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_policy_precedence_and_explicit_override() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-policy-precedence");
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
policies:
  networks:
    - match:
        backend_type: "tap"
      defaults:
        model: "e1000"
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("overlay.yaml"),
        r#"
policies:
  networks:
    - match:
        backend_type: "tap"
      defaults:
        model: "virtio-net-pci"
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
name: "policy-precedence-test"
backend: "qemu"
profiles:
  - "base"
  - "overlay"
policies:
  networks:
    - match:
        backend_type: "tap"
      defaults:
        model: "e1000e"
devices:
  networks:
    - id: "net0"
      backend:
        type: "tap"
        ifname: "tap0"
    - id: "net1"
      backend:
        type: "tap"
        ifname: "tap1"
      model: "rtl8139"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();

    let net0 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net0")
        .unwrap();
    assert_eq!(net0.model, "e1000e");

    let net1 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net1")
        .unwrap();
    assert_eq!(net1.model, "rtl8139");

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_applies_network_defaults_and_placement_to_legacy_tap_mode() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-legacy-tap-placement");
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
policies:
  networks:
    - match:
        backend_type: "tap"
      defaults:
        model: "virtio-net-pci"
        rx_queue_size: 1024
      placement:
        addr:
          scope: "bus"
          bus: "pci.0"
          start: "0x12"
          step: 1
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
name: "legacy-tap-placement-test"
backend: "qemu"
profiles:
  - "base"
devices:
  networks:
    - id: "net0"
      mode: "tap,ifname=tap0,script=no,downscript=no,vhost=on"
      bus: "pci.0"
    - id: "net1"
      mode: "tap,ifname=tap1,script=no,downscript=no,vhost=on"
      bus: "pci.0"
      addr: "0x14"
"#,
    )
    .unwrap();

    unsafe {
        std::env::set_var("EZKVM_CONFIG", &central_config_path);
    }

    let config = VmConfig::from_file(&vm_config_path).unwrap();

    let net0 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net0")
        .unwrap();
    assert_eq!(net0.model, "virtio-net-pci");
    assert_eq!(net0.rx_queue_size, Some(1024));
    assert_eq!(
        net0.mode,
        "tap,ifname=tap0,script=no,downscript=no,vhost=on"
    );
    assert_eq!(net0.addr.as_deref(), Some("0x12"));
    assert!(net0.backend.is_none());
    let net0_backend = net0.resolved_backend().unwrap();
    assert_eq!(net0_backend.backend_type, "tap");
    assert_eq!(net0_backend.ifname.as_deref(), Some("tap0"));
    assert_eq!(net0_backend.vhost, Some(true));

    let net1 = config
        .devices
        .networks
        .iter()
        .find(|network| network.id == "net1")
        .unwrap();
    assert_eq!(net1.addr.as_deref(), Some("0x14"));

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}
