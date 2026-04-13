use super::*;

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
        .map(String::as_str)
        .collect();
    assert_eq!(feature_names, vec!["hv_relaxed", "hv_time", "kvm=off"]);

    unsafe {
        std::env::remove_var("EZKVM_CONFIG");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn test_vm_config_from_file_appends_unique_nested_cpu_features() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let root = unique_test_dir("ezkvm-profile-cpu-nested-features-append");
    let profile_dir = root.join("profiles");
    std::fs::create_dir_all(&profile_dir).unwrap();

    std::fs::write(
        profile_dir.join("base.yaml"),
        r#"
system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  cpu:
    model: "host"
    vcpus: 2
    features:
            - "hv_relaxed"
            - "hv_time"
"#,
    )
    .unwrap();

    std::fs::write(
        profile_dir.join("overlay.yaml"),
        r#"
system:
  cpu:
    features:
            - "hv_time"
            - "kvm=off"
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
name: "cpu-nested-features-append-test"
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
        .cpu
        .features
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(feature_names, vec!["hv_relaxed", "hv_time", "kvm=off"]);
    assert_eq!(config.system.cpu.model, "host");
    assert_eq!(config.system.cpu.vcpus, 2);

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
