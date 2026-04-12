//! Integration tests for ezkvm functionality

use std::fs;
use ezkvm::config::{CentralConfig, VmConfig};
use ezkvm::qemu::QemuManager;

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
    - type: "qxl"
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

      #[test]
      fn test_wakiza_matches_key_proxmox_fragments() {
        let config = VmConfig::from_file("examples/wakiza.yaml").unwrap();
        let has_cdrom = config.devices.drives.iter().any(|drive| drive.r#type == "cdrom");
        let has_passthrough = config.hostpci.iter().any(|d| d.id.starts_with("hostpci0"));
        let has_usb = !config.usb_devices.is_empty();
        let manager = QemuManager::new(config, CentralConfig::default());
        let args = manager.build_command().unwrap();
        let generated = format!(
          "{} {}",
          manager.binary_name(),
          args.iter().map(|arg| arg.as_str()).collect::<Vec<_>>().join(" ")
        );

        let proxmox_cmd = include_str!("../input/wakiza/108.cmd");

        // Always-present fragments (independent of passthrough / TPM / USB)
        let always_fragments: &[&[&str]] = &[
          &["if=pflash,unit=0", "readonly=on", "OVMF_CODE_4M.secboot.fd"],
          &["if=pflash,unit=1", "id=drive-efidisk0", "format=raw"],
          &["qemu-xhci", "id=xhci", "p2=15", "p3=15", "bus=pci.1", "addr=0x1b"],
          &["ich9-intel-hda", "id=audiodev0", "bus=pci.2", "addr=0xc"],
          &["hda-micro", "id=audiodev0-codec0", "bus=audiodev0.0", "cad=0", "audiodev=spice-backend0"],
          &["hda-duplex", "id=audiodev0-codec1", "bus=audiodev0.0", "cad=1", "audiodev=spice-backend0"],
          &["spice,id=spice-backend0"],
          &["virtio-balloon-pci", "id=balloon0", "bus=pci.0", "addr=0x3", "free-page-reporting=on"],
          &["pvscsi", "id=scsihw0", "bus=pci.0", "addr=0x5"],
          &["scsi-hd", "drive=drive-scsi0", "id=scsi0", "bus=scsihw0.0", "scsi-id=0", "bootindex=100"],
          &["scsi-hd", "drive=drive-scsi1", "id=scsi1", "bus=scsihw0.0", "scsi-id=1"],
          &["virtio-net-pci", "netdev=net0", "mac=BC:24:11:3A:21:B7", "bus=pci.0", "addr=0x12", "rx_queue_size=1024", "tx_queue_size=256", "bootindex=102"],
          &["port=5903", "addr=0.0.0.0", "disable-ticketing=on"],
          &["virtio-mouse"],
          &["virtio-keyboard"],
          &["ivshmem-plain", "memdev=ivshmem0", "bus=pcie.0"],
          &["memory-backend-file", "id=ivshmem0", "share=on", "mem-path=/dev/kvmfr0", "size=128M"],
        ];

        for group in always_fragments {
          for fragment in *group {
            assert!(proxmox_cmd.contains(fragment), "Fixture is missing fragment: {fragment}");
            assert!(generated.contains(fragment), "Generated command is missing fragment: {fragment}\n{generated}");
          }
        }

        // Passthrough-dependent fragments: only checked when hostpci is configured
        if has_passthrough {
          let passthrough_fragments: &[&[&str]] = &[
            &["-vga", "none", "-nographic"],
            &["vfio-pci", "host=0000:03:00.0", "id=hostpci0.0", "bus=ich9-pcie-port-1", "addr=0x0.0", "multifunction=on"],
            &["vfio-pci", "host=0000:03:00.1", "id=hostpci0.1", "bus=ich9-pcie-port-1", "addr=0x0.1"],
          ];
          for group in passthrough_fragments {
            for fragment in *group {
              assert!(proxmox_cmd.contains(fragment), "Fixture is missing fragment: {fragment}");
              assert!(generated.contains(fragment), "Generated command is missing fragment: {fragment}\n{generated}");
            }
          }
        }

        // USB fragments: only checked when usb_devices are configured
        if has_usb {
          let usb_fragments = ["usb-host", "hostbus=1", "hostport=2.2", "id=usb0", "bus=xhci.0", "port=1"];
          for fragment in usb_fragments {
            assert!(proxmox_cmd.contains(fragment), "Fixture is missing fragment: {fragment}");
            assert!(generated.contains(fragment), "Generated command is missing fragment: {fragment}\n{generated}");
          }
        }

        if has_cdrom {
          let cdrom_fragments = ["ide-cd", "drive=drive-ide2", "id=ide2", "bus=ide.1", "unit=0", "bootindex=101"];

          for fragment in cdrom_fragments {
            assert!(proxmox_cmd.contains(fragment), "Fixture is missing fragment: {fragment}");
            assert!(generated.contains(fragment), "Generated command is missing fragment: {fragment}\n{generated}");
          }
        }

        let virtio_serial_controller_count = generated.matches("virtio-serial-pci").count();
        assert_eq!(virtio_serial_controller_count, 1, "Expected a single virtio-serial-pci controller in generated command\n{generated}");
      }
}