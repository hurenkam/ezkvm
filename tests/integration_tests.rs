//! Integration tests for ezkvm functionality

use std::fs;
use ezkvm::config::VmConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_config_file_parsing() {
        // Create a temporary config file
        let config_content = r#"
name: "integration-test-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 1024
  vcpus: 1
  cpu_model: "host"

boot:
  firmware: "bios"
  boot_order: ["disk"]

devices:
  drives:
    - id: "root"
      path: "/tmp/test-disk.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"

  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "user"

  displays:
    - type: "cirrus"
      vram: 64

options:
  enable_kvm: true
  daemonize: false
"#;

        let temp_path = "/tmp/test_config.yaml";
        fs::write(temp_path, config_content).unwrap();

        // Test parsing from file
        let config = VmConfig::from_file(temp_path).unwrap();

        assert_eq!(config.name, "integration-test-vm");
        assert_eq!(config.system.memory, 1024);
        assert_eq!(config.devices.drives.len(), 1);
        assert_eq!(config.devices.networks.len(), 1);
        assert_eq!(config.devices.displays.len(), 1);

        // Clean up
        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_config_validation() {
        // Test that validation catches invalid configurations
        let invalid_yaml = r#"
name: "invalid-vm"
backend: "qemu"

system:
  architecture: "invalid_arch"
  machine: "q35"
  memory: 1024
  vcpus: 2
  cpu_model: "host"
"#;

        let result = VmConfig::from_str(invalid_yaml);
        // This should fail validation for invalid architecture
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_config() {
        let minimal_yaml = r#"
name: "minimal-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "pc"
  memory: 512
  vcpus: 1
  cpu_model: "qemu64"
"#;

        let config = VmConfig::from_str(minimal_yaml).unwrap();
        assert_eq!(config.name, "minimal-vm");
        assert_eq!(config.system.architecture, "x86_64");
        assert_eq!(config.system.memory, 512);
        assert_eq!(config.system.vcpus, 1);
        // Check defaults
        assert_eq!(config.options.enable_kvm, true); // default_true
        assert_eq!(config.options.daemonize, false);
    }
}