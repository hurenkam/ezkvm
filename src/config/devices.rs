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