use crate::config::qemu::QemuCommandLineBuilder;
use crate::runtime::{
    GenericUsbDevice, UsbAddress, UsbBusDeviceKind, UsbDevice, UsbDeviceKind, UsbHostIdentity,
};

/// Dispatch over `UsbBusDeviceKind::Generic` (the only variant a `Q35Chipset.usb_bus` entry
/// can currently hold), then over the nested `UsbDeviceKind` for the concrete device model.
pub(crate) fn emit_usb_device(
    builder: &mut QemuCommandLineBuilder,
    address: &UsbAddress,
    device: &dyn UsbDevice,
) {
    match device.device_kind() {
        UsbBusDeviceKind::Generic => {
            let generic = device.as_any().downcast_ref::<GenericUsbDevice>().unwrap();
            match generic.kind() {
                UsbDeviceKind::HostPassthrough { identity } => {
                    match identity {
                        UsbHostIdentity::BusPort { bus, port } => builder.push_device(format!(
                            "-device usb-host,bus=xhci.0,port={},hostbus={},hostport={},id=usb{}",
                            address.port(),
                            bus,
                            port,
                            address.port()
                        )),
                        UsbHostIdentity::VendorProduct { vendor_id, product_id } => {
                            // Assumption A1: usb-host uses vendorid/productid hex flags; spot-check
                            // against `qemu-system-x86_64 -device usb-host,help` before treating as fully verified.
                            builder.push_device(format!(
                                "-device usb-host,bus=xhci.0,port={},vendorid=0x{},productid=0x{},id=usb{}",
                                address.port(),
                                vendor_id,
                                product_id,
                                address.port()
                            ))
                        }
                    };
                }
                UsbDeviceKind::Tablet => {
                    builder.push_device(format!(
                        "-device usb-tablet,bus=xhci.0,port={},id=usb{}",
                        address.port(),
                        address.port()
                    ));
                }
                UsbDeviceKind::NetworkController => {
                    builder.push_device(format!(
                        "-device usb-net,bus=xhci.0,port={},id=usb{}",
                        address.port(),
                        address.port()
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_07_03_usb_host_passthrough_parses_hostbus_hostport() {
        let device = GenericUsbDevice::new(UsbDeviceKind::HostPassthrough {
            identity: UsbHostIdentity::BusPort {
                bus: "1".to_string(),
                port: "2.2".to_string(),
            },
        });
        let address = UsbAddress::new("0".to_string());

        let mut builder = QemuCommandLineBuilder::new();
        emit_usb_device(&mut builder, &address, &device);
        let output = builder.build().to_string();

        assert!(output.contains("usb-host"), "output was: {output}");
        assert!(output.contains("hostbus=1"), "output was: {output}");
        assert!(output.contains("hostport=2.2"), "output was: {output}");
        assert!(output.contains("id=usb0"), "output was: {output}");
    }

    #[test]
    fn test_07_03_usb_host_passthrough_emits_vendor_product() {
        let device = GenericUsbDevice::new(UsbDeviceKind::HostPassthrough {
            identity: UsbHostIdentity::VendorProduct {
                vendor_id: "0451".to_string(),
                product_id: "16a0".to_string(),
            },
        });
        let address = UsbAddress::new("4".to_string());

        let mut builder = QemuCommandLineBuilder::new();
        emit_usb_device(&mut builder, &address, &device);
        let output = builder.build().to_string();

        assert!(output.contains("usb-host"), "output was: {output}");
        assert!(output.contains("vendorid=0x0451"), "output was: {output}");
        assert!(output.contains("productid=0x16a0"), "output was: {output}");
        assert!(output.contains("id=usb4"), "output was: {output}");
    }
}
