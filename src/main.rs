use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};
use ezkvm::lifecycle::{
    host_config::HostConfig,
    kill, reset,
    start::start,
    status::{StatusReport, status},
    stop,
};

#[derive(Debug, Parser)]
#[command(name = "ezkvm")]
#[command(about = "VM lifecycle management for ezkvm YAML configs")]
struct Cli {
    #[arg(long, default_value = "/etc/ezkvm", global = true)]
    config_dir: PathBuf,
    #[command(subcommand)]
    verb: Verb,
}

#[derive(Debug, Subcommand)]
enum Verb {
    Start {
        vm_name: String,
    },
    Stop {
        #[arg(long = "escalate-after")]
        escalate_after: Option<u64>,
        vm_name: String,
    },
    Kill {
        vm_name: String,
    },
    Reset {
        vm_name: String,
    },
    Status {
        vm_name: String,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(Some(message)) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<Option<String>, String> {
    let cli = Cli::parse();
    let host_config = HostConfig::load(&cli.config_dir).map_err(|err| err.to_string())?;

    match cli.verb {
        Verb::Start { vm_name } => {
            start(&host_config, &vm_name).map_err(|err| err.to_string())?;
            Ok(None)
        }
        Verb::Status { vm_name } => {
            let report =
                status(host_config.state_dir(), &vm_name).map_err(|err| err.to_string())?;
            Ok(Some(match report {
                StatusReport::Running {
                    qemu_pid,
                    swtpm_pid,
                    ui_client_pid,
                } => format!(
                    "running (qemu_pid={qemu_pid}, swtpm_pid={swtpm_pid:?}, ui_client_pid={ui_client_pid:?})"
                ),
                StatusReport::NotRunning => "not running".to_string(),
            }))
        }
        Verb::Stop {
            vm_name,
            escalate_after,
        } => {
            let effective_host = if let Some(timeout) = escalate_after {
                HostConfig::new(
                    host_config.vm_dir().clone(),
                    host_config.state_dir().clone(),
                    host_config.qemu_path().clone(),
                    host_config.qemu_default_args().clone(),
                    host_config.swtpm_path().clone(),
                    host_config.remote_viewer_path().clone(),
                    host_config.remote_viewer_default_args().clone(),
                    host_config.looking_glass_client_path().clone(),
                    host_config.looking_glass_client_default_args().clone(),
                    host_config.host_resources().clone(),
                    Some(timeout),
                )
            } else {
                host_config.clone()
            };
            stop::stop(&effective_host, &vm_name).map_err(|err| err.to_string())?;
            Ok(None)
        }
        Verb::Kill { vm_name } => {
            kill::kill(&host_config, &vm_name).map_err(|err| err.to_string())?;
            Ok(None)
        }
        Verb::Reset { vm_name } => {
            reset::reset(&host_config, &vm_name).map_err(|err| err.to_string())?;
            Ok(None)
        }
    }
}
