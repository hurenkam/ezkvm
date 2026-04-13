use super::*;

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
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: "host"
controllers:
  xhci:
    - id: "xhci"
      p2: 8
      p3: 8
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("xhci_overlay.yaml"),
        r#"
controllers:
  xhci:
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
    assert_eq!(config.controllers.xhci.len(), 2);

    let xhci = config
        .controllers
        .xhci
        .iter()
        .find(|c| c.id == "xhci")
        .unwrap();
    assert_eq!(xhci.p2, Some(8));
    assert_eq!(xhci.p3, Some(8));
    assert_eq!(xhci.bus.as_deref(), Some("pci.1"));
    assert_eq!(xhci.addr.as_deref(), Some("0x1b"));

    let xhci2 = config
        .controllers
        .xhci
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
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: "host"
spice:
  enabled: true
  audio: true
devices:
  audio:
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
devices:
  audio:
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
    assert_eq!(config.devices.audio.len(), 3);

    let hda = config
        .devices
        .audio
        .iter()
        .find(|d| d.id == "audiodev0")
        .unwrap();
    assert_eq!(hda.r#type, "ich9-intel-hda");
    assert_eq!(hda.bus.as_deref(), Some("pci.2"));
    assert_eq!(hda.addr.as_deref(), Some("0xc"));

    let codec0 = config
        .devices
        .audio
        .iter()
        .find(|d| d.id == "audiodev0-codec0")
        .unwrap();
    assert_eq!(codec0.r#type, "hda-micro");

    let codec1 = config
        .devices
        .audio
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
