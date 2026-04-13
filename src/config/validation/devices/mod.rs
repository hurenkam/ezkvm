mod display;
mod drive;
mod network;
mod serial;

use anyhow::Result;

use crate::config::DeviceConfig;

pub(crate) fn validate_device_config(devices: &DeviceConfig) -> Result<()> {
    for drive in &devices.drives {
        drive::validate_drive_config(drive)?;
    }

    for network in &devices.networks {
        network::validate_network_config(network)?;
    }

    for display in &devices.displays {
        display::validate_display_config(display)?;
    }

    for serial in &devices.serials {
        serial::validate_serial_config(serial)?;
    }

    Ok(())
}
