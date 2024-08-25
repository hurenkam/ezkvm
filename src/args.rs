extern crate getopts;

use getopts::Options;
use log::{LevelFilter};

#[derive(Debug)]
pub enum EzkvmCommand {
    Help,
    Start { name: String },
    Stop { name: String },
    Hibernate { name: String }
}

pub struct EzkvmArguments {
    pub program: String,
    pub command: EzkvmCommand,
    pub opts: Options,
    pub log_level: LevelFilter
}

impl EzkvmArguments {
    pub fn new(args: Vec<String>) -> Self {
        let mut command = EzkvmCommand::Help;
        let program = args[0].to_string();

        let mut opts = Options::new();
        opts.optopt(
            "",
            "help",
            "print usage message",
            "",
        );
        opts.optopt(
            "",
            "start",
            "start a virtual machine by name",
            "",
        );
        opts.optopt(
            "",
            "shutdown",
            "shutdown a virtual machine by name",
            "",
        );
        opts.optopt(
            "",
            "hibernate",
            "hibernate a virtual machine by name",
            "",
        );

        let matches = match opts.parse(&args[1..]) {
            Ok(m) => m,
            Err(f) => {
                panic!("{}", f.to_string())
            }
        };

        if matches.opt_present("start") {
            match matches.opt_str("start") {
                None => {},
                Some(name) => { command = EzkvmCommand::Start { name } }
            }
        }

        if matches.opt_present("shutdown") {
            match matches.opt_str("shutdown") {
                None => {},
                Some(name) => { command = EzkvmCommand::Stop { name } }
            }
        }

        if matches.opt_present("hibernate") {
            match matches.opt_str("hibernate") {
                None => {},
                Some(name) => { command = EzkvmCommand::Hibernate { name } }
            }
        }

        if matches.opt_present("help") {
            match matches.opt_str("help") {
                None => {},
                Some(name) => { command = EzkvmCommand::Help }
            }
        }

        let log_level = LevelFilter::Off;
        EzkvmArguments {
            program,
            command,
            opts,
            log_level
        }
    }

    pub fn print_usage(&self) {
        let brief = format!("Usage: {} [options]", self.program);
        print!("{}", self.opts.usage(&brief));
    }
}
