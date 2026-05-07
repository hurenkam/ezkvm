use crate::cli::NetworkCommands;
use crate::cli::commands::CliResult;

pub(crate) async fn handle_network(cmd: NetworkCommands) -> CliResult {
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
