use super::{base_config, runtime};

#[test]
fn test_build_looking_glass_launch_uses_ivshmem_mem_path() {
    let config = base_config();
    let temp_dir = std::env::temp_dir().join("ezkvm-lg-launch-test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let lg_binary = temp_dir.join("looking-glass-client");
    std::fs::write(&lg_binary, b"#!/bin/sh\nexit 0\n").expect("create fake client");

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            qemu: None,
            swtpm: None,
            remote_viewer: None,
            looking_glass: Some(lg_binary.to_string_lossy().to_string()),
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions {
            mode: None,
            program: None,
            full_screen: Some(true),
            size: Some("1707x1067".to_string()),
            grab_keyboard: Some(true),
            escape_key: Some("KEY_F12".to_string()),
        },
        host_capabilities: crate::config::HostCapabilitiesConfig::default(),
    };

    let launch = runtime::build_looking_glass_launch(
        &config,
        &central_config,
        &crate::config::RuntimeCliOverrides::default(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(launch.program, lg_binary.to_string_lossy().to_string());
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

    let _ = std::fs::remove_file(&lg_binary);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_build_looking_glass_launch_returns_none_without_tool() {
    let config = base_config();
    let central_config = crate::config::CentralConfig::default();

    let launch = runtime::build_looking_glass_launch(
        &config,
        &central_config,
        &crate::config::RuntimeCliOverrides::default(),
    )
    .unwrap();
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
    memory:
        size: 1024
        ivshmem:
            enabled: true
            size: 128
            id: "ivshmem0"
            mem_path: "/dev/kvmfr0"
    cpu:
        vcpus: 1
        model: "host"
            "#,
    )
    .unwrap();

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            qemu: None,
            swtpm: None,
            remote_viewer: None,
            looking_glass: Some("looking-glass-client".to_string()),
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
        host_capabilities: crate::config::HostCapabilitiesConfig::default(),
    };

    let launch = runtime::build_looking_glass_launch(
        &config,
        &central_config,
        &crate::config::RuntimeCliOverrides::default(),
    )
    .unwrap();
    assert!(launch.is_none());
}

#[test]
fn test_build_looking_glass_launch_rejects_empty_tool_path() {
    let mut config = base_config();
    config.options.looking_glass = Some(crate::config::LookingGlassOptions {
        mode: Some("explicit".to_string()),
        program: Some("   ".to_string()),
        full_screen: None,
        size: None,
        grab_keyboard: None,
        escape_key: None,
    });
    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            qemu: None,
            swtpm: None,
            remote_viewer: None,
            looking_glass: None,
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
        host_capabilities: crate::config::HostCapabilitiesConfig::default(),
    };

    let err = runtime::build_looking_glass_launch(
        &config,
        &central_config,
        &crate::config::RuntimeCliOverrides::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("explicit mode"));
}

#[test]
fn test_looking_glass_cli_override_has_highest_precedence() {
    let temp_dir = std::env::temp_dir().join("ezkvm-lg-cli-override-test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let cli_binary = temp_dir.join("looking-glass-client");
    std::fs::write(&cli_binary, b"#!/bin/sh\nexit 0\n").expect("create fake client");

    let mut config = base_config();
    config.options.looking_glass = Some(crate::config::LookingGlassOptions {
        mode: None,
        program: Some("vm-local-looking-glass".to_string()),
        full_screen: None,
        size: None,
        grab_keyboard: None,
        escape_key: None,
    });

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            qemu: None,
            swtpm: None,
            remote_viewer: None,
            looking_glass: Some("legacy-looking-glass".to_string()),
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions {
            mode: None,
            program: Some("central-looking-glass".to_string()),
            full_screen: None,
            size: None,
            grab_keyboard: None,
            escape_key: None,
        },
        host_capabilities: crate::config::HostCapabilitiesConfig {
            integrations: crate::config::IntegrationHostCapabilities {
                remote_viewer: crate::config::ProgramCapability::default(),
                looking_glass: crate::config::LookingGlassCapability {
                    program: Some("hostcap-looking-glass".to_string()),
                    shared_memory_device: None,
                },
            },
            ..Default::default()
        },
    };

    let launch = runtime::build_looking_glass_launch(
        &config,
        &central_config,
        &crate::config::RuntimeCliOverrides {
            looking_glass_program: Some(cli_binary.to_string_lossy().to_string()),
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();

    assert_eq!(launch.program, cli_binary.to_string_lossy().to_string());

    let _ = std::fs::remove_file(&cli_binary);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_build_looking_glass_launch_returns_none_for_disabled_mode() {
    let mut config = base_config();
    config.options.looking_glass = Some(crate::config::LookingGlassOptions {
        mode: Some("disabled".to_string()),
        program: Some("looking-glass-client".to_string()),
        full_screen: None,
        size: None,
        grab_keyboard: None,
        escape_key: None,
    });

    let launch = runtime::build_looking_glass_launch(
        &config,
        &crate::config::CentralConfig::default(),
        &crate::config::RuntimeCliOverrides::default(),
    )
    .expect("launch build should succeed");

    assert!(launch.is_none());
}
