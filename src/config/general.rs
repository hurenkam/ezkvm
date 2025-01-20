use crate::config::default_when_missing;
use crate::config::types::Boolean;
use crate::config::QemuDevice;
use derive_getters::Getters;
use serde::Deserialize;

#[derive(Debug, Deserialize, Getters)]
pub struct General {
    name: String,
    uuid: Option<String>,
    #[serde(default, deserialize_with = "default_when_missing")]
    monitor: Boolean,
    #[serde(default, deserialize_with = "default_when_missing")]
    agent: Boolean,
}
impl Default for General {
    fn default() -> Self {
        Self {
            name: "anonymous".to_string(),
            uuid: None,
            monitor: Boolean::default(),
            agent: Boolean::default(),
        }
    }
}

impl General {
    fn get_agent_args(&self) -> Vec<String> {
        if self.agent == Boolean::Yes {
            vec![
                format!(
                    "-chardev socket,id=qga0,path=/var/ezkvm/{}.qga,server=on,wait=off",
                    self.name
                ),
                "-device virtio-serial,id=qga0,bus=pci.0,addr=0x8".to_string(),
                "-device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0".to_string(),
            ]
        } else {
            vec![]
        }
    }
    fn get_monitor_args(&self) -> Vec<String> {
        if self.monitor == Boolean::Yes {
            vec![
                format!(
                    "-monitor unix:/var/ezkvm/{}.monitor,server=on,nowait",
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
        } else {
            vec![]
        }
    }
}

impl QemuDevice for General {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut args = vec!["-accel kvm".to_string(), "-nodefaults".to_string()];

        args.extend(self.get_monitor_args());
        args.extend(self.get_agent_args());

        args
    }
}
