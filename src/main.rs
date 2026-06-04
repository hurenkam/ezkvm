mod cli;

use cli::{CliCommand, parse_cli_options, print_help};

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
            let runtime = input.import_runtime_config()?;
            runtime.validate_runtime(None)?;
            println!("validation passed");
        }
        CliCommand::Convert { input, output } => {
            let runtime = input.import_runtime_config()?;
            let path = output.export_runtime(&runtime)?;
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
