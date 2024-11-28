use crate::config::display::Display;
use crate::config::types::QemuDevice;
use crate::config::{default_when_missing, Config};
use crate::osal::{Osal, OsalError};
use derive_getters::Getters;
use log::{debug, warn};
use serde::Deserialize;
use std::os::unix::prelude::CommandExt;
use std::process::{Child, Command};
use crate::config::spice::SpiceSocket;

#[derive(Deserialize, Debug, Getters)]
pub struct LookingGlass {
    device: Device,
    #[serde(default, deserialize_with = "default_when_missing")]
    window: Option<Window>,
    #[serde(default, deserialize_with = "default_when_missing")]
    input: Option<Input>,
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
            None => {},
            Some(spice) => {
                match spice.socket() {
                    SpiceSocket::TcpPort { addr, port } => {
                        result.extend(vec![
                            format!("spice:host={}", addr),
                            format!("spice:port={}", port),
                        ])
                    },
                    SpiceSocket::UnixSocket { path, .. } => {
                        result.extend(vec![
                            format!("spice:host={}", path),
                            format!("spice:port={}", 0),
                        ])
                    }
                }
            }
        }

        result
    }

    fn start_lg_client(&self, config: &Config) -> Result<Child, OsalError> {
        let (uid, gid) = config.get_default_uid_and_gid();
        debug!("start_lg_client() uid: {}, gid: {}", uid, gid);

        let mut args = vec!["looking-glass-client".to_string()];
        args.extend(self.get_lg_client_args(config));

        Osal::execute_command(
            Command::new("/usr/bin/env").args(args).uid(uid).gid(gid),
            Some("looking-glass-client".to_string()),
        )
    }
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

#[typetag::deserialize(name = "looking_glass")]
impl Display for LookingGlass {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let display = LookingGlass {
            device: Device {
                path: "".to_string(),
                size: "".to_string(),
            },
            window: None,
            input: None,
        };
        let expected: Vec<String> = vec![
            "-vga none".to_string(),
            "-nographic".to_string(),
            "-device virtio-mouse".to_string(),
            "-device virtio-keyboard".to_string(),
            "-device ivshmem-plain,memdev=ivshmem0,bus=pcie.0".to_string(),
            "-object memory-backend-file,id=ivshmem0,share=on,mem-path=,size=".to_string(),
        ];
        assert_eq!(display.get_qemu_args(0), expected);
    }
}
