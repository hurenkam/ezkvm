use crate::config::qemu_device::QemuDevice;
use crate::config::system::bios::Bios;
use derive_getters::Getters;
use serde::Deserialize;

const BOOT_SPLASH_FILE: &str = "/usr/share/ezkvm/bootsplash.jpg";

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone, Getters)]
pub struct SeaBios {
    uuid: String,
}
impl SeaBios {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self {
            uuid: "".to_string(),
        })
    }
}

impl QemuDevice for SeaBios {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            format!(
                "-boot menu=on,strict=on,reboot-timeout=1000,splash={}",
                BOOT_SPLASH_FILE
            ),
            format!("-smbios type=1,uuid={}", self.uuid()),
        ]
    }
}

#[typetag::deserialize(name = "seabios")]
impl Bios for SeaBios {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let seabios = SeaBios {
            uuid: "the_uuid".to_string(),
        };
        assert_eq!(seabios.get_qemu_args(0), vec![
            "-boot menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg".to_string(),
            "-smbios type=1,uuid=the_uuid".to_string(),
        ]);
    }
}
