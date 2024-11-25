use crate::config::display::Display;
use crate::config::types::QemuDevice;
use crate::config::{Config};
use crate::osal::{Osal, OsalError};
use derive_getters::Getters;
use log::{debug, warn};
use serde::Deserialize;
use std::os::unix::prelude::CommandExt;
use std::process::{Child, Command};

fn yes() -> bool { true }
#[derive(Deserialize, Debug, Getters)]
pub struct RemoteViewer {
    #[serde(default="yes")]
    auto_resize: bool,
    #[serde(default)]
    full_screen: bool,
    // cursor
    // hotkeys
    // keymap
}

impl RemoteViewer {
    fn get_args(&self, config: &Config) -> Vec<String> {
        let mut result = vec![];
        match config.spice() {
            None => {}
            Some(spice) => result.extend(vec![
                "--connect".to_string(),
                format!("spice://{}:{}",spice.addr(),spice.port())
            ]),
        }

        if *self.auto_resize() {
            result.extend(vec!["--auto-resize".to_string()]);
        }
        if *self.full_screen() {
            result.extend(vec!["--full-screen".to_string()]);
        }

        result
    }

    fn start(&self, config: &Config) -> Result<Child, OsalError> {
        let (uid, gid) = config.get_default_uid_and_gid();
        debug!("start() uid: {}, gid: {}", uid, gid);

        let mut args = vec!["remote-viewer".to_string()];
        args.extend(self.get_args(config));

        Osal::execute_command(
            Command::new("/usr/bin/env").args(args).uid(uid).gid(gid),
            Some("remote-viewer".to_string()),
        )
    }
}

impl QemuDevice for RemoteViewer {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![]
    }

    fn post_start(&self, config: &Config) {
        match self.start(&config) {
            Ok(_child) => debug!("LookingGlass::post_start() succeeded"),
            Err(_error) => warn!("LookingGlass::post_start() failed"),
        }
    }
}

#[typetag::deserialize(name = "remote_viewer")]
impl Display for RemoteViewer {}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let display = RemoteViewer {
            auto_resize: false,
            full_screen: false,
        };
        let expected: Vec<String> = vec![];
        assert_eq!(display.get_qemu_args(0), expected);
    }
}
