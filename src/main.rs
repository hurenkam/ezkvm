mod args;
mod yaml;
mod resource;

extern crate colored;

use std::{env, fmt, fs, process, thread};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::{Child, Command};

use home::home_dir;
use serde::{de, Deserialize, Deserializer, Serialize};
use crate::args::{EzkvmArguments, EzkvmCommand};

use chrono::Local;
use env_logger::Builder;
use log::{debug, info, warn, Level, LevelFilter};
use crate::colored::Colorize;
use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::os::unix::prelude::CommandExt;
use std::path::Path;
use std::thread::{sleep, spawn};
use std::time::Duration;
use nix::libc::geteuid;
use serde::de::{MapAccess, Visitor};
use crate::resource::lock::{EzkvmError, Lock};
use crate::resource::data_manager::DataManager;
use crate::resource::resource_pool::ResourcePool;
use crate::yaml::config::Config;
use crate::yaml::{SwtpmArgs,QemuArgs,LgClientArgs};

fn main() {
    let args = EzkvmArguments::new(env::args().collect());
    init_logger(args.log_level);

    let uid = nix::unistd::getuid();
    let gid = nix::unistd::getgid();
    let euid = nix::unistd::geteuid();
    let egid = nix::unistd::getegid();
    debug!("main()  euid: {}, egid: {}, uid: {}, gid: {}", euid, egid, uid, gid);
    debug!("main( {:?} )",args.command);

    let resource_manager = DataManager::instance();

    match args.command {
        EzkvmCommand::Start { name } => {
            let config = load_vm(format!("/etc/ezkvm/{}.yaml",name).as_str());

            if config.has_tpm() {
                if let Ok(child) = start_swtpm(&name, &config) {
                    // nothing to do with child yet
                }
            }

            if let Ok(lock) = start_vm(&name, &config) {
                // use lock
            } else {
                debug!("Unable to start the vm");
            }

            if config.has_lg() {
                if let Ok(child) = start_lg_client(&name,&config) {
                    //let output = child
                    //    .wait_with_output().unwrap();
                    //println!("Done {}", std::str::from_utf8(&output.stdout).unwrap());
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
}

fn get_qemu_uid_and_gid(config: &Config) -> (u32,u32) {
    let mut uid = u32::from(nix::unistd::geteuid());
    let mut gid = u32::from(nix::unistd::getegid());

    // if gtk ui is selected, qemu can not be run as root
    // so drop to actual uid/gid instead of euid/egid
    if let Some(display) = config.get_display() {
        if display.get_driver() == "gtk" {
            uid = u32::from(nix::unistd::getuid());
            gid = u32::from(nix::unistd::getgid());
        }
    }

    (uid,gid)
}

fn get_swtpm_uid_and_gid(config: &Config) -> (u32,u32) {
    // swtpm must run with same permissions as qemu otherwise
    // it can not connect to the socket created by qemu
    get_qemu_uid_and_gid(config)
}

fn get_lg_uid_and_gid(_config: &Config) -> (u32,u32) {
    // looking-glass-client can not be run as root
    // so drop to actual uid/gid instead of euid/egid
    (u32::from(nix::unistd::getuid()),u32::from(nix::unistd::getgid()))
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

fn start_vm(name: &String, config: &Config) -> Result<Lock, EzkvmError> {
    debug!("start_vm()");

    let args = config.get_qemu_args(0);
    let resources: Vec<String> = config.allocate_resources()?;

    let (uid,gid) = get_qemu_uid_and_gid(config);
    if let Ok(child) = Command::new("/usr/bin/env")
        .args(args)
        .uid(uid)
        .gid(gid)
        .spawn()
    {
        debug!("start_vm(): Started qemu with pid {}",child.id());

        Ok(Lock::new(
            name.clone(),
            child.id(),
            resources
        ))
    } else {
        debug!("start_vm(): Failed to start qemu");
        Err(EzkvmError::ExecError { file: name.clone() })
    }
}

fn start_swtpm(name: &String, config: &Config) -> Result<Child, EzkvmError> {
    debug!("start_swtpm()");

    let mut args = vec!["swtpm".to_string()];
    args.extend(config.get_swtpm_args(0));

    let (uid,gid) = get_swtpm_uid_and_gid(config);
    if let Ok(child) = Command::new("/usr/bin/env")
        .args(args.clone())
        .uid(uid)
        .gid(gid)
        .spawn()
    {
        debug!("start_swtpm(): Started swtpm with pid: {}, uid: {}, gid: {}\n{}",child.id(),uid,gid,args.join(" "));
        return Ok(child)
    }

    Err(EzkvmError::ExecError { file: name.clone() })
}

fn start_lg_client(name: &String, config: &Config) -> Result<Child, EzkvmError> {
    debug!("start_lg_client()");

    let mut args = vec!["looking-glass-client".to_string()];
    args.extend(config.get_lg_client_args(0));

    let (uid,gid) = get_lg_uid_and_gid(config);
    debug!("start_lg_client() uid: {}, gid: {}",uid,gid);

    let log_file = File::create("looking-glass-client.log").unwrap();
    let log = process::Stdio::from(log_file);
    let err_file = File::create("looking-glass-client.err").unwrap();
    let err = process::Stdio::from(err_file);

    let mut lg_cmd = Command::new("/usr/bin/env");
    lg_cmd
        .uid(uid)
        .gid(gid)
        .args(args.clone());
    match lg_cmd
        .stdout(log)
        .stderr(err)
        .spawn()
    {
        Ok(child) => {
            debug!("start_lg_client(): Started looking-glass-client with pid {}\n{}",child.id(),args.join(" "));
            return Ok(child)
        }
        Err(e) => {
            warn!("start_lg_client(): unable to start looking-glass-client due to error {}\n",e);
        }
    }

    Err(EzkvmError::ExecError { file: name.clone() })
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
