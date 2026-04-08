use super::config::{Boolean, Config, QemuDevice};
use crate::osal::{Osal, OsalError};
use crate::resource::lock::Lock;
use crate::rpc::{
    GuestAgentService, GuestAgentServiceApi, MonitorService, MonitorServiceApi, SocketConnection,
};
use log::{error, info, trace};
use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum VirtualMachineError {
    NotConnected,
    CommandNotSupported,
    CommandFailed,
}

pub struct VirtualMachine {
    name: String,
    config: Config,
}

impl VirtualMachine {
    pub fn load(name: String) -> Self {
        trace!("VirtualMachine::load({})", name);

        let mut file =
            File::open(format!("/etc/ezkvm/{}.yaml", name)).expect("Unable to open file");
        let mut contents = String::new();

        file.read_to_string(&mut contents)
            .expect("Unable to read file");

        Self {
            name,
            config: serde_yaml::from_str(contents.as_str())
                .expect("unable to load vm configuration"),
        }
    }

    pub fn start(&self) -> Result<Lock, OsalError> {
        trace!("VirtualMachine[{}].start()", self.name.clone());

        self.config.pre_start(&self.config);

        let (uid, gid) = self.config.get_escalated_uid_and_gid();

        let mut args: Vec<String> = vec!["qemu-system-x86_64".to_string()];
        args.extend(self.config.get_qemu_args(0));

        info!("{}", args.join(" "));

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

    fn connect_guest_agent(
        &self,
    ) -> Result<Arc<GuestAgentService<SocketConnection>>, VirtualMachineError> {
        trace!(
            "VirtualMachine[{}].connect_guest_agent()",
            self.name.clone()
        );

        if self.config.general().agent().clone() == Boolean::No {
            error!("guest agent is not configured for this vm");
            return Err(VirtualMachineError::NotConnected);
        }

        let connection = SocketConnection::connect(format!("/var/ezkvm/{}.qga", self.name.clone()))
            .map_err(|_| VirtualMachineError::NotConnected)?;

        GuestAgentService::new(connection).map_err(|_| VirtualMachineError::NotConnected)
    }

    fn connect_monitor(
        &self,
    ) -> Result<Arc<MonitorService<SocketConnection>>, VirtualMachineError> {
        trace!("VirtualMachine[{}].connect_monitor()", self.name.clone());

        if self.config.general().monitor().clone() == Boolean::No {
            error!("monitor is not configured for this vm");
            return Err(VirtualMachineError::NotConnected);
        }

        let connection = SocketConnection::connect(format!("/var/ezkvm/{}.qmp", self.name.clone()))
            .map_err(|_| VirtualMachineError::NotConnected)?;

        MonitorService::new(connection).map_err(|_| VirtualMachineError::NotConnected)
    }

    pub fn qga_shutdown(&self) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qga_shutdown()", self.name.clone());
        let agent = self.connect_guest_agent()?;
        agent
            .shutdown()
            .map_err(|_| VirtualMachineError::CommandFailed)?;
        Ok(())
    }

    pub fn qga_hibernate(&self) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qga_hibernate()", self.name.clone());
        let agent = self.connect_guest_agent()?;
        agent
            .hibernate()
            .map_err(|_| VirtualMachineError::CommandFailed)?;
        Ok(())
    }

    pub fn qga(&self, cmd: String) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qga()", self.name.clone());
        let agent = self.connect_guest_agent()?;
        let result = agent
            .raw(cmd)
            .map_err(|_| VirtualMachineError::CommandFailed)?;
        println!("{}", result);
        Ok(())
    }

    pub fn qmp_quit(&self) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qmp_quit()", self.name.clone());
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp_system_reset(&self) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qmp_system_reset()", self.name.clone());
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp_system_power_down(&self) -> Result<(), VirtualMachineError> {
        trace!(
            "VirtualMachine[{}].qmp_system_power_down()",
            self.name.clone()
        );
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp_system_wake_up(&self) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qmp_system_wake_up()", self.name.clone());
        Err(VirtualMachineError::CommandNotSupported)
    }

    pub fn qmp(&self, cmd: String) -> Result<(), VirtualMachineError> {
        trace!("VirtualMachine[{}].qmp()", self.name.clone());
        let monitor = self.connect_monitor()?;
        let result = monitor
            .raw(cmd)
            .map_err(|_| VirtualMachineError::CommandFailed)?;
        println!("{}", result);
        Ok(())
    }
}
