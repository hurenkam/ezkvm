use super::runtime;

#[test]
fn test_remote_viewer_is_suppressed_for_primary_passthrough_gpu() {
    let config = crate::config::VmConfig::from_str(
        r#"
        name: "test-vm"
        backend: "qemu"

        system:
          architecture: "x86_64"
          machine: "q35"
          memory:
            size: 1024
          cpu:
            vcpus: 1
            model: "host"
        host:
          pci:
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
            qemu: None,
            swtpm: None,
            remote_viewer: Some("remote-viewer".to_string()),
            looking_glass: None,
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
        host_capabilities: crate::config::HostCapabilitiesConfig::default(),
    };

    assert!(
        runtime::build_remote_viewer_launch(
            &config,
            &central_config,
            &crate::config::RuntimeCliOverrides::default(),
        )
        .is_none()
    );
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
          memory:
            size: 1024
          cpu:
            vcpus: 1
            model: "host"
        host:
          pci:
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
            qemu: None,
            swtpm: None,
            remote_viewer: Some("remote-viewer".to_string()),
            looking_glass: None,
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
        host_capabilities: crate::config::HostCapabilitiesConfig::default(),
    };

    assert!(
        runtime::build_remote_viewer_launch(
            &config,
            &central_config,
            &crate::config::RuntimeCliOverrides::default(),
        )
        .is_none()
    );
}

#[test]
fn test_remote_viewer_cli_override_wins_over_central_defaults() {
    let config = crate::config::VmConfig::from_str(
        r#"
        name: "test-vm"
        backend: "qemu"

        system:
          architecture: "x86_64"
          machine: "q35"
          memory:
            size: 1024
          cpu:
            vcpus: 1
            model: "host"

        spice:
          enabled: true
          port: 5903
          addr: "0.0.0.0"
        "#,
    )
    .unwrap();

    let central_config = crate::config::CentralConfig {
        tools: crate::config::ToolsConfig {
            qemu: None,
            swtpm: None,
            remote_viewer: Some("legacy-remote-viewer".to_string()),
            looking_glass: None,
        },
        locations: crate::config::LocationsConfig::default(),
        looking_glass: crate::config::LookingGlassOptions::default(),
        host_capabilities: crate::config::HostCapabilitiesConfig {
            integrations: crate::config::IntegrationHostCapabilities {
                remote_viewer: crate::config::ProgramCapability {
                    program: Some("hostcap-remote-viewer".to_string()),
                },
                looking_glass: crate::config::LookingGlassCapability::default(),
            },
            ..Default::default()
        },
    };

    let launch = runtime::build_remote_viewer_launch(
        &config,
        &central_config,
        &crate::config::RuntimeCliOverrides {
            remote_viewer_program: Some("cli-remote-viewer".to_string()),
            ..Default::default()
        },
    )
    .expect("remote-viewer launch should be built");

    assert_eq!(launch.program, "cli-remote-viewer");
}
