use crate::cli::StorageCommands;
use crate::cli::commands::CliResult;

pub(crate) async fn handle_storage(cmd: StorageCommands) -> CliResult {
    match cmd {
        StorageCommands::Create { name, size } => handle_storage_create(&name, size),
        StorageCommands::List => handle_storage_list(),
        StorageCommands::Info { disk } => handle_storage_info(&disk),
        StorageCommands::Resize { disk, size } => handle_storage_resize(&disk, size),
        StorageCommands::Snapshot { disk, name } => handle_storage_snapshot(&disk, &name),
    }
}

fn handle_storage_create(name: &str, size: u32) -> CliResult {
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

fn handle_storage_list() -> CliResult {
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

fn handle_storage_info(disk: &str) -> CliResult {
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

fn handle_storage_resize(disk: &str, size: u32) -> CliResult {
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

fn handle_storage_snapshot(disk: &str, name: &str) -> CliResult {
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
