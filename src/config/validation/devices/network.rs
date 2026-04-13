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

    let backend = network.resolved_backend()?;
    let valid_backend_types = ["user", "bridge", "socket", "tap", "vhost-user"];
    if !valid_backend_types.contains(&backend.backend_type.as_str()) {
        return Err(anyhow!(
            "Unsupported network backend type: {}. Supported: {:?}",
            backend.backend_type,
            valid_backend_types
        ));
    }

    validate_backend_specific_fields(network, &backend)?;

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

fn validate_backend_specific_fields(
    network: &NetworkConfig,
    backend: &crate::config::NetworkBackendConfig,
) -> Result<()> {
    match backend.backend_type.as_str() {
        "user" => {
            if backend.ifname.is_some()
                || backend.script.is_some()
                || backend.downscript.is_some()
                || backend.bridge.is_some()
            {
                return Err(anyhow!(
                    "Network '{}' uses backend type 'user' but also sets tap/bridge-only backend fields",
                    network.id
                ));
            }
        }
        "tap" => {
            if let Some(ifname) = &backend.ifname
                && ifname.trim().is_empty()
            {
                return Err(anyhow!(
                    "Network '{}' tap ifname cannot be empty",
                    network.id
                ));
            }
        }
        "bridge" => {
            if let Some(bridge) = &backend.bridge
                && bridge.trim().is_empty()
            {
                return Err(anyhow!(
                    "Network '{}' bridge name cannot be empty",
                    network.id
                ));
            }
        }
        "socket" | "vhost-user" => {
            if let Some(listen) = &backend.listen
                && listen.trim().is_empty()
            {
                return Err(anyhow!(
                    "Network '{}' listen endpoint cannot be empty",
                    network.id
                ));
            }
            if let Some(connect) = &backend.connect
                && connect.trim().is_empty()
            {
                return Err(anyhow!(
                    "Network '{}' connect endpoint cannot be empty",
                    network.id
                ));
            }
        }
        _ => {}
    }

    if let Some(queues) = backend.queues
        && queues == 0
    {
        return Err(anyhow!(
            "Network '{}' backend queues must be greater than 0",
            network.id
        ));
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
