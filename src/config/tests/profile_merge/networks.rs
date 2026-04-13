use super::*;

#[test]
fn test_vm_config_from_file_appends_devices_networks_in_order() {
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
      model: "e1000"
      mode: "user"
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
    assert_eq!(config.devices.networks.len(), 3);

    let base_net0 = &config.devices.networks[0];
    assert_eq!(base_net0.id, "net0");
    assert_eq!(base_net0.model, "virtio-net-pci");
    assert_eq!(base_net0.mode, "user");
    assert_eq!(base_net0.mac.as_deref(), Some("52:54:00:12:34:56"));
    assert_eq!(base_net0.boot_index, None);

    let overlay_net0 = &config.devices.networks[1];
    assert_eq!(overlay_net0.id, "net0");
    assert_eq!(overlay_net0.model, "e1000");
    assert_eq!(overlay_net0.mode, "user");
    assert_eq!(overlay_net0.boot_index, Some(110));
    assert_eq!(overlay_net0.tx_queue_size, Some(256));

    let net1 = &config.devices.networks[2];
    assert_eq!(net1.id, "net1");
    assert_eq!(net1.model, "e1000");

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}
