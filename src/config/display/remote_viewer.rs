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
    render_node: Option<String>
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
                //"--connect".to_string(),
                format!("spice://{}:{}",spice.addr(),spice.port())
            ]),
        }

        if !*self.auto_resize() {
            result.extend(vec!["--auto-resize=never".to_string()]);
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
        let render_node = match &self.render_node {
            Some(render_node) => format!(",rendernode={}",render_node),
            None => "".to_string()
        };

        vec![
            format!("-display egl-headless{}",render_node)
        ]
    }

    fn post_start(&self, config: &Config) {
        match self.start(&config) {
            Ok(_child) => debug!("RemoteViewer::post_start() succeeded"),
            Err(_error) => warn!("RemoteViewer::post_start() failed"),
        }
    }
}

#[typetag::deserialize(name = "remote-viewer")]
impl Display for RemoteViewer {}


#[cfg(test)]
mod tests {
    use crate::config::Spice;
    use super::*;
    #[test]
    fn test_defaults() {
        let display = RemoteViewer {
            auto_resize: true,
            full_screen: true,
            render_node: None,
        };
        let expected: Vec<String> = vec!["-display egl-headless".to_string()];
        assert_eq!(display.get_qemu_args(0), expected);

        let expected: Vec<String> = vec!["--full-screen".to_string()];
        assert_eq!(display.get_args(&Config::default()), expected);
    }
    #[test]
    fn test_sane_values() {
        let display = RemoteViewer {
            auto_resize: false,
            full_screen: false,
            render_node: Some("/dev/dri/renderD128".to_string()),
        };
        let expected: Vec<String> = vec!["-display egl-headless,rendernode=/dev/dri/renderD128".to_string()];
        assert_eq!(display.get_qemu_args(0), expected);

        let expected: Vec<String> = vec!["spice://127.0.0.1:5900".to_string(),"--auto-resize=never".to_string()];
        assert_eq!(display.get_args(
            &Config::default()
                .with_spice(Some(Spice::new("127.0.0.1".to_string(),5900)))
        ), expected);
    }
}
