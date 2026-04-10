extern crate getopts;

use getopts::Options;
use log::LevelFilter;

const DEFAULT_PROXMOX_STORAGE_PATH: &str = "/etc/pve/storage.cfg";

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum EzkvmCommand {
    Help,
    Start { name: String },
    ImportProxmoxConfig {
        input: String,
        storage: Option<String>,
        output: Option<String>,
        import_name: Option<String>,
        strict: bool,
        dry_run: bool,
    },

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
// ezkvm --vm <name> --qmp-quit
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
        opts.optflag("", "qmp-quit", "send a quit command to the vm monitor service of a vm");
        opts.optopt("", "qmp", "send a command to the vm monitor service of a vm", "monitor command");
        opts.optopt("", "import-proxmox", "import a proxmox vm config file into ezkvm yaml", "proxmox config file");
        opts.optopt("", "proxmox-storage", "optional proxmox storage.cfg file used to resolve volume IDs (defaults to /etc/pve/storage.cfg)", "storage.cfg file");
        opts.optopt("", "output", "write imported yaml to this file", "yaml output file");
        opts.optopt("", "import-name", "override imported vm name", "imported vm name");
        opts.optflag("", "strict", "fail import when warnings are present");
        opts.optflag("", "dry-run", "print generated yaml without writing output file");
        opts.optflag("h", "help", "print usage message");

        let matches = match opts.parse(&args[1..]) {
            Ok(m) => m,
            Err(f) => {
                panic!("{}", f.to_string())
            }
        };

        if let Some(input) = matches.opt_str("import-proxmox") {
            command = EzkvmCommand::ImportProxmoxConfig {
                input,
                storage: Some(
                    matches
                        .opt_str("proxmox-storage")
                        .unwrap_or_else(|| DEFAULT_PROXMOX_STORAGE_PATH.to_string()),
                ),
                output: matches.opt_str("output"),
                import_name: matches.opt_str("import-name"),
                strict: matches.opt_present("strict"),
                dry_run: matches.opt_present("dry-run"),
            }
        } else if matches.opt_present("name") {
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
                } else if matches.opt_present("qmp-quit") {
                    command = EzkvmCommand::QmpQuit { name }
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

#[cfg(test)]
mod tests {
    use super::{EzkvmArguments, EzkvmCommand};

    #[test]
    fn test_import_proxmox_defaults_storage_cfg_path() {
        let args = EzkvmArguments::new(vec![
            "ezkvm".to_string(),
            "--import-proxmox".to_string(),
            "100.conf".to_string(),
        ]);

        assert_eq!(
            args.command,
            EzkvmCommand::ImportProxmoxConfig {
                input: "100.conf".to_string(),
                storage: Some("/etc/pve/storage.cfg".to_string()),
                output: None,
                import_name: None,
                strict: false,
                dry_run: false,
            }
        );
    }

    #[test]
    fn test_import_proxmox_preserves_explicit_storage_cfg_path() {
        let args = EzkvmArguments::new(vec![
            "ezkvm".to_string(),
            "--import-proxmox".to_string(),
            "100.conf".to_string(),
            "--proxmox-storage".to_string(),
            "/tmp/storage.cfg".to_string(),
        ]);

        assert_eq!(
            args.command,
            EzkvmCommand::ImportProxmoxConfig {
                input: "100.conf".to_string(),
                storage: Some("/tmp/storage.cfg".to_string()),
                output: None,
                import_name: None,
                strict: false,
                dry_run: false,
            }
        );
    }
}
