use anyhow::{Result, anyhow};

use crate::config::NetworkConfig;

pub(super) fn validate_network_config(network: &NetworkConfig) -> Result<()> {
    let valid_models = ["virtio-net", "virtio-net-pci", "e1000", "e1000e", "rtl8139"];
    if !valid_models.contains(&network.model.as_str()) {
        return Err(anyhow!(
            "Unsupported network model: {}. Supported: {:?}",
            network.model,
            valid_models
        ));
    }

    let valid_mode_prefixes = ["user", "bridge", "socket", "tap"];
    let mode_valid = valid_mode_prefixes
        .iter()
        .any(|prefix| network.mode.starts_with(prefix));
    if !mode_valid {
        return Err(anyhow!(
            "Unsupported network mode: {}. Must start with one of: {:?}",
            network.mode,
            valid_mode_prefixes
        ));
    }

    if let Some(mac) = &network.mac
        && !is_valid_mac_address(mac)
    {
        return Err(anyhow!("Invalid MAC address format: {}", mac));
    }

    if let Some(rx_queue_size) = network.rx_queue_size
        && rx_queue_size == 0
    {
        return Err(anyhow!("RX queue size must be greater than 0"));
    }

    if let Some(tx_queue_size) = network.tx_queue_size
        && tx_queue_size == 0
    {
        return Err(anyhow!("TX queue size must be greater than 0"));
    }

    if let Some(bus) = &network.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Network bus cannot be empty"));
    }

    if let Some(addr) = &network.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("Network device address cannot be empty"));
    }

    Ok(())
}

fn is_valid_mac_address(mac: &str) -> bool {
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return false;
    }

    for part in parts {
        if part.len() != 2 {
            return false;
        }
        if u8::from_str_radix(part, 16).is_err() {
            return false;
        }
    }

    true
}
