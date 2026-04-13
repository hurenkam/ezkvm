use anyhow::Result;

use super::{DeviceCommands, NetworkCommands, PciCommands, StorageCommands, UsbCommands};

pub(crate) async fn handle_create(config_path: &str, validate_only: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path);

    let config = crate::config::VmConfig::from_file(config_path)?;
    println!("✓ Configuration loaded and validated");
    println!("\nVM Details:");
    println!("  Name: {}", config.name);
    println!("  Architecture: {}", config.system.architecture);
    println!("  Machine: {}", config.system.machine);
    println!("  Memory: {} MiB", config.system.memory.size);
    println!("  vCPUs: {}", config.system.cpu.vcpus);
    println!("  Devices:");
    println!("    Drives: {}", config.devices.drives.len());
    println!("    Networks: {}", config.devices.networks.len());
    println!("    Displays: {}", config.devices.displays.len());

    if validate_only {
        println!("\n✓ Validation successful");
        return Ok(());
    }

    crate::state::cache_config(&config.name, &config)?;
    println!("\n✓ VM '{}' configuration cached", config.name);
    let state_dir = crate::state::get_state_dir()?;
    println!("Configuration saved to: {}", state_dir.display());

    Ok(())
}

pub(crate) async fn handle_storage(cmd: StorageCommands) -> Result<()> {
    match cmd {
        StorageCommands::Create { name, size } => handle_storage_create(&name, size),
        StorageCommands::List => handle_storage_list(),
        StorageCommands::Info { disk } => handle_storage_info(&disk),
        StorageCommands::Resize { disk, size } => handle_storage_resize(&disk, size),
        StorageCommands::Snapshot { disk, name } => handle_storage_snapshot(&disk, &name),
    }
}

fn handle_storage_create(name: &str, size: u32) -> Result<()> {
    println!("Creating storage image: {}", name);
    println!("Size: {} GB", size);

    let output = std::process::Command::new("qemu-img")
        .args(["create", "-f", "qcow2", name])
        .arg(format!("{}G", size))
        .output()?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to create storage image: {}",
            err_msg
        ));
    }

    println!("✓ Storage image created at: {}", name);
    Ok(())
}

fn handle_storage_list() -> Result<()> {
    println!("Available storage images:");

    let home = std::env::var("HOME").unwrap_or_default();
    let home_storage = format!("{}/.ezkvm/storage", home);
    let common_paths = [".", "./storage", "./images", &home_storage];

    for path in common_paths {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str()
                    && (name.ends_with(".qcow2") || name.ends_with(".img"))
                {
                    println!("  {}", entry.path().display());
                }
            }
        }
    }

    Ok(())
}

fn handle_storage_info(disk: &str) -> Result<()> {
    println!("Getting information about disk: {}", disk);

    let output = std::process::Command::new("qemu-img")
        .args(["info", disk])
        .output()?;

    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout);
        println!("{}", info);
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        println!("Error getting disk info: {}", err_msg);
    }

    Ok(())
}

fn handle_storage_resize(disk: &str, size: u32) -> Result<()> {
    println!("Resizing disk '{}' to {} GB", disk, size);

    let output = std::process::Command::new("qemu-img")
        .args(["resize", disk])
        .arg(format!("{}G", size))
        .output()?;

    if output.status.success() {
        println!("✓ Disk resized successfully");
        return Ok(());
    }

    let err_msg = String::from_utf8_lossy(&output.stderr);
    Err(anyhow::anyhow!("Failed to resize disk: {}", err_msg))
}

fn handle_storage_snapshot(disk: &str, name: &str) -> Result<()> {
    println!("Creating snapshot '{}' of disk '{}'", name, disk);

    let output = std::process::Command::new("qemu-img")
        .args(["snapshot", "-c", name, disk])
        .output()?;

    if output.status.success() {
        println!("✓ Snapshot created successfully");
        return Ok(());
    }

    let err_msg = String::from_utf8_lossy(&output.stderr);
    Err(anyhow::anyhow!("Failed to create snapshot: {}", err_msg))
}

pub(crate) async fn handle_device(cmd: DeviceCommands) -> Result<()> {
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

pub(crate) async fn handle_network(cmd: NetworkCommands) -> Result<()> {
    match cmd {
        NetworkCommands::Bridge { name } => {
            println!("Creating bridge: {}", name);
            println!("Note: This requires root/sudo privileges");

            println!("\nYou can create a bridge manually with:");
            println!("  sudo brctl addbr {}", name);
            println!("  sudo brctl addif {} <interface>", name);
            println!("  sudo ip addr add <ip>/<mask> dev {}", name);
            println!("  sudo ip link set {} up", name);

            Ok(())
        }
    }
}
