//! Device configuration module
//!
//! Handles configuration of virtual devices like drives, networks, displays.

use super::DeviceConfig;
use crate::qemu::types::QemuArgs;

impl From<DeviceConfig> for QemuArgs {
    fn from(config: DeviceConfig) -> Self {
        let mut args = QemuArgs::new();

        // Add drive arguments
        for drive in config.drives {
            args.extend(QemuArgs::from(drive));
        }

        // Add network arguments
        for network in config.networks {
            args.extend(QemuArgs::from(network));
        }

        // Add display arguments
        for display in config.displays {
            args.extend(QemuArgs::from(display));
        }

        // Add serial arguments
        for serial in config.serials {
            args.extend(QemuArgs::from(serial));
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{DeviceConfig, DisplayConfig, DriveConfig, NetworkConfig, SerialConfig};
    use crate::qemu::types::QemuArgs;

    #[test]
    fn test_drive_config_with_advanced_options() {
        let drive = DriveConfig {
            id: "root".to_string(),
            path: "/path/to/disk.qcow2".to_string(),
            interface: "virtio".to_string(),
            r#type: "disk".to_string(),
            format: "qcow2".to_string(),
            readonly: false,
            discard: true,
            ssd: false,
            cache: Some("none".to_string()),
            aio: Some("io_uring".to_string()),
            detect_zeroes: Some("unmap".to_string()),
            controller: None,
            boot_index: None,
            scsi_id: None,
            rotation_rate: None,
            bus: None,
            unit: None,
        };

        let args = QemuArgs::from(drive).into_inner();
        assert_eq!(args[0], "-drive");
        assert!(args[1].contains("file=/path/to/disk.qcow2,if=virtio,format=qcow2"));
        assert!(args[1].contains(",discard=unmap"));
        assert!(args[1].contains(",cache=none"));
        assert!(args[1].contains(",aio=io_uring"));
        assert!(args[1].contains(",detect-zeroes=unmap"));
    }

    #[test]
    fn test_scsi_drive_with_device_attachment_options() {
        let drive = DriveConfig {
            id: "scsi0".to_string(),
            path: "/path/to/disk.raw".to_string(),
            interface: "scsi".to_string(),
            r#type: "disk".to_string(),
            format: "raw".to_string(),
            readonly: false,
            discard: true,
            ssd: false,
            cache: Some("none".to_string()),
            aio: Some("io_uring".to_string()),
            detect_zeroes: Some("unmap".to_string()),
            controller: Some("scsihw0".to_string()),
            boot_index: Some(100),
            scsi_id: Some(0),
            rotation_rate: None,
            bus: None,
            unit: None,
        };

        let args = QemuArgs::from(drive).into_inner();
        assert_eq!(args[0], "-drive");
        assert!(args[1].contains("if=none"));
        assert!(args[1].contains("id=drive-scsi0"));
        assert_eq!(args[2], "-device");
        assert!(args[3].contains("scsi-hd,drive=drive-scsi0,id=scsi0"));
        assert!(args[3].contains(",bus=scsihw0.0"));
        assert!(args[3].contains(",scsi-id=0"));
        assert!(args[3].contains(",bootindex=100"));
    }

    #[test]
    fn test_ide_cdrom_with_bus_and_unit() {
        let drive = DriveConfig {
            id: "ide2".to_string(),
            path: "".to_string(),
            interface: "ide".to_string(),
            r#type: "cdrom".to_string(),
            format: "raw".to_string(),
            readonly: true,
            discard: false,
            ssd: false,
            cache: None,
            aio: Some("io_uring".to_string()),
            detect_zeroes: None,
            controller: None,
            boot_index: Some(101),
            scsi_id: None,
            rotation_rate: None,
            bus: Some("ide.1".to_string()),
            unit: Some(0),
        };

        let args = QemuArgs::from(drive).into_inner();
        assert_eq!(args[0], "-drive");
        assert!(args[1].contains("if=none"));
        assert!(args[1].contains("media=cdrom"));
        assert!(!args[1].contains("format="));
        assert!(!args[1].contains("aio="));
        assert_eq!(args[2], "-device");
        assert!(args[3].contains("ide-cd,drive=drive-ide2,id=ide2"));
        assert!(args[3].contains(",bus=ide.1"));
        assert!(args[3].contains(",unit=0"));
        assert!(args[3].contains(",bootindex=101"));
    }

    #[test]
    fn test_network_config_with_advanced_options() {
        let network = NetworkConfig {
            id: "net0".to_string(),
            model: "virtio-net-pci".to_string(),
            backend: Some(crate::config::NetworkBackendConfig {
                backend_type: "tap".to_string(),
                ifname: Some("tap0".to_string()),
                script: Some("no".to_string()),
                downscript: Some("no".to_string()),
                vhost: Some(true),
                ..Default::default()
            }),
            mac: Some("52:54:00:12:34:56".to_string()),
            rx_queue_size: Some(1024),
            tx_queue_size: Some(256),
            boot_index: Some(102),
            bus: Some("pci.0".to_string()),
            addr: Some("0x12".to_string()),
        };

        let args = QemuArgs::from(network).into_inner();
        assert_eq!(args[0], "-netdev");
        assert!(args[1].contains("id=net0"));
        assert!(args[1].contains("type=tap"));
        assert!(args[1].contains("ifname=tap0"));
        assert!(args[1].contains("script=no"));
        assert!(args[1].contains("downscript=no"));
        assert!(args[1].contains("vhost=on"));
        assert_eq!(args[2], "-device");
        assert!(args[3].contains("virtio-net-pci,netdev=net0"));
        assert!(args[3].contains(",mac=52:54:00:12:34:56"));
        assert!(args[3].contains(",bus=pci.0"));
        assert!(args[3].contains(",addr=0x12"));
        assert!(args[3].contains(",rx_queue_size=1024"));
        assert!(args[3].contains(",tx_queue_size=256"));
        assert!(args[3].contains(",bootindex=102"));
    }

    #[test]
    fn test_display_config_with_vram() {
        let display = DisplayConfig {
            r#type: "qxl".to_string(),
            vram: Some(256),
        };

        let args = QemuArgs::from(display).into_inner();
        assert_eq!(args[0], "-device");
        assert_eq!(args[1], "qxl,vram_size_mb=256");
    }

    #[test]
    fn test_serial_file_backend() {
        let serial = SerialConfig {
            r#type: "file".to_string(),
            port: Some(0),
            path: Some("/tmp/serial.log".to_string()),
            host: None,
            socket_port: None,
            server: true,
            wait: false,
        };

        let args = QemuArgs::from(serial).into_inner();
        assert_eq!(args, vec!["-serial", "file:/tmp/serial.log"]);
    }

    #[test]
    fn test_serial_socket_backend() {
        let serial = SerialConfig {
            r#type: "socket".to_string(),
            port: Some(0),
            path: None,
            host: Some("127.0.0.1".to_string()),
            socket_port: Some(4444),
            server: true,
            wait: false,
        };

        let args = QemuArgs::from(serial).into_inner();
        assert_eq!(args, vec!["-serial", "tcp:127.0.0.1:4444,server,nowait"]);
    }

    #[test]
    fn test_device_config_conversion_preserves_section_order() {
        let config = DeviceConfig {
            drives: vec![DriveConfig {
                id: "root".to_string(),
                path: "/images/root.qcow2".to_string(),
                interface: "virtio".to_string(),
                r#type: "disk".to_string(),
                format: "qcow2".to_string(),
                readonly: false,
                discard: false,
                ssd: false,
                cache: None,
                aio: None,
                detect_zeroes: None,
                controller: None,
                boot_index: None,
                scsi_id: None,
                rotation_rate: None,
                bus: None,
                unit: None,
            }],
            networks: vec![NetworkConfig {
                id: "net0".to_string(),
                model: "virtio-net-pci".to_string(),
                backend: Some(crate::config::NetworkBackendConfig {
                    backend_type: "user".to_string(),
                    ..Default::default()
                }),
                mac: Some("52:54:00:12:34:56".to_string()),
                rx_queue_size: None,
                tx_queue_size: None,
                boot_index: None,
                bus: None,
                addr: None,
            }],
            displays: vec![DisplayConfig {
                r#type: "qxl".to_string(),
                vram: Some(64),
            }],
            serials: vec![SerialConfig {
                r#type: "pty".to_string(),
                port: None,
                path: None,
                host: None,
                socket_port: None,
                server: true,
                wait: false,
            }],
            input: vec![],
            audio: vec![],
        };

        let args = QemuArgs::from(config).into_inner();
        assert_eq!(args[0], "-drive");
        assert_eq!(args[2], "-netdev");
        assert_eq!(args[4], "-device");
        assert_eq!(args[6], "-device");
        assert_eq!(args[8], "-serial");
    }
}
