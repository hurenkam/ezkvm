mod cli;
mod config_format;
mod runtime_model;

use cli::{CliArgs, CliCommand, print_help};
use config_format::{
        EzkvmConfigFileStore, EzkvmRuntimeBuilder, EzkvmSchemaBuilder, ProxmoxRuntimeBuilder,
        ProxmoxSchemaBuilder, ProxmoxStorageConfig, RuntimeBuilder, SchemaBuilder,
    };
use runtime_model::RuntimeModel;

use crate::cli::ConfigOptions;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}\n");
        print_help();
        std::process::exit(2);
    }
}

fn import_runtime(name: String) -> Result<RuntimeModel, String> {
    let ezkvm_store = EzkvmConfigFileStore::new(std::path::PathBuf::from("./dist/etc/ezkvm/vm.d"));
    let ezkvm_config = ezkvm_store
        .load_config(&name)
        .expect("failed to load ezkvm config");

    EzkvmRuntimeBuilder::default()
        .with_schema(ezkvm_config)
        .with_host_path(std::path::PathBuf::from("./dist/etc/ezkvm/host.yaml"))
        .with_vm_name(name)
        .build()
}

fn run() -> Result<(), String> {
    let args: CliArgs = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("missing arguments".to_string());
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    let command = CliCommand::try_from(args)?;

    match command {
        CliCommand::Import(load_options) => match load_options.options {
            ConfigOptions::Qemu { qemu } => {
                println!(
                    "importing qemu config file from file '{file}'",
                    file = qemu.file
                );
            }
            ConfigOptions::Libvirt { libvirt } => {
                println!(
                    "importing libvirt config file from file '{file}'",
                    file = libvirt.file
                );
            }
            ConfigOptions::Proxmox { pve } => {
                println!(
                    "importing proxmox config file from storage '{storage}' and file '{file}', and save it with name '{name}'",
                    storage = pve.storage,
                    file = pve.file,
                    name = load_options.name
                );

                let ezkvm_store =
                    EzkvmConfigFileStore::new(std::path::PathBuf::from("./dist/etc/ezkvm/vm.d"));
                let storage_config =
                    ProxmoxStorageConfig::new(std::path::PathBuf::from(pve.storage));
                let schema = ProxmoxSchemaBuilder::default()
                    .with_storage_config(storage_config.clone())
                    .with_source_file(std::path::PathBuf::from(pve.file))
                    .build()?;
                let runtime_model = ProxmoxRuntimeBuilder::default()
                    .with_storage_config(storage_config)
                    .with_schema(schema)
                    .build()
                    .map_err(|e| e.to_string())?;

                let ezkvm_config = EzkvmSchemaBuilder::default()
                    .with_runtime(runtime_model)
                    .with_host_path(std::path::PathBuf::from("./dist/etc/ezkvm/host.yaml"))
                    .build()?;
                ezkvm_store
                    .save_config(&load_options.name, ezkvm_config)
                    .map_err(|e| e.to_string())?;
            }
        },
        CliCommand::Export(save_options) => match save_options.options {
            ConfigOptions::Qemu { qemu } => {
                println!(
                    "saving qemu runtime model to file '{file}'",
                    file = qemu.file
                );
            }
            ConfigOptions::Libvirt { libvirt } => {
                println!(
                    "saving libvirt runtime model to file '{file}'",
                    file = libvirt.file
                );
            }
            ConfigOptions::Proxmox { pve } => {
                println!(
                    "saving proxmox runtime model to storage '{storage}' and file '{file}', and save it with name '{name}'",
                    storage = pve.storage,
                    file = pve.file,
                    name = save_options.name
                );
            }
        },
        CliCommand::ShowRuntime { name } => {
            print!("{}", import_runtime(name)?);
        }
        CliCommand::Start { name } => {
            import_runtime(name)?.start()?;
        }
        CliCommand::Stop { name } => {
            import_runtime(name)?.stop()?;
        }
        CliCommand::Reset { name } => {
            import_runtime(name)?.reset()?;
        }
        CliCommand::Shutdown { name } => {
            import_runtime(name)?.shutdown()?;
        }
    }

    Ok(())
}
