mod args;
mod yaml;
mod resource;

extern crate colored;

use std::{env, fmt, fs};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use home::home_dir;
use serde::{de, Deserialize, Deserializer, Serialize};
use crate::args::{EzkvmArguments, EzkvmCommand};

use chrono::Local;
use env_logger::Builder;
use log::{debug, info, Level, LevelFilter};
use crate::colored::Colorize;
use std::io::Write;
use std::path::Path;
use serde::de::{MapAccess, Visitor};
use crate::resource::lock::{EzkvmError, Lock};
use crate::resource::data_manager::DataManager;
use crate::resource::resource_pool::ResourcePool;
use crate::yaml::config::Config;
use crate::yaml::QemuArgs;

fn main() {
    let args = EzkvmArguments::new(env::args().collect());
    init_logger(args.log_level);

    debug!("main({:?})",args.command);
    let resource_manager = DataManager::instance();

    match args.command {
        EzkvmCommand::Start { name } => {
            let config = load_vm(format!("/etc/ezkvm/{}.yaml",name).as_str());

            if let Ok(lock) = start_vm(name,config) {
                // use lock
            } else {
                debug!("Unable to start the vm");
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

fn load_vm(file: &str) -> Config {
    debug!("load_vm({})",file);

    let mut file = File::open(file).expect("Unable to open file");
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read file");

    serde_yaml::from_str(contents.as_str()).unwrap()
}

fn load_pool(file: &str) -> ResourcePool {
    debug!("load_vm({})",file);

    let mut file = File::open(file).expect("Unable to open file");
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read file");

    serde_yaml::from_str(contents.as_str()).unwrap()
}

fn start_vm(name: String, config: Config) -> Result<Lock, EzkvmError> {
    debug!("start_vm()");

    let args = config.get_qemu_args(0);
    let resources: Vec<String> = config.allocate_resources()?;

    let mut qemu_cmd = Command::new("/usr/bin/env");
    qemu_cmd.args(args);
    if let Ok(child) = qemu_cmd.spawn() {
        debug!("start_vm(): Started qemu with pid {}",child.id());

        Ok(Lock::new(
            name,
            child.id(),
            resources
        ))
    } else {
        debug!("start_vm(): Failed to start qemu");
        Err(EzkvmError::ExecError { file: name })
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

