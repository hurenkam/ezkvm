use derive_getters::Getters;
use derive_new::new;

use crate::config::proxmox::ProxmoxImportError;
use crate::runtime::UsbDevice;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UsbHostIdentity {
    BusPort { bus: String, port: String },
    VendorProduct { vendor_id: String, product_id: String },
}

impl UsbHostIdentity {
    pub fn parse(raw: &str) -> Result<Self, ProxmoxImportError> {
        if let Some((bus, port)) = raw.split_once('-') {
            if !bus.is_empty() && !port.is_empty() {
                return Ok(Self::BusPort {
                    bus: bus.to_string(),
                    port: port.to_string(),
                });
            }
        } else if let Some((vendor_id, product_id)) = raw.split_once(':') {
            if !vendor_id.is_empty() && !product_id.is_empty() {
                return Ok(Self::VendorProduct {
                    vendor_id: vendor_id.to_string(),
                    product_id: product_id.to_string(),
                });
            }
        }

        Err(ProxmoxImportError::MalformedUsbHostIdentity {
            raw: raw.to_string(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbDeviceKind {
    NetworkController,
    Tablet,
    HostPassthrough { identity: UsbHostIdentity },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct GenericUsbDevice {
    kind: UsbDeviceKind,
}

impl UsbDevice for GenericUsbDevice {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::UsbBusDeviceKind {
        crate::runtime::UsbBusDeviceKind::Generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_host_identity_parses_bus_port() {
        assert_eq!(
            UsbHostIdentity::parse("1-2.2").unwrap(),
            UsbHostIdentity::BusPort {
                bus: "1".to_string(),
                port: "2.2".to_string(),
            }
        );
    }

    #[test]
    fn usb_host_identity_parses_vendor_product() {
        assert_eq!(
            UsbHostIdentity::parse("0451:16a0").unwrap(),
            UsbHostIdentity::VendorProduct {
                vendor_id: "0451".to_string(),
                product_id: "16a0".to_string(),
            }
        );
    }

    #[test]
    fn usb_host_identity_rejects_malformed_value() {
        assert!(matches!(
            UsbHostIdentity::parse("garbage"),
            Err(ProxmoxImportError::MalformedUsbHostIdentity { raw }) if raw == "garbage"
        ));
    }
}
