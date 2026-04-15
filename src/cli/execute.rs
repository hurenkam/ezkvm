use anyhow::Result;

use super::commands::{
    handle_create, handle_device, handle_import_proxmox, handle_network, handle_storage,
};
use super::runtime::{
    handle_console, handle_kill, handle_list, handle_start, handle_status, handle_stop,
    handle_validate,
};
use super::{Cli, Commands};

/// Execute the CLI command
pub async fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Create {
            config,
            validate_only,
        } => handle_create(&config, validate_only).await,
        Commands::Start {
            config,
            daemon,
            dry_run,
        } => handle_start(&config, daemon, dry_run).await,
        Commands::Stop { config, force } => handle_stop(&config, force).await,
        Commands::Kill { config } => handle_kill(&config).await,
        Commands::List => handle_list().await,
        Commands::Status { config } => handle_status(&config).await,
        Commands::Console { config } => handle_console(&config).await,
        Commands::Validate {
            config,
            show_resolved_config,
        } => handle_validate(&config, show_resolved_config).await,
        Commands::ImportProxmox {
            input,
            proxmox_storage,
            output,
            dry_run,
            strict,
            no_compact,
        } => {
            handle_import_proxmox(
                &input,
                proxmox_storage.as_deref(),
                output.as_deref(),
                dry_run,
                strict,
                no_compact,
            )
            .await
        }
        Commands::Storage(cmd) => handle_storage(cmd).await,
        Commands::Device(cmd) => handle_device(cmd).await,
        Commands::Network(cmd) => handle_network(cmd).await,
    }
}
