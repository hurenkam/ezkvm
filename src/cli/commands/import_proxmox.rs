use crate::cli::ImportOutputModeArg;
use crate::cli::commands::CliResult;
use crate::cli::types::RuntimeTargetArg;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_import_proxmox(
    input: &str,
    proxmox_storage: Option<&str>,
    output: Option<&str>,
    dry_run: bool,
    strict: bool,
    no_compact: bool,
    output_mode: ImportOutputModeArg,
    runtime_target: RuntimeTargetArg,
) -> CliResult {
    let output_mode = match output_mode {
        ImportOutputModeArg::Canonical => crate::import::proxmox::ImportOutputMode::Canonical,
        ImportOutputModeArg::Compact => crate::import::proxmox::ImportOutputMode::Compact,
        ImportOutputModeArg::Debug => crate::import::proxmox::ImportOutputMode::DebugCanonical,
    };

    let runtime_target = match runtime_target {
        RuntimeTargetArg::PortableLinux => crate::import::proxmox::RuntimeTarget::PortableLinux,
        RuntimeTargetArg::ProxmoxParity => crate::import::proxmox::RuntimeTarget::ProxmoxParity,
    };

    let options = crate::import::proxmox::ImportRunOptions {
        output_path: output.map(ToString::to_string),
        storage_path: proxmox_storage.map(ToString::to_string),
        strict,
        dry_run,
        compact_lists: !no_compact,
        output_mode,
        runtime_target,
    };

    let result = crate::import::proxmox::run_import_from_files(input, &options)
        .map_err(|e| anyhow::anyhow!("proxmox import failed: {}", e))?;

    if dry_run {
        println!("# import-proxmox dry run");
        println!("# input: {}", input);
        println!("# output (not written): {}", result.output_path);
        println!("# output mode: {:?}", output_mode);
        println!("# runtime target: {:?}", runtime_target);
        match runtime_target {
            crate::import::proxmox::RuntimeTarget::PortableLinux => {
                println!(
                    "# capability precedence: cli > vm-override > profile-default > central-config > platform-default"
                );
            }
            crate::import::proxmox::RuntimeTarget::ProxmoxParity => {
                println!(
                    "# capability precedence: bypassed (proxmox-parity target preserves parity defaults)"
                );
            }
        }
        if !result.warnings.is_empty() {
            println!("# warnings ({}):", result.warnings.len());
            for warning in &result.warnings {
                println!("# - {}: {}", warning.source_field, warning.message);
            }
        }
        println!("{}", result.yaml);
    } else {
        println!("Import complete");
        println!("Input: {}", input);
        println!("Output: {}", result.output_path);
        println!("Output mode: {:?}", output_mode);
        if !result.warnings.is_empty() {
            println!("Warnings ({}):", result.warnings.len());
            for warning in &result.warnings {
                println!("- {}: {}", warning.source_field, warning.message);
            }
        }
    }

    Ok(())
}
