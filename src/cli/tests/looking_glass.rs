use super::{base_config, runtime};

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
            program: None,
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
