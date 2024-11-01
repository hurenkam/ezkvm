mod args;
mod config;
mod osal;
mod resource;

extern crate colored;

use crate::args::{EzkvmArguments, EzkvmCommand};
use std::env;
use std::fs::File;
use std::io::Read;
use std::process::Command;

use crate::colored::Colorize;
use crate::config::{Config, QemuDevice};
use crate::osal::{Osal, OsalError};
use crate::resource::data_manager::DataManager;
use crate::resource::lock::Lock;
use crate::resource::resource_pool::ResourcePool;
use chrono::Local;
use env_logger::Builder;
use log::{debug, Level, LevelFilter};
use std::io::Write;
use std::os::unix::prelude::CommandExt;

fn main() {
    let args = EzkvmArguments::new(env::args().collect());
    init_logger(args.log_level);
    debug!("main( {:?} )", args.command);

    let _resource_manager = DataManager::instance();

    match args.command {
        EzkvmCommand::Start { name } => handle_start_command(name),
        EzkvmCommand::Stop { .. } => todo!(),
        EzkvmCommand::Hibernate { .. } => todo!(),
        _ => args.print_usage(),
    }
}

fn handle_start_command(name: String) {
    let config = load_vm(format!("/etc/ezkvm/{}.yaml", name).as_str());

    config.pre_start(&config);

    if let Ok(_lock) = start_vm(&name, &config) {
        // use lock
    } else {
        debug!("Unable to start the vm");
    }

    config.post_start(&config);
}

fn load_vm(file: &str) -> Config {
    debug!("load_vm({})", file);

    let mut file = File::open(file).expect("Unable to open file");
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read file");

    serde_yaml::from_str(contents.as_str()).unwrap()
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

fn start_vm(name: &String, config: &Config) -> Result<Lock, OsalError> {
    debug!("start_vm()");

    let (uid, gid) = config.get_escalated_uid_and_gid();
    let args = config.get_qemu_args(0);
    let resources: Vec<String> = config.allocate_resources()?;

    match Osal::execute_command(
        Command::new("/usr/bin/env").args(args).uid(uid).gid(gid),
        Some("qemu".to_string()),
    ) {
        Ok(child) => Ok(Lock::new(name.clone(), child.id(), resources)),
        Err(error) => Err(error),
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
