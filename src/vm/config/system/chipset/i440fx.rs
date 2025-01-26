use super::Chipset;
use super::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct I440FX {}

impl QemuDevice for I440FX {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![
            "-machine type=pc".to_string(),
            "-device pci-bridge,id=pci.1,chassis_nr=1,bus=pci.0,addr=0x1e".to_string(),
            "-device pci-bridge,id=pci.2,chassis_nr=2,bus=pci.0,addr=0x1f".to_string(),
            "-device piix3-usb-uhci,id=uhci,bus=pci.0,addr=0x1.0x2".to_string(),
        ]
    }
}

#[typetag::deserialize(name = "i440fx")]
impl Chipset for I440FX {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let i440fx = I440FX {};
        assert_eq!(
            i440fx.get_qemu_args(0),
            vec![
                "-machine type=pc".to_string(),
                "-device pci-bridge,id=pci.1,chassis_nr=1,bus=pci.0,addr=0x1e".to_string(),
                "-device pci-bridge,id=pci.2,chassis_nr=2,bus=pci.0,addr=0x1f".to_string(),
                "-device piix3-usb-uhci,id=uhci,bus=pci.0,addr=0x1.0x2".to_string()
            ]
        );
    }
}
