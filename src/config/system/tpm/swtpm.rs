use crate::config::system::tpm::Tpm;
use crate::config::types::QemuDevice;
use crate::config::Config;
//use crate::{get_swtpm_uid_and_gid};
use crate::osal::{Osal, OsalError};
use log::{debug, warn};
use serde::Deserialize;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct SwTpm {
    disk: String,
    socket: String,
}

#[typetag::deserialize(name = "swtpm")]
impl Tpm for SwTpm {}

impl SwTpm {
    #[cfg(test)]
    pub fn new(disk: String, socket: String) -> Self {
        Self { disk, socket }
    }
    fn spawn(&self, uid: u32, gid: u32, name: String) -> Result<Child, OsalError> {
        debug!(
            "SwTpm::spawn() uid: {}, gid: {}, name: {}",
            uid,
            gid,
            name.to_string()
        );

        Osal::execute_command(
            Command::new("/usr/bin/env")
                .args(self.get_args())
                .uid(uid)
                .gid(gid),
            Some("swtpm".to_string()),
        )
    }

    fn get_args(&self) -> Vec<String> {
        vec![
            "swtpm".to_string(),
            "socket".to_string(),
            "--tpmstate".to_string(),
            format!("backend-uri=file://{}", self.disk),
            "--ctrl".to_string(),
            format!("type=unixio,path={},mode=0600", self.socket),
            "--tpm2".to_string(),
        ]
    }
}

impl QemuDevice for SwTpm {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            format!(
                "-chardev socket,id=chrtpm{},path={}",
                index,
                self.socket.clone()
            ),
            format!("-tpmdev emulator,id=tpm{},chardev=chrtpm{}", index, index),
            format!("-device tpm-tis,tpmdev=tpm{}", index),
        ]
    }

    fn pre_start(&self, config: &Config) {
        debug!("SwTpm::start()");

        //let (uid, gid) = get_swtpm_uid_and_gid(config);
        let (uid, gid) = config.get_escalated_uid_and_gid();
        let name = config.general().name().clone();
        match self.spawn(uid, gid, name) {
            Ok(_child) => debug!("SwTpm::pre_start() succeeded"),
            Err(_error) => warn!("SwTpm::pre_start() failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let tpm = SwTpm {
            disk: "the_disk".to_string(),
            socket: "the_socket".to_string(),
        };
        assert_eq!(
            tpm.get_args(),
            vec![
                "swtpm".to_string(),
                "socket".to_string(),
                "--tpmstate".to_string(),
                "backend-uri=file://the_disk".to_string(),
                "--ctrl".to_string(),
                "type=unixio,path=the_socket,mode=0600".to_string(),
                "--tpm2".to_string()
            ]
        );
        assert_eq!(
            tpm.get_qemu_args(0),
            vec![
                "-chardev socket,id=chrtpm0,path=the_socket".to_string(),
                "-tpmdev emulator,id=tpm0,chardev=chrtpm0".to_string(),
                "-device tpm-tis,tpmdev=tpm0".to_string()
            ]
        );
    }
}
