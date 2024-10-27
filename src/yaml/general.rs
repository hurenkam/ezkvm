use crate::yaml::QemuArgs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct General {
    name: String,
    uuid: Option<String>,
}
impl Default for General {
    fn default() -> Self {
        Self {
            name: "anonymous".to_string(),
            uuid: None,
        }
    }
}

impl QemuArgs for General {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            "-accel kvm".to_string(),
            //"-daemonize".to_string(),
            "-nodefaults".to_string(),
            format!(
                "-monitor unix:/var/ezkvm/{}.monitor,server,nowait",
                self.name
            ),
            format!(
                "-chardev socket,id=qmp,path=/var/ezkvm/{}.qmp,server=on,wait=off",
                self.name
            ),
            "-mon chardev=qmp,mode=control".to_string(),
            "-chardev socket,id=qmp-event,path=/var/run/qmeventd.sock,reconnect=5".to_string(),
            "-mon chardev=qmp-event,mode=control".to_string(),
        ]
    }
}
