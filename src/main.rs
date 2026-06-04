use std::path::{Path, PathBuf};

mod cli;

use cli::{CliCommand, parse_cli_options, print_help};
use ezkvm::config_format::{
    ExportOptions, Exporter, EzkvmExporter, EzkvmImporter, ImportOptions, Importer,
    LibvirtExporter, LibvirtImporter, ProxmoxExporter, ProxmoxImporter, QemuExporter, QemuImporter,
    RuntimeConfig,
};
use ezkvm::runtime_config::validate_runtime_config;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}\n");
        print_help();
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("missing arguments".to_string());
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    let command = parse_cli_options(&args)?;

    match command {
        CliCommand::Import { input } => {
            let runtime = import_runtime_config(&input)?;
            validate_runtime(&runtime, None)?;
            println!("validation passed");
        }
        CliCommand::Convert { input, output } => {
            let runtime = import_runtime_config(&input)?;
            let path = export_runtime(&runtime, &output)?;
            println!("exported output to {}", path.display());
        }
        CliCommand::Export { output } => {
            let _ = output;
            return Err(
                "export without an import context is not implemented yet; use convert".to_string(),
            );
        }
        CliCommand::ShowRuntime { name } => {
            println!(
                "show-runtime requested for vm '{}'; use convert/import to materialize runtime context",
                name
            );
        }
        CliCommand::Start { name } => {
            println!(
                "lifecycle action 'start' requested for vm '{}'; execution is not implemented yet",
                name
            );
        }
        CliCommand::Stop { name } => {
            println!(
                "lifecycle action 'stop' requested for vm '{}'; execution is not implemented yet",
                name
            );
        }
        CliCommand::Reset { name } => {
            println!(
                "lifecycle action 'reset' requested for vm '{}'; execution is not implemented yet",
                name
            );
        }
        CliCommand::Shutdown { name } => {
            println!(
                "lifecycle action 'shutdown' requested for vm '{}'; execution is not implemented yet",
                name
            );
        }
    }

    Ok(())
}

fn import_runtime_config(options: &ImportOptions) -> Result<RuntimeConfig, String> {
    match options {
        ImportOptions::Ezkvm { .. } => EzkvmImporter.import(options.clone()),
        ImportOptions::Proxmox { .. } => ProxmoxImporter.import(options.clone()),
        ImportOptions::Qemu { .. } => QemuImporter.import(options.clone()),
        ImportOptions::Libvirt { .. } => LibvirtImporter.import(options.clone()),
    }
    .map_err(|error| format!("import failed: {error}"))
}

fn validate_runtime(runtime: &RuntimeConfig, source_path: Option<&Path>) -> Result<(), String> {
    let fallback = PathBuf::from(format!("{}.yaml", runtime.metadata.vm_name));
    let path = source_path.unwrap_or(&fallback);

    validate_runtime_config(runtime, path).map_err(|error| match error.report() {
        Some(report) => {
            let formatter = ezkvm::runtime_config::DefaultReportFormatter::new();
            report.render_with(
                &formatter,
                ezkvm::runtime_config::ValidationReportFormat::Human,
            )
        }
        None => error.to_string(),
    })
}

fn export_runtime(runtime: &RuntimeConfig, options: &ExportOptions) -> Result<PathBuf, String> {
    match options {
        ExportOptions::Ezkvm { .. } => EzkvmExporter.export(runtime, options.clone()),
        ExportOptions::Proxmox { .. } => ProxmoxExporter.export(runtime, options.clone()),
        ExportOptions::Qemu { .. } => QemuExporter.export(runtime, options.clone()),
        ExportOptions::Libvirt { .. } => LibvirtExporter.export(runtime, options.clone()),
    }
    .map_err(|error| format!("export failed: {error}"))
}
