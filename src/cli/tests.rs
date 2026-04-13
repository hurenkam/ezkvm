use super::runtime;

fn base_config() -> crate::config::VmConfig {
    crate::config::VmConfig::from_str(
        r#"
name: "test-vm"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    memory: 1024
    vcpus: 1
    cpu_model: "host"

hostpci:
    - device: "0000:03:00.0"
      id: "hostpci0"
      x_vga: true

ivshmem:
    enabled: true
    size: 128
    id: "ivshmem0"
    mem_path: "/dev/kvmfr0"

spice:
    enabled: true
    port: 5903
    addr: "0.0.0.0"
        "#,
    )
    .unwrap()
}

#[test]
fn test_build_looking_glass_launch_uses_ivshmem_mem_path() {
    let config = base_config();
    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            swtpm: None,
            remote_viewer: None,
            looking_glass: Some("looking-glass-client".to_string()),
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions {
            full_screen: Some(true),
            size: Some("1707x1067".to_string()),
            grab_keyboard: Some(true),
            escape_key: Some("KEY_F12".to_string()),
        },
    };

    let launch = runtime::build_looking_glass_launch(&config, &central_config)
        .unwrap()
        .unwrap();

    assert_eq!(launch.program, "looking-glass-client");
    assert_eq!(
        launch.args,
        vec![
            "app:shmFile=/dev/kvmfr0",
            "win:fullScreen=true",
            "win:size=1707x1067",
            "input:grabKeyboard=true",
            "input:escapeKey=KEY_F12",
            "spice:host=127.0.0.1",
            "spice:port=5903",
        ]
    );
    assert!(launch.inherit_output);
    assert!(launch.verify_running);
}

#[test]
fn test_build_looking_glass_launch_returns_none_without_tool() {
    let config = base_config();
    let central_config = crate::config::CentralConfig::default();

    let launch = runtime::build_looking_glass_launch(&config, &central_config).unwrap();
    assert!(launch.is_none());
}

#[test]
fn test_build_looking_glass_launch_returns_none_without_passthrough_gpu() {
    let config = crate::config::VmConfig::from_str(
        r#"
name: "test-vm"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    memory: 1024
    vcpus: 1
    cpu_model: "host"

ivshmem:
    enabled: true
    size: 128
    id: "ivshmem0"
    mem_path: "/dev/kvmfr0"
            "#,
    )
    .unwrap();

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            swtpm: None,
            remote_viewer: None,
            looking_glass: Some("looking-glass-client".to_string()),
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
    };

    let launch = runtime::build_looking_glass_launch(&config, &central_config).unwrap();
    assert!(launch.is_none());
}

#[test]
fn test_build_looking_glass_launch_rejects_empty_tool_path() {
    let config = base_config();
    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            swtpm: None,
            remote_viewer: None,
            looking_glass: Some("   ".to_string()),
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
    };

    let err = runtime::build_looking_glass_launch(&config, &central_config).unwrap_err();
    assert!(
        err.to_string()
            .contains("Looking Glass client path is empty")
    );
}

#[test]
fn test_tpm_emulator_requires_swtpm_tool() {
    let config = crate::config::VmConfig::from_str(
        r#"
                    name: "test-vm"
                    backend: "qemu"

                    system:
                        architecture: "x86_64"
                        machine: "q35"
                        memory: 1024
                        vcpus: 1
                        cpu_model: "host"

                    tpm:
                        version: "2.0"
                        backend: "emulator"
                        model: "tpm-tis"
                    "#,
    )
    .unwrap();

    let err = runtime::start_swtpm_if_configured(&config, &crate::config::CentralConfig::default())
        .unwrap_err();
    assert!(err.to_string().contains("tools.swtpm"));
}

#[test]
fn test_format_auxiliary_launch() {
    let launch = runtime::AuxiliaryLaunch {
        label: "Looking Glass client for ivshmem session",
        program: "looking-glass-client".to_string(),
        args: vec![
            "app:shmFile=/dev/kvmfr0".to_string(),
            "win:fullScreen=true".to_string(),
            "win:size=1707x1067".to_string(),
            "input:grabKeyboard=true".to_string(),
            "input:escapeKey=KEY_F12".to_string(),
            "spice:host=127.0.0.1".to_string(),
            "spice:port=5903".to_string(),
        ],
        inherit_output: true,
        verify_running: true,
    };

    assert_eq!(
        runtime::format_auxiliary_launch(&launch),
        "looking-glass-client app:shmFile=/dev/kvmfr0 win:fullScreen=true win:size=1707x1067 input:grabKeyboard=true input:escapeKey=KEY_F12 spice:host=127.0.0.1 spice:port=5903"
    );
}

#[test]
fn test_resolve_client_host_maps_wildcard_to_localhost() {
    assert_eq!(runtime::resolve_client_host("0.0.0.0"), "127.0.0.1");
    assert_eq!(runtime::resolve_client_host("::"), "127.0.0.1");
    assert_eq!(runtime::resolve_client_host("192.168.1.10"), "192.168.1.10");
}

#[test]
fn test_remote_viewer_is_suppressed_for_primary_passthrough_gpu() {
    let config = crate::config::VmConfig::from_str(
        r#"
        name: "test-vm"
        backend: "qemu"

        system:
          architecture: "x86_64"
          machine: "q35"
          memory: 1024
          vcpus: 1
          cpu_model: "host"

        hostpci:
          - device: "0000:03:00.0"
            id: "hostpci0"
            x_vga: true

        spice:
          enabled: true
          port: 5903
          addr: "0.0.0.0"
        "#,
    )
    .unwrap();

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            swtpm: None,
            remote_viewer: Some("remote-viewer".to_string()),
            looking_glass: None,
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
    };

    assert!(runtime::build_remote_viewer_launch(&config, &central_config).is_none());
}

#[test]
fn test_remote_viewer_is_suppressed_for_hostpci0_without_x_vga() {
    let config = crate::config::VmConfig::from_str(
        r#"
        name: "test-vm"
        backend: "qemu"

        system:
          architecture: "x86_64"
          machine: "q35"
          memory: 1024
          vcpus: 1
          cpu_model: "host"

        hostpci:
          - device: "0000:03:00.0"
            id: "hostpci0.0"

        spice:
          enabled: true
          port: 5903
          addr: "0.0.0.0"
        "#,
    )
    .unwrap();

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            swtpm: None,
            remote_viewer: Some("remote-viewer".to_string()),
            looking_glass: None,
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
    };

    assert!(runtime::build_remote_viewer_launch(&config, &central_config).is_none());
}
