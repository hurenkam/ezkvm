//! Device configuration module
//!
//! Handles configuration of virtual devices like drives, networks, displays.

use super::{DeviceConfig, DriveConfig, NetworkConfig, DisplayConfig, SerialConfig};
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

impl From<DriveConfig> for QemuArgs {
    fn from(drive: DriveConfig) -> Self {
        let mut args = QemuArgs::new();
        
        args.push_str("-drive");
        
        let mut drive_spec = format!("file={},if={},format={}",
                                   drive.path, drive.interface, drive.format);
        
        if drive.readonly {
            drive_spec.push_str(",readonly=on");
        }

        if drive.discard {
            drive_spec.push_str(",discard=unmap");
        }

        if drive.ssd {
            drive_spec.push_str(",ssd=on");
        }

        if let Some(cache) = drive.cache {
            drive_spec.push_str(&format!(",cache={}", cache));
        }

        if let Some(aio) = drive.aio {
            drive_spec.push_str(&format!(",aio={}", aio));
        }

        if let Some(detect_zeroes) = drive.detect_zeroes {
            drive_spec.push_str(&format!(",detect-zeroes={}", detect_zeroes));
        }
        
        args.push(drive_spec);
        args
    }
}

impl From<NetworkConfig> for QemuArgs {
    fn from(network: NetworkConfig) -> Self {
        let mut args = QemuArgs::new();
        
        args.push_str("-netdev");
        let netdev_spec = match network.mode.as_str() {
            "user" => format!("type=user,id={}", network.id),
            _ => format!("type={},id={}", network.mode, network.id),
        };
        args.push(netdev_spec);
        
        args.push_str("-device");
        let mut device_spec = format!("{},netdev={}", network.model, network.id);
        
        if let Some(mac) = network.mac {
            device_spec.push_str(&format!(",mac={}", mac));
        }

        if let Some(bus) = network.bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = network.addr {
            device_spec.push_str(&format!(",addr={}", addr));
        }

        if let Some(rx_queue_size) = network.rx_queue_size {
            device_spec.push_str(&format!(",rx_queue_size={}", rx_queue_size));
        }

        if let Some(tx_queue_size) = network.tx_queue_size {
            device_spec.push_str(&format!(",tx_queue_size={}", tx_queue_size));
        }

        if let Some(boot_index) = network.boot_index {
            device_spec.push_str(&format!(",bootindex={}", boot_index));
        }
        
        args.push(device_spec);
        args
    }
}

impl From<DisplayConfig> for QemuArgs {
    fn from(display: DisplayConfig) -> Self {
        let mut args = QemuArgs::new();
        
        args.push_str("-device");
        let device_spec = display.r#type.clone();
        
        // For virtio-gpu, VRAM is specified differently or not at all
        // Let's skip VRAM for now to get basic functionality working
        args.push(device_spec);
        args
    }
}

impl From<SerialConfig> for QemuArgs {
    fn from(serial: SerialConfig) -> Self {
        let mut args = QemuArgs::new();
        
        match serial.r#type.as_str() {
            "pty" => {
                args.push_str("-serial");
                args.push("pty".to_string());
            }
            "stdio" => {
                args.push_str("-serial");
                args.push("stdio".to_string());
            }
            "file" => {
                // Would need a path parameter, simplified for now
                args.push_str("-serial");
                args.push("file:/dev/null".to_string());
            }
            "socket" => {
                args.push_str("-serial");
                args.push("tcp:127.0.0.1:4444,server,nowait".to_string());
            }
            _ => {
                // Default to pty
                args.push_str("-serial");
                args.push("pty".to_string());
            }
        }
        
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_network_config_with_advanced_options() {
        let network = NetworkConfig {
            id: "net0".to_string(),
            model: "virtio-net-pci".to_string(),
            mode: "tap,ifname=tap0,script=no,downscript=no".to_string(),
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
        assert_eq!(args[2], "-device");
        assert!(args[3].contains("virtio-net-pci,netdev=net0"));
        assert!(args[3].contains(",mac=52:54:00:12:34:56"));
        assert!(args[3].contains(",bus=pci.0"));
        assert!(args[3].contains(",addr=0x12"));
        assert!(args[3].contains(",rx_queue_size=1024"));
        assert!(args[3].contains(",tx_queue_size=256"));
        assert!(args[3].contains(",bootindex=102"));
    }
}