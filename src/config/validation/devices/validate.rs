use anyhow::Result;

use crate::config::DeviceConfig;

pub(crate) fn validate_device_config(devices: &DeviceConfig) -> Result<()> {
    for drive in &devices.drives {
        super::drive::validate_drive_config(drive)?;
    }

    for network in &devices.networks {
        super::network::validate_network_config(network)?;
    }

    for display in &devices.displays {
        super::display::validate_display_config(display)?;
    }

    for serial in &devices.serials {
        super::serial::validate_serial_config(serial)?;
    }

    Ok(())
}
