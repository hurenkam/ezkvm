pub mod config;

use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use log::{Level, LevelFilter};
use serde::{Deserialize, Deserializer, Serialize};
use crate::config::config::{Config, QemuDevice};
use chrono::Local;
use crate::colored::Colorize;
use env_logger::Builder;
use std::io::Write;

extern crate colored;

fn main() {
    init_logger(LevelFilter::Trace);

    let content = load_file("etc/gyndine.yaml");
    let config: Config = serde_yaml::from_str(content.as_str()).unwrap();
/*
    let config = Config {
        network: vec![
            Network::Pool   { pool: "x550t2".to_string(), mac: "BC:24:11:FF:76:89".to_string() },
            Network::Bridge { bridge: "vmbr0".to_string(), mac: "BC:24:11:FF:76:89".to_string(), driver: "virtio-net-pci".to_string() }
        ]
    };
    let config = serde_yaml::to_string(&config);
 */

    println!("{:?}",config);
    let _ = config.get_args(0);
}

fn load_file(file: &str) -> String {
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
