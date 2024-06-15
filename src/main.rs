pub mod config;
mod args;

use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use log::{debug, Level, LevelFilter};
use serde::{Deserialize, Deserializer, Serialize};
use crate::config::config::{Config, QemuDevice};
use chrono::Local;
use crate::colored::Colorize;
use env_logger::Builder;
use std::io::Write;
use std::process::Command;
use crate::args::{EzkvmArguments, EzkvmCommand};

extern crate colored;

fn main() {
    let args = EzkvmArguments::new(env::args().collect());
    init_logger(args.log_level);

    debug!("main({:?})",args.command);
    //let resource_manager = DataManager::instance();

    match args.command {
        EzkvmCommand::Start { name } => {
            //let config = load_file(format!("/etc/ezkvm/{}.yaml",name));
            let config = load_file(format!("etc/{}.yaml",name));
            match serde_yaml::from_str::<Config>(config.as_str()) {
                Ok(config) => {
                    // start the vm
                    let args = config.get_args(0);

                    let mut qemu_cmd = Command::new("/usr/bin/env");
                    qemu_cmd.args(args);
                    if let Ok(child) = qemu_cmd.spawn() {
                        debug!("start_vm(): Started qemu with pid {}",child.id());
                    } else {
                        debug!("start_vm(): Failed to start qemu");
                        //Err(EzkvmError::ExecError { file: name })
                    }
                }
                Err(e) => {
                    println!("Unable to parse the config file");
                    debug!("Error: {}", e);
                }
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

/*
    init_logger(LevelFilter::Trace);

    let content = load_file("etc/gyndine.yaml");
    let config: Config = serde_yaml::from_str(content.as_str()).unwrap();

    println!("{:?}",config);
    let _ = config.get_args(0);
*/
}

fn load_file(file: String) -> String {
    let mut file = File::open(file).expect("Unable to open file");
    let mut content = String::new();

    file.read_to_string(&mut content)
        .expect("Unable to read file");

    content
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
