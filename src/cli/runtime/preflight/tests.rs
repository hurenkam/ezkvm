use super::{
    apparmor_glob_matches, apparmor_rules_allow_path, bridge_acl_allows_bridge,
    ensure_bridge_helper_acl_allows_bridge, ensure_bridge_helper_acl_exists,
    ensure_bridge_helper_acl_requirements, run_runtime_preflight, swtpm_apparmor_path_is_allowed,
    swtpm_apparmor_socket_path_is_allowed,
};
use crate::config::{CentralConfig, RuntimeCliOverrides, VmConfig};
use std::path::Path;

fn unique_temp_path(suffix: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ezkvm-{}-{}", suffix, nanos))
}

#[test]
fn preflight_succeeds_for_minimal_portable_vm() {
    let config = VmConfig::from_str(
        r#"
name: preflight-ok
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
    )
    .expect("vm config should parse");

    let result = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides::default(),
        "/bin/sh",
    )
    .expect("preflight should pass");

    assert!(result.optional_warnings().is_empty());
}

#[test]
fn preflight_fails_when_readconfig_path_is_missing() {
    let config = VmConfig::from_str(
        r#"
name: preflight-missing-readconfig
backend: qemu
system:
    architecture: x86_64
    machine: q35
    readconfig:
        - /definitely/missing/ezkvm-q35.cfg
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
devices: {}
"#,
    )
    .expect("vm config should parse");

    let err = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides::default(),
        "/bin/sh",
    )
    .expect_err("preflight should fail");

    assert!(err.to_string().contains(
        "system.readconfig path '/definitely/missing/ezkvm-q35.cfg' does not exist on this host"
    ));
}

#[test]
fn preflight_accepts_existing_readconfig_path() {
    let config = VmConfig::from_str(
        r#"
name: preflight-existing-readconfig
backend: qemu
system:
    architecture: x86_64
    machine: q35
    readconfig:
        - /etc/hosts
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
devices: {}
"#,
    )
    .expect("vm config should parse");

    let result = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides::default(),
        "/bin/sh",
    );

    assert!(result.is_ok());
}

#[test]
fn preflight_fails_when_tpm_emulator_has_no_swtpm_binary() {
    let config = VmConfig::from_str(
        r#"
name: preflight-tpm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
  tpm:
    version: "2.0"
    backend: emulator
    model: tpm-tis
devices: {}
"#,
    )
    .expect("vm config should parse");

    let err = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides {
            swtpm_binary: Some("/definitely/missing/swtpm".to_string()),
            ..Default::default()
        },
        "/bin/sh",
    )
    .expect_err("preflight should fail");

    assert!(
        err.to_string()
            .contains("required swtpm binary '/definitely/missing/swtpm' is not available")
    );
}

#[test]
fn preflight_fails_when_tpm_backend_uri_local_path_missing() {
    let config = VmConfig::from_str(
        r#"
name: preflight-tpm-uri-missing
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
    tpm:
        version: "2.0"
        backend: emulator
        state_backend_uri: file:///definitely/missing/vm-tpm-state
devices: {}
"#,
    )
    .expect("vm config should parse");

    let err = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides {
            swtpm_binary: Some("/bin/sh".to_string()),
            tpm_socket_path: Some("/run/libvirt/qemu/swtpm/preflight.sock".to_string()),
            ..Default::default()
        },
        "/bin/sh",
    )
    .expect_err("preflight should fail");

    assert!(err.to_string().contains(
        "system.tpm.state_backend_uri resolves to local path '/definitely/missing/vm-tpm-state'"
    ));
}

#[test]
fn preflight_accepts_existing_tpm_backend_uri_local_path() {
    let config = VmConfig::from_str(
        r#"
name: preflight-tpm-uri-existing
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
    tpm:
        version: "2.0"
        backend: emulator
        state_backend_uri: file:///etc/hosts
devices: {}
"#,
    )
    .expect("vm config should parse");

    let result = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides {
            swtpm_binary: Some("/bin/sh".to_string()),
            tpm_socket_path: Some("/run/libvirt/qemu/swtpm/preflight.sock".to_string()),
            ..Default::default()
        },
        "/bin/sh",
    );

    assert!(result.is_ok());
}

#[test]
fn preflight_keeps_shared_memory_warning_when_looking_glass_auto_mode_skips_client() {
    let config = VmConfig::from_str(
        r#"
name: preflight-lg
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
    ivshmem:
      enabled: true
      mem_path: /definitely/missing/kvmfr0
  cpu:
    model: host
    vcpus: 2
devices: {}
host:
  pci:
    - id: hostpci0
      device: "0000:03:00.0"
"#,
    )
    .expect("vm config should parse");

    let result = run_runtime_preflight(
        &config,
        &CentralConfig::default(),
        &RuntimeCliOverrides::default(),
        "/bin/sh",
    )
    .expect("preflight should succeed with optional warnings");

    assert!(
        result
            .optional_warnings()
            .iter()
            .any(|warning| warning.contains("shared memory path"))
    );
}

#[test]
fn preflight_fails_for_invalid_network_backend_policy() {
    let config = VmConfig::from_str(
        r#"
name: preflight-network-policy
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
    )
    .expect("vm config should parse");

    let central: CentralConfig = serde_yaml::from_str(
        r#"
host_capabilities:
  network:
    preferred_backend: invalid-backend
"#,
    )
    .expect("central config should parse");

    let err = run_runtime_preflight(
        &config,
        &central,
        &RuntimeCliOverrides::default(),
        "/bin/sh",
    )
    .expect_err("preflight should fail");

    assert!(
        err.to_string()
            .contains("host_capabilities.network.preferred_backend")
    );
}

#[test]
fn preflight_fails_for_invalid_tpm_placement_mode() {
    let config = VmConfig::from_str(
        r#"
name: preflight-tpm-policy
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
    )
    .expect("vm config should parse");

    let central: CentralConfig = serde_yaml::from_str(
        r#"
host_capabilities:
  tpm:
    placement_mode: invalid
"#,
    )
    .expect("central config should parse");

    let err = run_runtime_preflight(
        &config,
        &central,
        &RuntimeCliOverrides::default(),
        "/bin/sh",
    )
    .expect_err("preflight should fail");

    assert!(
        err.to_string()
            .contains("host_capabilities.tpm.placement_mode")
    );
}

#[test]
fn bridge_acl_helper_accepts_existing_file() {
    let existing = Path::new("/etc/hosts");
    let result = ensure_bridge_helper_acl_exists(existing);
    assert!(result.is_ok());
}

#[test]
fn bridge_acl_helper_rejects_missing_file() {
    let missing = Path::new("/definitely/missing/bridge.conf");
    let err = ensure_bridge_helper_acl_exists(missing).expect_err("missing ACL should fail");
    assert!(err.to_string().contains("/definitely/missing/bridge.conf"));
    assert!(
        err.to_string()
            .contains("qemu-bridge-helper requires this ACL file")
    );
}

#[test]
fn bridge_acl_parser_accepts_allow_rule_for_bridge() {
    assert!(bridge_acl_allows_bridge("allow vmbr0\n", "vmbr0"));
    assert!(bridge_acl_allows_bridge("allow all\n", "vmbr0"));
}

#[test]
fn bridge_acl_parser_rejects_missing_or_denied_bridge() {
    assert!(!bridge_acl_allows_bridge("allow br0\n", "vmbr0"));
    assert!(!bridge_acl_allows_bridge(
        "deny vmbr0\nallow vmbr0\n",
        "vmbr0"
    ));
}

#[test]
fn bridge_acl_requirement_rejects_unlisted_bridge_helper_bridge() {
    let acl_path = unique_temp_path("bridge-acl");
    std::fs::write(&acl_path, "allow br0\n").expect("acl file should be created");

    let err = ensure_bridge_helper_acl_allows_bridge(&acl_path, "vmbr0", "net0")
        .expect_err("unlisted bridge should fail");

    assert!(err.to_string().contains("network 'net0'"));
    assert!(err.to_string().contains("allow vmbr0"));

    let _ = std::fs::remove_file(acl_path);
}

#[test]
fn bridge_acl_requirement_skips_non_bridge_helper_networks() {
    let config = VmConfig::from_str(
        r#"
name: preflight-bridge-acl-skip
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices:
  networks:
    - id: net0
      model: virtio-net-pci
      backend:
        type: user
"#,
    )
    .expect("vm config should parse");

    let result = ensure_bridge_helper_acl_requirements(&config, &CentralConfig::default());
    assert!(result.is_ok());
}

#[test]
fn swtpm_apparmor_path_allows_libvirt_sockets() {
    assert!(swtpm_apparmor_socket_path_is_allowed(
        "/run/libvirt/qemu/swtpm/wakiza.sock",
        None,
    ));
    assert!(swtpm_apparmor_socket_path_is_allowed(
        "/var/run/libvirt/qemu/swtpm/wakiza.sock",
        None,
    ));
}

#[test]
fn swtpm_apparmor_path_allows_single_run_socket() {
    assert!(swtpm_apparmor_socket_path_is_allowed(
        "/run/swtpm/sock",
        None
    ));
    assert!(swtpm_apparmor_socket_path_is_allowed(
        "/var/run/swtpm/sock",
        None,
    ));
}

#[test]
fn swtpm_apparmor_path_rejects_unlisted_locations() {
    assert!(!swtpm_apparmor_socket_path_is_allowed(
        "/var/run/ezkvm/tpmstate0-tpm.socket",
        None,
    ));
    assert!(!swtpm_apparmor_socket_path_is_allowed(
        "/tmp/ezkvm/wakiza.swtpm",
        None,
    ));
}

#[test]
fn swtpm_apparmor_path_accepts_local_override_glob() {
    let local = "/var/run/ezkvm/*.socket rwk,\n/var/run/ezkvm/*.pid rwk,";
    assert!(swtpm_apparmor_socket_path_is_allowed(
        "/var/run/ezkvm/tpmstate0-tpm.socket",
        Some(local),
    ));
}

#[test]
fn swtpm_apparmor_path_accepts_local_override_for_tpm_backend_device() {
    let local = "/dev/vm1/vm-*-tpmstate rwk,\n/dev/dm-* rwk,";
    assert!(swtpm_apparmor_path_is_allowed(
        "/dev/vm1/vm-108-tpmstate",
        Some(local),
        false,
    ));
    assert!(swtpm_apparmor_path_is_allowed(
        "/dev/dm-28",
        Some(local),
        false,
    ));
}

#[test]
fn apparmor_glob_matches_simple_single_star() {
    assert!(apparmor_glob_matches(
        "/var/run/ezkvm/*.socket",
        "/var/run/ezkvm/tpmstate0-tpm.socket",
    ));
    assert!(!apparmor_glob_matches(
        "/var/run/ezkvm/*.sock",
        "/var/run/ezkvm/tpmstate0-tpm.socket",
    ));
}

#[test]
fn apparmor_rules_allow_path_ignores_comments_and_permissions() {
    let rules = r#"
# comment
/var/run/ezkvm/*.socket rwk,
/var/log/ezkvm/*.log rwk,
"#;
    assert!(apparmor_rules_allow_path(
        rules,
        "/var/run/ezkvm/tpmstate0-tpm.socket",
    ));
}
