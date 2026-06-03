use std::path::{Path, PathBuf};

mod cli;

use ezkvm::config_importer::{
    ConfigArgs, ConfigImportError, ConfigImporter, EzkvmConfigImporter, LibvirtConfigImporter,
    ProxmoxConfigImporter, QemuConfigImporter,
};
use ezkvm::runtime_config::{RuntimeConfig, validate_runtime_config};
use cli::{parse_cli_options, print_help, OutputSpec, SourceSpec};

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

    let options = parse_cli_options(&args)?;
    let runtime = import_runtime_config(&options.source)?;

    if options.validate {
        validate_runtime(&runtime, options.source.source_config_path.as_deref())?;
        println!("validation passed");
    }

    if options.show_runtime {
        print_runtime_layout(&runtime);
    }

    if let Some(output) = &options.output {
        let path = export_runtime_stub(&runtime, output)?;
        println!(
            "exported {} output to {}",
            output.output_type,
            path.display()
        );
    }

    Ok(())
}

fn import_runtime_config(source: &SourceSpec) -> Result<RuntimeConfig, String> {
    let config_args = ConfigArgs::new(source.importer_args.clone());
    match source.importer.as_str() {
        "ezkvm" => import_with(&EzkvmConfigImporter, config_args),
        "proxmox" => import_with(&ProxmoxConfigImporter, config_args),
        "qemu" => import_with(&QemuConfigImporter, config_args),
        "libvirt" => import_with(&LibvirtConfigImporter, config_args),
        other => Err(format!(
            "unsupported importer '{}'; expected one of: ezkvm, proxmox, qemu, libvirt",
            other
        )),
    }
}

fn import_with(
    importer: &dyn ConfigImporter<ConfigError = ConfigImportError>,
    config_args: ConfigArgs,
) -> Result<RuntimeConfig, String> {
    importer
        .import_config(config_args)
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

fn print_runtime_layout(runtime: &RuntimeConfig) {
    println!("RuntimeConfig layout:");
    println!("metadata:");
    println!("  schema_version: {}", runtime.metadata.schema_version);
    println!("  vm_name: {}", runtime.metadata.vm_name);
    println!("virtual_machine.system:");
    println!(
        "  machine: family={}, chipset={}",
        runtime.virtual_machine.system.machine.family,
        runtime.virtual_machine.system.machine.chipset
    );
    println!("  cpu: model={}", runtime.virtual_machine.system.cpu.model);
    println!(
        "  memory: min={}",
        runtime.virtual_machine.system.memory.min
    );
    print_collection_layout(
        "virtual_machine.storage",
        runtime
            .virtual_machine
            .storage
            .iter()
            .map(|entry| entry.id.as_str()),
    );
    print_collection_layout(
        "virtual_machine.network",
        runtime
            .virtual_machine
            .network
            .iter()
            .map(|entry| entry.id.as_str()),
    );
    print_collection_layout(
        "virtual_machine.resources",
        runtime
            .virtual_machine
            .resources
            .iter()
            .map(|entry| entry.id.as_str()),
    );
}

fn print_collection_layout<'a>(name: &str, ids: impl Iterator<Item = &'a str>) {
    let values: Vec<&str> = ids.collect();
    println!("{} ({}):", name, values.len());
    for id in values {
        println!("  - id={}", id);
    }
}

fn export_runtime_stub(runtime: &RuntimeConfig, output: &OutputSpec) -> Result<PathBuf, String> {
    let path = output
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(&runtime.metadata.vm_name, &output.output_type));

    let content = match output.output_type.as_str() {
        "qemu" => format!(
            "# stub exporter output\n# target=qemu\n# vm_name={}\n# TODO: connect runtime_resolution + render_stage\n",
            runtime.metadata.vm_name
        ),
        "ezkvm" => format!(
            "# stub exporter output\n# target=ezkvm\n# vm_name={}\n# TODO: emit canonical ezkvm yaml\n",
            runtime.metadata.vm_name
        ),
        other => {
            return Err(format!(
                "unsupported output type '{}'; expected one of: qemu, ezkvm",
                other
            ));
        }
    };

    std::fs::write(&path, content)
        .map_err(|error| format!("failed to write output {}: {}", path.display(), error))?;
    Ok(path)
}

fn default_output_path(vm_name: &str, output_type: &str) -> PathBuf {
    match output_type {
        "qemu" => PathBuf::from(format!("{}.qemu.cmd", vm_name)),
        "ezkvm" => PathBuf::from(format!("{}.yaml", vm_name)),
        other => PathBuf::from(format!("{}.{}.out", vm_name, other)),
    }
}
