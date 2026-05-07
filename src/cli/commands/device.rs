use crate::cli::commands::CliResult;
use crate::cli::{DeviceCommands, PciCommands, UsbCommands};

pub(crate) async fn handle_device(cmd: DeviceCommands) -> CliResult {
    match cmd {
        DeviceCommands::Usb { cmd } => match cmd {
            Some(UsbCommands::List) | None => {
                println!("Available USB devices:");

                if let Ok(output) = std::process::Command::new("lsusb").output() {
                    if output.status.success() {
                        let devices = String::from_utf8_lossy(&output.stdout);
                        print!("{}", devices);
                    } else {
                        println!("  (lsusb not available)");
                    }
                } else {
                    println!("  (USB listing requires lsusb tool)");
                }
                Ok(())
            }
        },
        DeviceCommands::Pci { cmd } => match cmd {
            Some(PciCommands::List) | None => {
                println!("Available PCI devices:");

                if let Ok(output) = std::process::Command::new("lspci").output() {
                    if output.status.success() {
                        let devices = String::from_utf8_lossy(&output.stdout);
                        print!("{}", devices);
                    } else {
                        println!("  (lspci not available)");
                    }
                } else {
                    println!("  (PCI listing requires lspci tool)");
                }
                Ok(())
            }
        },
    }
}
