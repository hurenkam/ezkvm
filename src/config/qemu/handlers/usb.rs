use crate::config::qemu::QemuCommandLineBuilder;
use crate::runtime::{GenericUsbDevice, UsbAddress, UsbBusDeviceKind, UsbDevice, UsbDeviceKind};

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
                UsbDeviceKind::HostPassthrough { resource } => {
                    let (hostbus, hostport) = resource.split_once('-').expect(
                        "UsbDeviceKind::HostPassthrough.resource must be '<bus>-<port>' (D-06 — malformed data is a programmer error)",
                    );
                    builder.push_device(format!(
                        "-device usb-host,bus=xhci.0,port={},hostbus={},hostport={},id=usb{}",
                        address.port(),
                        hostbus,
                        hostport,
                        address.port()
                    ));
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
            resource: "1-2.2".to_string(),
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
}
