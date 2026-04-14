use super::QemuDevice;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Usb {
    BusPort {
        vm_port: String,
        host_bus: String,
        host_port: String,
    },
    VendorProduct {
        vendor_id: String,
        product_id: String,
    },
}

impl QemuDevice for Usb {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        match self {
            Usb::BusPort { vm_port, host_bus, host_port } => vec![format!(
                "-device usb-host,bus=xhci.0,port={},hostbus={},hostport={},id=usb{}",
                vm_port, host_bus, host_port, index
            )],
            Usb::VendorProduct { vendor_id, product_id } => vec![format!(
                "-device usb-host,vendorid=0x{},productid=0x{},id=usb{}",
                vendor_id, product_id, index
            )],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_port_usb_qemu_arg() {
        let usb = Usb::BusPort {
            vm_port: "1".to_string(),
            host_bus: "1".to_string(),
            host_port: "2.2".to_string(),
        };
        let args = usb.get_qemu_args(0);
        assert_eq!(args, vec!["-device usb-host,bus=xhci.0,port=1,hostbus=1,hostport=2.2,id=usb0"]);
    }

    #[test]
    fn test_vendor_product_usb_qemu_arg() {
        let usb = Usb::VendorProduct {
            vendor_id: "0451".to_string(),
            product_id: "16a0".to_string(),
        };
        let args = usb.get_qemu_args(0);
        assert_eq!(args, vec!["-device usb-host,vendorid=0x0451,productid=0x16a0,id=usb0"]);
    }

    #[test]
    fn test_vendor_product_usb_deserialize() {
        let yaml = "vendor_id: \"0451\"\nproduct_id: \"16a0\"\n";
        let usb: Usb = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(usb, Usb::VendorProduct { .. }));
        let args = usb.get_qemu_args(2);
        assert_eq!(args, vec!["-device usb-host,vendorid=0x0451,productid=0x16a0,id=usb2"]);
    }
}
