mod args;
pub mod config;
mod lock;
mod resource;
mod types;

use crate::args::{EzkvmArguments, EzkvmCommand};
use crate::colored::Colorize;
use crate::config::config::{Config, QemuDevice};
use crate::lock::lock_manager::LockManager;
use crate::resource::resource_collection::ResourceCollection;
use chrono::Local;
use env_logger::Builder;
use log::{debug, Level, LevelFilter};
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::Command;

extern crate colored;

fn main() {
    let args = EzkvmArguments::new(env::args().collect());
    init_logger(args.log_level);

    debug!("main({:?})", args.command);
    let resource_collection = ResourceCollection::instance();
    let lock_manager = LockManager::instance();

    match args.command {
        EzkvmCommand::Start { name } => {
            if let Ok(config) = read_config(format!("etc/{}.yaml", name)) {
                start_vm(config);
            } else {
                println!("Unable to start the vm");
            }
        }
        EzkvmCommand::Check { name } => {
            debug!("resource_collection: {:?}\n\n", resource_collection);
            let lock = lock_manager.create_lock("gyndine".to_string());
            debug!("lock_manager: {:?}\n\n", lock_manager);

            if let Ok(config) = read_config(format!("etc/{}.yaml", name)) {
                let _args = config.get_args(0);
            }
        }
        EzkvmCommand::Stop { name } => {
            todo!()
        }
        EzkvmCommand::Hibernate { name } => {
            todo!()
        }
        _ => {
            args.print_usage();
        }
    }
}

fn load_file(file: String) -> String {
    let mut file = File::open(file).expect("Unable to open file");
    let mut content = String::new();

    file.read_to_string(&mut content)
        .expect("Unable to read file");

    content
}

fn read_config(file: String) -> Result<Config, ()> {
    let config = load_file(file);
    match serde_yaml::from_str::<Config>(config.as_str()) {
        Ok(config) => Ok(config),
        Err(e) => {
            println!("Unable to parse the config file");
            debug!("Error: {}", e);
            Err(())
        }
    }
}

fn start_vm(config: Config) {
    let args = config.get_args(0);

    let mut qemu_cmd = Command::new("/usr/bin/env");
    qemu_cmd.args(args);
    if let Ok(child) = qemu_cmd.spawn() {
        debug!("start_vm(): Started qemu with pid {}", child.id());
    } else {
        debug!("start_vm(): Failed to start qemu");
    }
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

            let mut colorized_line = match record.level() {
                Level::Error => {
                    format!("{}", line.red().bold())
                }
                Level::Warn => {
                    format!("{}", line.yellow().bold())
                }
                Level::Info => {
                    format!("{}", line.bold())
                }
                Level::Debug => {
                    format!("{}", line)
                }
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
