use crate::config::VmConfig;
use std::collections::HashSet;

/// Parse a QEMU `-readconfig` INI file and return the names of all
/// `[device "name"]` sections defined in it.
///
/// Returns `None` if the file cannot be read (e.g. not present on this host).
fn device_names_from_readconfig(path: &str) -> Option<HashSet<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let names = content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("[device \"") && line.ends_with("\"]") {
                Some(line[9..line.len() - 2].to_string())
            } else {
                None
            }
        })
        .collect();
    Some(names)
}

/// Collect every `ich9-pcie-port-*` bus name referenced by hostpci entries in
/// the given config.
fn referenced_root_port_buses(config: &VmConfig) -> HashSet<String> {
    config
        .host_pci()
        .iter()
        .filter_map(|h| h.bus.as_deref())
        .filter(|bus| bus.starts_with("ich9-pcie-port"))
        .map(str::to_string)
        .collect()
}

fn uses_synthesized_portable_q35_topology(config: &VmConfig) -> bool {
    let mode = crate::state::detect_runtime_capability_mode(config);
    if mode != crate::state::RuntimeCapabilityMode::PortableLinux {
        return false;
    }

    if !config.system.machine.to_lowercase().contains("q35") {
        return false;
    }

    config
        .system
        .readconfig
        .iter()
        .any(|path| path.contains("ezkvm-q35.cfg"))
}

/// Check that every `ich9-pcie-port-*` bus name referenced by hostpci entries
/// is actually defined in one of the loaded readconfig files.
///
/// This catches mismatches between the planner's root-port allocation and the
/// readconfig template on disk — for example if the template defines 4 ports but
/// a config was somehow created with a reference to port-5.
///
/// Returns a sorted list of human-readable warning strings. An empty vec means
/// all references are satisfied (or no check was possible).
pub(super) fn check_hostpci_bus_references(config: &VmConfig) -> Vec<String> {
    if uses_synthesized_portable_q35_topology(config) {
        return vec![];
    }

    let referenced = referenced_root_port_buses(config);
    if referenced.is_empty() {
        return vec![];
    }

    // Build the set of device names defined across all readable readconfig files.
    let mut defined: HashSet<String> = HashSet::new();
    let mut any_read = false;
    for path in &config.system.readconfig {
        if let Some(names) = device_names_from_readconfig(path) {
            defined.extend(names);
            any_read = true;
        }
    }

    if !any_read {
        // No readconfig file could be read on this host; skip the check rather
        // than emit false positives.
        return vec![];
    }

    let mut missing: Vec<&str> = referenced
        .iter()
        .filter(|bus| !defined.contains(*bus))
        .map(String::as_str)
        .collect();
    missing.sort_unstable();

    missing
        .into_iter()
        .map(|bus| {
            format!(
                "hostpci device references bus '{}' which is not defined in any loaded \
                 readconfig file; QEMU may fail to start",
                bus
            )
        })
        .collect()
}

/// Check that no hostpci entry references a legacy `pci.N` bus name on a portable
/// Q35 machine when no loaded readconfig defines legacy PCI bridge buses.
///
/// `pci.N` buses are valid when a Q35 topology template defines those bridge names
/// (for example Proxmox `pve-q35-4.0.cfg` or ezkvm `ezkvm-q35.cfg`).
///
/// Returns a sorted list of human-readable warning strings. An empty vec means
/// all references are valid (or the check is not applicable).
pub(super) fn check_legacy_pci_bus_references(config: &VmConfig) -> Vec<String> {
    let machine_lower = config.system.machine.to_lowercase();
    if !machine_lower.contains("q35") {
        return vec![];
    }

    if uses_synthesized_portable_q35_topology(config) {
        return vec![];
    }

    // Only applies when no known Q35 bridge template is loaded.
    let has_legacy_pci_bridge_readconfig = config
        .system
        .readconfig
        .iter()
        .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
    if has_legacy_pci_bridge_readconfig {
        return vec![];
    }

    let mut warnings: Vec<String> = config
        .host_pci()
        .iter()
        .filter_map(|h| h.bus.as_deref())
        .filter(|bus| bus.starts_with("pci."))
        .map(|bus| {
            format!(
                "hostpci device references legacy PCI bus '{}' on a portable Q35 machine; \
                 this bus is only defined when a Q35 bridge readconfig is loaded \
                 (for example pve-q35-4.0.cfg or ezkvm-q35.cfg) and \
                 QEMU may fail to start",
                bus
            )
        })
        .collect();
    warnings.sort_unstable();
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_readconfig(content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ezkvm-preflight-test-{}.cfg",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut f = std::fs::File::create(&path).expect("create tempfile");
        write!(f, "{}", content).expect("write tempfile");
        path
    }

    fn config_with_hostpci_bus(bus: &str, readconfig_path: &str) -> VmConfig {
        let yaml = format!(
            r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
  readconfig:
    - "{readconfig_path}"
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      pcie: true
      bus: "{bus}"
"#
        );
        VmConfig::from_str(&yaml).expect("parse config")
    }

    #[test]
    fn no_warnings_when_bus_is_defined_in_readconfig() {
        let path = write_readconfig(
            r#"
[device "ich9-pcie-port-1"]
driver = "pcie-root-port"
bus = "pcie.0"
"#,
        );
        let config = config_with_hostpci_bus("ich9-pcie-port-1", &path.to_string_lossy());
        assert!(check_hostpci_bus_references(&config).is_empty());
    }

    #[test]
    fn warning_when_bus_is_not_in_readconfig() {
        let path = write_readconfig(
            r#"
[device "ich9-pcie-port-1"]
driver = "pcie-root-port"
bus = "pcie.0"
"#,
        );
        let config = config_with_hostpci_bus("ich9-pcie-port-5", &path.to_string_lossy());
        let warnings = check_hostpci_bus_references(&config);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("ich9-pcie-port-5"),
            "warning should name the missing bus: {:?}",
            warnings
        );
    }

    #[test]
    fn no_warnings_when_bus_is_not_an_ich9_port() {
        let path = write_readconfig("[device \"some-device\"]\n");
        let config = config_with_hostpci_bus("pcie.0", &path.to_string_lossy());
        assert!(check_hostpci_bus_references(&config).is_empty());
    }

    #[test]
    fn no_warnings_when_readconfig_file_is_missing() {
        let config = config_with_hostpci_bus("ich9-pcie-port-5", "/nonexistent/path.cfg");
        // File cannot be read → check is skipped, no false positive.
        assert!(check_hostpci_bus_references(&config).is_empty());
    }

    #[test]
    fn no_warnings_when_no_hostpci_bus_is_set() {
        let yaml = r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      pcie: true
"#;
        let config = VmConfig::from_str(yaml).expect("parse config");
        assert!(check_hostpci_bus_references(&config).is_empty());
    }

    #[test]
    fn no_readconfig_bus_warnings_when_portable_q35_synthesis_is_active() {
        let yaml = r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  readconfig:
    - /usr/share/ezkvm/ezkvm-q35.cfg
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      pcie: true
      bus: ich9-pcie-port-5
"#;
        let config = VmConfig::from_str(yaml).expect("parse config");
        assert!(check_hostpci_bus_references(&config).is_empty());
    }

    fn portable_q35_config_with_hostpci_bus(bus: &str) -> VmConfig {
        let yaml = format!(
            r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      pcie: true
      bus: "{bus}"
"#
        );
        VmConfig::from_str(&yaml).expect("parse config")
    }

    #[test]
    fn legacy_pci_bus_warns_on_portable_q35() {
        let config = portable_q35_config_with_hostpci_bus("pci.0");
        let warnings = check_legacy_pci_bus_references(&config);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("pci.0"),
            "warning should name the legacy bus: {:?}",
            warnings
        );
    }

    #[test]
    fn legacy_pci_bus_no_warning_when_proxmox_readconfig_loaded() {
        let yaml = r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
  readconfig:
    - /usr/share/qemu-server/pve-q35-4.0.cfg
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      pcie: true
      bus: pci.0
"#;
        let config = VmConfig::from_str(yaml).expect("parse config");
        // pci.0 is valid when Proxmox bridges are loaded
        assert!(check_legacy_pci_bus_references(&config).is_empty());
    }

    #[test]
    fn legacy_pci_bus_no_warning_when_ezkvm_readconfig_loaded() {
        let yaml = r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
  readconfig:
    - /usr/share/ezkvm/ezkvm-q35.cfg
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      pcie: true
      bus: pci.1
"#;
        let config = VmConfig::from_str(yaml).expect("parse config");
        // pci.1 is valid when ezkvm q35 legacy bridges are loaded
        assert!(check_legacy_pci_bus_references(&config).is_empty());
    }

    #[test]
    fn legacy_pci_bus_no_warning_for_pcie_bus() {
        let config = portable_q35_config_with_hostpci_bus("pcie.0");
        assert!(check_legacy_pci_bus_references(&config).is_empty());
    }

    #[test]
    fn legacy_pci_bus_no_warning_for_ich9_port() {
        let config = portable_q35_config_with_hostpci_bus("ich9-pcie-port-1");
        assert!(check_legacy_pci_bus_references(&config).is_empty());
    }

    #[test]
    fn legacy_pci_bus_no_warning_for_non_q35_machine() {
        let yaml = r#"
name: test-vm
backend: qemu
system:
  architecture: x86_64
  machine: pc
  memory:
    size: 4096
  cpu:
    vcpus: 2
    model: host
host:
  pci:
    - device: "0000:03:00.0"
      id: hostpci0
      bus: pci.0
"#;
        let config = VmConfig::from_str(yaml).expect("parse config");
        // pci.0 is the correct bus for non-Q35 machines
        assert!(check_legacy_pci_bus_references(&config).is_empty());
    }
}
