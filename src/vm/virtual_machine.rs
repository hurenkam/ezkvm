use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;
use log::{debug, error, info};
use crate::config::{Config, QemuDevice};
use crate::osal::{Osal, OsalError};
use crate::resource::lock::Lock;
use crate::rpc::{GuestAgentService, GuestAgentServiceApi, MonitorService, MonitorServiceApi, SocketConnection};
use crate::config::Boolean;

#[derive(Clone,Debug,PartialEq)]
pub enum VirtualMachineError {
    NotConnected,
    CommandNotSupported,
    CommandFailed
}

pub struct VirtualMachine {
    name: String,
    config: Config
}

impl VirtualMachine {
    pub fn load(name: String) -> Self {
        debug!("VirtualMachine::load({})", name);

        let mut file = File::open(format!("/etc/ezkvm/{}.yaml",name)).expect("Unable to open file");
        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .expect("Unable to read file");

        Self {
            name,
            config: serde_yaml::from_str(contents.as_str()).expect("unable to load vm configuration")
        }
    }

    pub fn start(&self) -> Result<Lock, OsalError> {
        debug!("VirtualMachine[{}].Start()",self.name.clone());

        self.config.pre_start(&self.config);

        let (uid, gid) = self.config.get_escalated_uid_and_gid();

        let config_qemu_args = self.config.get_qemu_args(0);
        let mut args = "qemu-system-x86_64".to_string();
        for arg in config_qemu_args {
            //info!("{}", arg);
            args = format!("{} {}", args, arg).to_string();
        }
        info!("{}", args);
        let args: Vec<String> = args.split_whitespace().map(str::to_string).collect();

        let resources: Vec<String> = self.config.allocate_resources()?;

        let result = match Osal::execute_command(
            Command::new("/usr/bin/env").args(args).uid(uid).gid(gid),
            Some("qemu".to_string()),
        ) {
            Ok(child) => Ok(Lock::new(self.name.clone(), child.id(), resources)),
            Err(error) => Err(error),
        };

        self.config.post_start(&self.config);

        result
    }

    fn connect_guest_agent(&self) -> Result<Arc<GuestAgentService<SocketConnection>>, VirtualMachineError> {
        if self.config.general().agent().clone() == Boolean::No {
            error!("guest agent is not configured for this vm");
            return Err(VirtualMachineError::NotConnected);
        }

        let connection = SocketConnection::connect(
            format!("/var/ezkvm/{}.qga",self.name.clone())).map_err(|_|VirtualMachineError::NotConnected)?;
        let agent = GuestAgentService::new(connection);
        agent.sync().map_err(|_|VirtualMachineError::NotConnected)?;
        Ok(agent)
    }

    fn connect_monitor(&self) -> Result<Arc<MonitorService<SocketConnection>>, VirtualMachineError> {
        if self.config.general().monitor().clone() == Boolean::No {
            error!("monitor is not configured for this vm");
            return Err(VirtualMachineError::NotConnected);
        }

        let connection = SocketConnection::connect(
            format!("/var/ezkvm/{}.qga",self.name.clone())).map_err(|_|VirtualMachineError::NotConnected)?;
        let monitor = MonitorService::new(connection);
        //monitor.sync().map_err(|_|VirtualMachineError::NotConnected)?;
        Ok(monitor)
    }

    pub fn qga_shutdown(&self) -> Result<(), VirtualMachineError> {
        let agent = self.connect_guest_agent()?;
        agent.shutdown().map_err(|_|VirtualMachineError::CommandFailed)?;
        Ok(())
    }

    pub fn qga_hibernate(&self) -> Result<(), VirtualMachineError> {
        let agent = self.connect_guest_agent()?;
        agent.hibernate().map_err(|_|VirtualMachineError::CommandFailed)?;
        Ok(())
    }

    pub fn qga(&self, cmd: String) -> Result<(), VirtualMachineError> {
        let agent = self.connect_guest_agent()?;
        let result = agent.raw(cmd).map_err(|_|VirtualMachineError::CommandFailed)?;
        println!("{}",result);
        Ok(())
    }

    pub fn qmp_quit(&self) -> Result<(), VirtualMachineError> {
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp_system_reset(&self) -> Result<(), VirtualMachineError> {
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp_system_power_down(&self) -> Result<(), VirtualMachineError> {
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp_system_wake_up(&self) -> Result<(), VirtualMachineError> {
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp(&self, cmd: String) -> Result<(), VirtualMachineError> {
        let monitor = self.connect_monitor()?;
        let result = monitor.raw(cmd).map_err(|_|VirtualMachineError::CommandFailed)?;
        println!("{}",result);
        Ok(())
    }
}
