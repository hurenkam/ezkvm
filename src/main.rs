mod cli;

use cli::{CliArgs, CliCommand, print_help};
use ezkvm::{config_format::ImportOptions, runtime_model::RuntimeModel};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}\n");
        print_help();
        std::process::exit(2);
    }
}

fn import_runtime(name: String) -> Result<RuntimeModel, String> {
    let runtime_config = ImportOptions::Ezkvm {
        host: "./dist/etc/ezkvm/host.yaml".to_string(),
        vm: name,
    }
    .import_runtime()?;
    RuntimeModel::try_from(runtime_config)
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
        CliCommand::Import { input } => {
            let runtime_config = input.import_runtime()?;
            runtime_config.validate_runtime(None)?;
            println!("validation passed");
        }
        CliCommand::Convert { input, output } => {
            let runtime_config = input.import_runtime()?;
            runtime_config.validate_runtime(None)?;
            let path = output.export_runtime(&runtime_config)?;
            println!("exported output to {}", path.display());
        }
        CliCommand::Export { output } => {
            let _ = output;
            return Err(
                "export without an import context is not implemented yet; use convert".to_string(),
            );
        }
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
