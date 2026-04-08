extern crate colored;
mod args;
mod import;
mod osal;
mod resource;
mod rpc;
mod vm;

use crate::args::{EzkvmArguments, EzkvmCommand};
use std::env;
use std::fs::File;
use std::io::Read;

use crate::colored::Colorize;
use crate::resource::data_manager::DataManager;
use crate::resource::resource_pool::ResourcePool;
use crate::vm::VirtualMachine;
use chrono::Local;
use env_logger::Builder;
use log::{debug, Level, LevelFilter};
use std::io::Write;

fn main() {
    let args = EzkvmArguments::new(env::args().collect());
    init_logger(args.log_level);
    debug!("main( {:?} )", args.command);

    let _resource_manager = DataManager::instance();

    match args.command {
        EzkvmCommand::Start { name } => {
            VirtualMachine::load(name)
                .start()
                .expect("unable to start vm");
        }
        EzkvmCommand::QgaShutdown { name } => {
            VirtualMachine::load(name)
                .qga_shutdown()
                .expect("unable to shutdown vm");
        }
        EzkvmCommand::QgaHibernate { name } => {
            VirtualMachine::load(name)
                .qga_hibernate()
                .expect("unable to hibernate vm");
        }
        EzkvmCommand::Qga { name, cmd } => {
            VirtualMachine::load(name)
                .qga(cmd)
                .expect("unable to execute guest-agent command");
        }
        EzkvmCommand::QmpQuit { name } => {
            VirtualMachine::load(name)
                .qmp_quit()
                .expect("unable to quit the vm");
        }
        EzkvmCommand::QmpSystemReset { name } => {
            VirtualMachine::load(name)
                .qmp_system_reset()
                .expect("unable to reset the vm");
        }
        EzkvmCommand::QmpSystemPowerDown { name } => {
            VirtualMachine::load(name)
                .qmp_system_power_down()
                .expect("unable to power down the vm");
        }
        EzkvmCommand::QmpSystemWakeUp { name } => {
            VirtualMachine::load(name)
                .qmp_system_wake_up()
                .expect("unable to wake up the vm");
        }

        EzkvmCommand::Qmp { name, cmd } => {
            VirtualMachine::load(name)
                .qmp(cmd)
                .expect("unable to execute monitor command");
        }
        EzkvmCommand::ImportProxmoxConfig {
            input,
            output,
            import_name,
            strict,
            dry_run,
        } => {
            let summary = import::io::import_from_file(
                &input,
                output.as_deref(),
                import_name.as_deref(),
                strict,
                dry_run,
            )
            .expect("unable to import proxmox configuration");
            println!("{}", summary);
        }
        _ => args.print_usage(),
    }
}

#[allow(dead_code)]
fn load_pool(file: &str) -> ResourcePool {
    debug!("load_pool({})", file);

    let mut file = File::open(file).expect("Unable to open file");
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read file");

    serde_yaml::from_str(contents.as_str()).unwrap()
}

fn init_logger(log_level: LevelFilter) {
    Builder::new()
        .format(|buf, record| {
            let path = record.module_path().unwrap_or("");
            let line = format!(
                "[{} {}  {}:{}]: {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S.%3f"),
                record.level(),
                path,
                record.line().unwrap_or(0),
                record.args()
            );

            let colorized_line = match record.level() {
                Level::Error => {
                    format!("{}", line.red().bold())
                }
                Level::Warn => {
                    format!("{}", line.yellow().bold())
                }
                Level::Info => {
                    format!("{}", line.bold())
                }
                Level::Debug => line.to_string(),
                Level::Trace => {
                    format!("{}", line.dimmed())
                }
            };

            writeln!(buf, "{}", colorized_line.green())
        })
        .filter(None, log_level)
        .parse_default_env()
        .init();
}
