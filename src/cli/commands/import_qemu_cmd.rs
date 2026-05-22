use crate::cli::ImportOutputModeArg;
use crate::cli::commands::CliResult;

pub(crate) async fn handle_import_qemu_cmd(
    input: &str,
    output: Option<&str>,
    dry_run: bool,
    strict: bool,
    output_mode: ImportOutputModeArg,
) -> CliResult {
    let output_mode = match output_mode {
        ImportOutputModeArg::Canonical => crate::import::qemu_cmd::ImportOutputMode::Canonical,
        ImportOutputModeArg::Compact => crate::import::qemu_cmd::ImportOutputMode::Compact,
        ImportOutputModeArg::Debug => crate::import::qemu_cmd::ImportOutputMode::DebugCanonical,
    };

    let options = crate::import::qemu_cmd::ImportRunOptions {
        output_path: output.map(ToString::to_string),
        strict,
        dry_run,
        output_mode,
    };

    let result = crate::import::qemu_cmd::run_import_from_files(input, &options)
        .map_err(|e| anyhow::anyhow!("qemu-cmd import failed: {}", e))?;

    if dry_run {
        println!("# import-qemu-cmd dry run");
        println!("# input: {}", input);
        println!("# output (not written): {}", result.output_path);
        println!("# output mode: {:?}", output_mode);
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
