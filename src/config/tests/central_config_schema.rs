use super::*;

#[test]
fn host_capability_sections_parse_and_resolve() {
    let yaml = r#"
tools:
  qemu: "/usr/bin/qemu-system-x86_64"
locations:
  profile_dir: "/etc/ezkvm/profiles.d"
host_capabilities:
  runtime:
    root_directory: "/var/run/ezkvm"
    pid_dir: "/var/run/ezkvm/pids"
    socket_dir: "/var/run/ezkvm/sockets"
    log_dir: "/var/log/ezkvm"
  firmware:
    ovmf_dir: "/usr/share/OVMF"
    search_paths:
      - "/usr/share/edk2/ovmf"
      - "/usr/share/OVMF"
    secure_boot_code_files:
      - "OVMF_CODE.secboot.fd"
    code_files:
      - "OVMF_CODE.fd"
  network:
    preferred_backend: "bridge"
    bridge_helper: "/usr/lib/qemu/qemu-bridge-helper"
    bridge_name: "br0"
  tpm:
    swtpm_binary: "/usr/bin/swtpm"
    placement_mode: "socket"
    state_dir: "/var/lib/ezkvm/tpm"
    socket_dir: "/var/run/ezkvm/tpm"
  integrations:
    remote_viewer:
      program: "/usr/bin/remote-viewer"
    looking_glass:
      program: "/usr/bin/looking-glass-client"
      shared_memory_device: "/dev/kvmfr0"
"#;

    let config: CentralConfig = serde_yaml::from_str(yaml).expect("parse central config");

    assert_eq!(config.runtime_run_dir(), Some("/var/run/ezkvm"));
    assert_eq!(config.ovmf_dir(), Some("/usr/share/OVMF"));
    assert_eq!(
        config.ovmf_search_dirs_with_overrides(&RuntimeCliOverrides::default()),
        vec![
            "/usr/share/OVMF".to_string(),
            "/usr/share/edk2/ovmf".to_string()
        ]
    );
    assert_eq!(config.swtpm_program(), Some("/usr/bin/swtpm"));
    assert_eq!(config.tpm_placement_mode(), Some("socket"));
    assert_eq!(
        config.remote_viewer_program(),
        Some("/usr/bin/remote-viewer")
    );
    assert_eq!(
        config.looking_glass_program(),
        Some("/usr/bin/looking-glass-client")
    );
    assert_eq!(config.network_backend_preference(), Some("bridge"));
    assert_eq!(
        config.bridge_helper(),
        Some("/usr/lib/qemu/qemu-bridge-helper")
    );
    assert_eq!(config.bridge_name(), Some("br0"));
    assert_eq!(config.tpm_state_dir(), Some("/var/lib/ezkvm/tpm"));
    assert_eq!(config.tpm_socket_dir(), Some("/var/run/ezkvm/tpm"));
    assert_eq!(
        config.looking_glass_shared_memory_device(),
        Some("/dev/kvmfr0")
    );
}

#[test]
fn host_capability_values_override_legacy_locations_and_tools() {
    let config = CentralConfig {
        tools: ToolsConfig {
            qemu: None,
            swtpm: Some("/legacy/swtpm".to_string()),
            remote_viewer: Some("/legacy/remote-viewer".to_string()),
            looking_glass: Some("/legacy/looking-glass-client".to_string()),
        },
        locations: LocationsConfig {
            run_dir: Some("/legacy/run".to_string()),
            ovmf_dir: Some("/legacy/ovmf".to_string()),
            vm_dir: None,
            template_dir: None,
            profile_dir: None,
            pid_dir: None,
            socket_dir: None,
            log_dir: None,
        },
        looking_glass: LookingGlassOptions::default(),
        host_capabilities: HostCapabilitiesConfig {
            runtime: RuntimeHostCapabilities {
                run_dir: Some("/host/run".to_string()),
                pid_dir: None,
                socket_dir: None,
                log_dir: None,
            },
            firmware: FirmwareHostCapabilities {
                ovmf_dir: Some("/host/ovmf".to_string()),
                search_paths: Vec::new(),
                secure_boot_code_files: Vec::new(),
                code_files: Vec::new(),
            },
            network: NetworkHostCapabilities::default(),
            tpm: TpmHostCapabilities {
                swtpm_binary: Some("/host/swtpm".to_string()),
                placement_mode: None,
                state_dir: None,
                socket_dir: None,
            },
            integrations: IntegrationHostCapabilities {
                remote_viewer: ProgramCapability {
                    program: Some("/host/remote-viewer".to_string()),
                },
                looking_glass: LookingGlassCapability {
                    program: Some("/host/looking-glass-client".to_string()),
                    shared_memory_device: None,
                },
            },
        },
    };

    assert_eq!(config.runtime_run_dir(), Some("/host/run"));
    assert_eq!(config.ovmf_dir(), Some("/host/ovmf"));
    assert_eq!(config.swtpm_program(), Some("/host/swtpm"));
    assert_eq!(config.remote_viewer_program(), Some("/host/remote-viewer"));
    assert_eq!(
        config.looking_glass_program(),
        Some("/host/looking-glass-client")
    );
}

#[test]
fn legacy_vm_dir_alias_still_parses() {
    let yaml = r#"
locations:
  vms_dir: "/etc/ezkvm/vms.d"
"#;

    let config: CentralConfig = serde_yaml::from_str(yaml).expect("parse central config");
    assert_eq!(config.locations.vm_dir.as_deref(), Some("/etc/ezkvm/vms.d"));
}

#[test]
fn unknown_host_capability_field_is_rejected() {
    let yaml = r#"
host_capabilities:
  runtime:
    unknown_key: true
"#;

    let err = serde_yaml::from_str::<CentralConfig>(yaml).expect_err("unknown field must fail");
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn runtime_root_directory_alias_is_supported() {
    let yaml = r#"
host_capabilities:
  runtime:
    root_directory: "/run/alias-root"
"#;

    let config: CentralConfig = serde_yaml::from_str(yaml).expect("parse central config");
    assert_eq!(config.runtime_run_dir(), Some("/run/alias-root"));
}
