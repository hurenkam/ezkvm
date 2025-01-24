extern crate getopts;

use getopts::Options;
use log::LevelFilter;

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum EzkvmCommand {
    Help,
    Start { name: String },

    Qga { name: String, cmd: String },
    QgaShutdown { name: String },
    QgaHibernate { name: String },

    Qmp { name: String, cmd: String },
    QmpQuit { name: String },
    QmpSystemReset { name: String },
    QmpSystemPowerDown { name: String },
    QmpSystemWakeUp { name: String },
}

pub struct EzkvmArguments {
    pub program: String,
    pub command: EzkvmCommand,
    pub opts: Options,
    pub log_level: LevelFilter,
}

// Usage:
// ezkvm --help                                                 -h
// ezkvm --vm <name> --start                                    -n <name> -r
// ezkvm --vm <name> --qga-shutdown                             -n <name> -q
// ezkvm --vm <name> --qga-hibernate                            -n <name> -p
// ezkvm --vm <name> --qga '{"execute": "guest-info"}'
// ezkvm --vm <name> --qmp-system-reset
// ezkvm --vm <name> --qmp-system-powerdown
// ezkvm --vm <name> --qmp-system-wakeup
// ezkvm --vm <name> --qmp-stop
// ezkvm --vm <name> --qmp '{"execute": "query-status"}'
impl EzkvmArguments {
    pub fn new(args: Vec<String>) -> Self {
        let mut command = EzkvmCommand::Help;
        let program = args[0].to_string();

        let mut opts = Options::new();
        opts.optopt("n","name","specify the vm","");
        opts.optflag("r", "start", "start a virtual machine");
        opts.optflag("q", "qga-shutdown", "shutdown a virtual machine through the guest-agent service of a vm");
        opts.optflag("p", "qga-hibernate", "hibernate a virtual machine through the guest-agent service of a vm");
        opts.optopt("", "qga", "send a command to the guest-agent service of a vm", "guest agent command");
        opts.optflag("", "qmp-system-reset", "send a reset command to the vm monitor service of a vm");
        opts.optflag("", "qmp-system-powerdown", "send a powerdown command to the vm monitor service of a vm");
        opts.optflag("", "qmp-system-wakeup", "send a wakeup command to the vm monitor service of a vm");
        opts.optopt("", "qmp", "send a command to the vm monitor service of a vm", "monitor command");
        opts.optflag("h", "help", "print usage message");

        let matches = match opts.parse(&args[1..]) {
            Ok(m) => m,
            Err(f) => {
                panic!("{}", f.to_string())
            }
        };

        if matches.opt_present("name") {
            if let Some(name) = matches.opt_str("name") {
                if matches.opt_present("start") {
                    command = EzkvmCommand::Start { name }
                } else if matches.opt_present("qga-shutdown") {
                    command = EzkvmCommand::QgaShutdown { name }
                } else if matches.opt_present("qga-hibernate") {
                    command = EzkvmCommand::QgaHibernate { name }
                } else if matches.opt_present("qmp-system-reset") {
                    command = EzkvmCommand::QmpSystemReset { name }
                } else if matches.opt_present("qmp-system-powerdown") {
                    command = EzkvmCommand::QmpSystemPowerDown { name }
                } else if matches.opt_present("qmp-system-wakeup") {
                    command = EzkvmCommand::QmpSystemWakeUp { name }
                } else if matches.opt_present("qga") {
                    match matches.opt_str("qga") {
                        None => {}
                        Some(cmd) => {
                            command = EzkvmCommand::Qga { name, cmd }
                        }
                    }
                } else if matches.opt_present("qmp") {
                    match matches.opt_str("qmp") {
                        None => {}
                        Some(cmd) => {
                            command = EzkvmCommand::Qmp { name, cmd }
                        }
                    }
                }
            }
        }

        if matches.opt_present("help") {
            match matches.opt_str("help") {
                None => {}
                Some(_name) => command = EzkvmCommand::Help,
            }
        }

        let log_level = LevelFilter::Off;
        EzkvmArguments {
            program,
            command,
            opts,
            log_level,
        }
    }

    pub fn print_usage(&self) {
        let brief = format!("Usage: {} [options]", self.program);
        print!("{}", self.opts.usage(&brief));
    }
}
