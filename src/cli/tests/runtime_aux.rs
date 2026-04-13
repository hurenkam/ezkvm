use super::{base_config, runtime};

#[test]
fn test_tpm_emulator_requires_swtpm_tool() {
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
fn test_base_config_parses() {
    let config = base_config();
    assert_eq!(config.name, "test-vm");
}
