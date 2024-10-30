use crate::config::QemuDevice;
use crate::get_lg_uid_and_gid;
use crate::resource::lock::EzkvmError;
use crate::yaml::config::Config;
use derive_getters::Getters;
use log::{debug, warn};
use serde::Deserialize;
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::process;
use std::process::{Child, Command};

#[derive(Debug, Deserialize, PartialEq, Getters)]
pub struct LookingGlass {
    device: Device,
    window: Option<Window>,
    input: Option<Input>,
}
impl QemuDevice for LookingGlass {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut result = vec![
            "-vga none".to_string(),
            "-nographic".to_string(),
            "-device virtio-mouse".to_string(),
            "-device virtio-keyboard".to_string(),
        ];

        result.extend(self.device.get_qemu_args(index));

        result
    }

    fn post_start(&self, config: &Config) {
        match self.start_lg_client(&config) {
            Ok(_child) => debug!("LookingGlass::post_start() succeeded"),
            Err(_error) => warn!("LookingGlass::post_start() failed"),
        }
    }
}

impl LookingGlass {
    fn get_lg_client_args(&self, config: &Config) -> Vec<String> {
        let mut result = vec![format!("app:shmFile={}", self.device().path())];
        match self.window() {
            None => {}
            Some(window) => result.extend(vec![
                format!("win:fullScreen={}", window.full_screen()),
                format!("win:size={}", window.size()),
            ]),
        }
        match self.input() {
            None => {}
            Some(input) => result.extend(vec![
                format!("input:grabKeyboard={}", input.grab_keyboard()),
                format!("input:escapeKey={}", input.escape_key()),
            ]),
        }
        match config.spice() {
            None => {}
            Some(spice) => result.extend(vec![
                format!("spice:host={}", spice.addr()),
                format!("spice:port={}", spice.port()),
            ]),
        }

        result
    }

    fn start_lg_client(&self, config: &Config) -> Result<Child, EzkvmError> {
        debug!("start_lg_client()");

        let mut args = vec!["looking-glass-client".to_string()];
        args.extend(self.get_lg_client_args(config));

        let (uid, gid) = get_lg_uid_and_gid(config);
        debug!("start_lg_client() uid: {}, gid: {}", uid, gid);

        let log_file = File::create("looking-glass-client.log").unwrap();
        let log = process::Stdio::from(log_file);
        let err_file = File::create("looking-glass-client.err").unwrap();
        let err = process::Stdio::from(err_file);

        let mut lg_cmd = Command::new("/usr/bin/env");
        lg_cmd.uid(uid).gid(gid).args(args.clone());
        match lg_cmd.stdout(log).stderr(err).spawn() {
            Ok(child) => {
                debug!(
                    "start_lg_client(): Started looking-glass-client with pid {}\n{}",
                    child.id(),
                    args.join(" ")
                );
                return Ok(child);
            }
            Err(e) => {
                warn!(
                    "start_lg_client(): unable to start looking-glass-client due to error {}\n",
                    e
                );
            }
        }

        let name = config.general().name();
        Err(EzkvmError::ExecError { file: name.clone() })
    }
}

#[derive(Debug, Deserialize, PartialEq, Getters)]
pub struct Device {
    path: String,
    size: String,
}

impl QemuDevice for Device {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-device ivshmem-plain,memdev=ivshmem{},bus=pcie.0", index),
            format!(
                "-object memory-backend-file,id=ivshmem{},share=on,mem-path={},size={}",
                index, self.path, self.size
            ),
        ]
    }
}

#[derive(Debug, Deserialize, PartialEq, Getters)]
pub struct Window {
    size: String,
    full_screen: bool,
}

#[derive(Debug, Deserialize, PartialEq, Getters)]
pub struct Input {
    grab_keyboard: bool,
    escape_key: String,
}
