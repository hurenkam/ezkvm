mod args;
mod config;
mod resource;
mod yaml;

extern crate colored;

use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::{env, process};

use crate::args::{EzkvmArguments, EzkvmCommand};

use crate::colored::Colorize;
use crate::config::{Config, QemuDevice};
use crate::resource::data_manager::DataManager;
use crate::resource::lock::{EzkvmError, Lock};
use crate::resource::resource_pool::ResourcePool;
use chrono::Local;
use env_logger::Builder;
use log::{debug, Level, LevelFilter};
use std::io::Write;
use std::os::unix::prelude::CommandExt;
use std::thread::sleep;
use std::time::Duration;

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

    //sleep(Duration::from_secs(5));
    config.post_start(&config);
}

fn get_qemu_uid_and_gid(config: &Config) -> (u32, u32) {
    let mut uid = u32::from(nix::unistd::geteuid());
    let mut gid = u32::from(nix::unistd::getegid());

    // if gtk ui is selected, qemu can not be run as root
    // so drop to actual uid/gid instead of euid/egid
    if let Some(display) = config.display() {
        if display.get_driver() == "gtk" {
            uid = u32::from(nix::unistd::getuid());
            gid = u32::from(nix::unistd::getgid());
        }
    }

    (uid, gid)
}

fn get_swtpm_uid_and_gid(config: &Config) -> (u32, u32) {
    // swtpm must run with same permissions as qemu otherwise
    // it can not connect to the socket created by qemu
    get_qemu_uid_and_gid(config)
}

fn get_lg_uid_and_gid(_config: &Config) -> (u32, u32) {
    // looking-glass-client can not be run as root
    // so drop to actual uid/gid instead of euid/egid
    (
        u32::from(nix::unistd::getuid()),
        u32::from(nix::unistd::getgid()),
    )
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

fn start_vm(name: &String, config: &Config) -> Result<Lock, EzkvmError> {
    debug!("start_vm()");

    let args = config.get_qemu_args(0);
    let resources: Vec<String> = config.allocate_resources()?;

    let log_file = File::create("qemu.log").unwrap();
    let log = process::Stdio::from(log_file);
    let err_file = File::create("qemu.err").unwrap();
    let err = process::Stdio::from(err_file);

    let (uid, gid) = get_qemu_uid_and_gid(config);
    if let Ok(child) = Command::new("/usr/bin/env")
        .args(args)
        .uid(uid)
        .gid(gid)
        .stdout(log)
        .stderr(err)
        .spawn()
    {
        debug!("start_vm(): Started qemu with pid {}", child.id());
        Ok(Lock::new(name.clone(), child.id(), resources))
    } else {
        debug!("start_vm(): Failed to start qemu");
        Err(EzkvmError::ExecError { file: name.clone() })
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
