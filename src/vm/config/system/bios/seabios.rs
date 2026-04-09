use super::Bios;
use super::QemuDevice;
use derive_getters::Getters;
use serde::Deserialize;

const BOOT_SPLASH_FILE: &str = "/usr/share/ezkvm/bootsplash.jpg";

#[derive(Deserialize, Debug, Clone, Getters)]
pub struct SeaBios {
    uuid: String,
    #[serde(default = "SeaBios::boot_menu_default")]
    boot_menu: bool,
    #[serde(default = "SeaBios::boot_strict_default")]
    boot_strict: bool,
    #[serde(default = "SeaBios::reboot_timeout_default")]
    reboot_timeout: u32,
    #[serde(default)]
    boot_order: Option<String>,
    #[serde(default)]
    boot_once: Option<String>,
}
impl SeaBios {
    fn boot_menu_default() -> bool {
        true
    }

    fn boot_strict_default() -> bool {
        true
    }

    fn reboot_timeout_default() -> u32 {
        1000
    }

    pub fn boxed_default() -> Box<Self> {
        Box::new(Self {
            uuid: "".to_string(),
            boot_menu: Self::boot_menu_default(),
            boot_strict: Self::boot_strict_default(),
            reboot_timeout: Self::reboot_timeout_default(),
            boot_order: None,
            boot_once: None,
        })
    }

    fn boot_arg(&self) -> String {
        let menu = if self.boot_menu { "on" } else { "off" };
        let strict = if self.boot_strict { "on" } else { "off" };

        let mut boot = format!(
            "-boot menu={},strict={},reboot-timeout={},splash={}",
            menu, strict, self.reboot_timeout, BOOT_SPLASH_FILE
        );

        if let Some(ref order) = self.boot_order {
            boot.push_str(format!(",order={}", order).as_str());
        }
        if let Some(ref once) = self.boot_once {
            boot.push_str(format!(",once={}", once).as_str());
        }

        boot
    }
}

impl QemuDevice for SeaBios {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            self.boot_arg(),
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
            boot_menu: true,
            boot_strict: true,
            reboot_timeout: 1000,
            boot_order: None,
            boot_once: None,
        };
        assert_eq!(seabios.get_qemu_args(0), vec![
            "-boot menu=on,strict=on,reboot-timeout=1000,splash=/usr/share/ezkvm/bootsplash.jpg".to_string(),
            "-smbios type=1,uuid=the_uuid".to_string(),
        ]);
    }

    #[test]
    fn test_boot_policy_customization() {
        let seabios: SeaBios = serde_yaml::from_str(
            r#"
                uuid: id
                boot_menu: false
                boot_strict: false
                reboot_timeout: 2000
                boot_order: dc
                boot_once: d
            "#,
        )
        .unwrap();

        let boot = seabios.get_qemu_args(0)[0].clone();
        assert!(boot.contains("menu=off"));
        assert!(boot.contains("strict=off"));
        assert!(boot.contains("reboot-timeout=2000"));
        assert!(boot.contains("order=dc"));
        assert!(boot.contains("once=d"));
    }
}
