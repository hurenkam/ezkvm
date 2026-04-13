use anyhow::{Result, anyhow};

use crate::config::{UsbDeviceConfig, XhciControllerConfig};

use super::helpers::{is_valid_usb_hostport, is_valid_usb_spec};

pub(crate) fn validate_usb_device_config(usb_device: &UsbDeviceConfig) -> Result<()> {
    let has_host_spec = !usb_device.host.trim().is_empty();
    let has_hostbus = usb_device
        .hostbus
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_hostport = usb_device
        .hostport
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if has_host_spec {
        if !is_valid_usb_spec(&usb_device.host) {
            return Err(anyhow!(
                "Invalid USB device specification: {}. Expected format: 'bus-port.path', 'vendor:product', or use hostbus/hostport fields",
                usb_device.host
            ));
        }
    } else if !(has_hostbus && has_hostport) {
        return Err(anyhow!(
            "USB device '{}' requires either host or both hostbus and hostport",
            usb_device.id
        ));
    }

    if let Some(hostbus) = usb_device.hostbus.as_deref()
        && !hostbus.chars().all(|c| c.is_ascii_digit())
    {
        return Err(anyhow!("USB hostbus must be numeric: {}", hostbus));
    }

    if let Some(hostport) = usb_device.hostport.as_deref()
        && !is_valid_usb_hostport(hostport)
    {
        return Err(anyhow!("Invalid USB hostport format: {}", hostport));
    }

    Ok(())
}

pub(crate) fn validate_xhci_controller_config(
    xhci_controller: &XhciControllerConfig,
) -> Result<()> {
    if xhci_controller.id.trim().is_empty() {
        return Err(anyhow!("XHCI controller id cannot be empty"));
    }

    if let Some(p2) = xhci_controller.p2
        && p2 == 0
    {
        return Err(anyhow!(
            "XHCI controller p2 must be greater than 0 when specified"
        ));
    }

    if let Some(p3) = xhci_controller.p3
        && p3 == 0
    {
        return Err(anyhow!(
            "XHCI controller p3 must be greater than 0 when specified"
        ));
    }

    if let Some(bus) = &xhci_controller.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("XHCI controller bus cannot be empty"));
    }

    if let Some(addr) = &xhci_controller.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("XHCI controller addr cannot be empty"));
    }

    Ok(())
}
