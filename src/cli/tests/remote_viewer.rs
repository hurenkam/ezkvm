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
