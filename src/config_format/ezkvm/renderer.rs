use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    config_format::ezkvm::{
        Boot, Device, EZKVM_CONFIG_SCHEMA_VERSION, EzkvmConfigSchema, Machine, Metadata,
        VirtualMachine, builder::EzkvmHostSchema,
    },
    runtime_model::{BiosModel, NetworkResource, Resource, RuntimeModel, StorageResource},
};

#[derive(Default)]
pub struct EzkvmRuntimeModelRenderer {
    model: Option<RuntimeModel>,
    host_schema: Option<EzkvmHostSchema>,
}
impl EzkvmRuntimeModelRenderer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_runtime_model(mut self, model: RuntimeModel) -> Self {
        self.model = Some(model);
        self
    }
    pub fn with_host_schema(mut self, host_schema: EzkvmHostSchema) -> Self {
        self.host_schema = Some(host_schema);
        self
    }

    pub fn render(&self) -> Result<EzkvmConfigSchema, String> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| "missing runtime model".to_string())?;

        let _host_schema = self
            .host_schema
            .as_ref()
            .ok_or_else(|| "missing host schema".to_string())?;

        let chipset = machine_chipset(model.qemu_command());

        let mut resources = Vec::new();
        let mut storage_resource_ids: BTreeMap<String, String> = BTreeMap::new();
        let mut network_resource_ids: BTreeMap<String, String> = BTreeMap::new();

        let boot = render_boot(
            model,
            &mut resources,
            &mut storage_resource_ids,
            &mut network_resource_ids,
        )?;
        let tpm = render_tpm(
            model,
            &mut resources,
            &mut storage_resource_ids,
            &mut network_resource_ids,
        )?;
        let devices = render_devices(
            model,
            &mut resources,
            &mut storage_resource_ids,
            &mut network_resource_ids,
        )?;

        Ok(EzkvmConfigSchema {
            metadata: Metadata {
                schema_version: EZKVM_CONFIG_SCHEMA_VERSION.to_string(),
                vm_name: model.name().clone(),
            },
            virtual_machine: VirtualMachine {
                machine: Machine {
                    family: "pc".to_string(),
                    chipset,
                    version: None,
                },
                cpu: Some(model.cpu().clone()),
                memory: model.memory().clone(),
                boot,
                tpm,
                devices,
            },
            resources,
        })
    }
}

fn machine_chipset(command: Vec<String>) -> String {
    for window in command.windows(2) {
        if let [flag, value] = window
            && flag == "-machine"
        {
            if value.contains("q35") {
                return "q35".to_string();
            }
            if value.contains("i440fx") {
                return "i440fx".to_string();
            }
        }
    }
    "q35".to_string()
}

fn render_boot(
    model: &RuntimeModel,
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> Result<Boot, String> {
    match model.boot().bios() {
        BiosModel::SeaBios(_) => Ok(Boot::default()),
        BiosModel::Uefi(uefi) => {
            let resource = storage_resource_id(
                uefi.storage(),
                resources,
                storage_resource_ids,
                network_resource_ids,
            );
            serde_json::from_value(json!({ "uefi": { "resource": resource } }))
                .map_err(|e| format!("failed to render UEFI boot section: {e}"))
        }
    }
}

fn render_tpm(
    model: &RuntimeModel,
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> Result<Option<crate::runtime_model::Tpm>, String> {
    let Some(tpm) = model.tpm().as_ref() else {
        return Ok(None);
    };

    let tpm_text = format!("{tpm}");
    let Some(storage) = parse_storage_from_tpm_display(&tpm_text) else {
        return Ok(None);
    };

    let version = parse_tpm_version_from_display(&tpm_text).unwrap_or(2.0);
    let resource = storage_resource_id(
        &storage,
        resources,
        storage_resource_ids,
        network_resource_ids,
    );

    serde_json::from_value(json!({ "swtpm": { "version": version, "resource": resource } }))
        .map(Some)
        .map_err(|e| format!("failed to render TPM section: {e}"))
}

fn render_devices(
    model: &RuntimeModel,
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> Result<Vec<Device>, String> {
    let mut rendered = Vec::new();

    let busses = model.busses();

    let mut pcie_bus_ids: Vec<u8> = busses.pcie_busses().keys().copied().collect();
    pcie_bus_ids.sort_unstable();
    for pcie_bus in pcie_bus_ids {
        if let Some(controller) = busses.pcie_busses().get(&pcie_bus) {
            let mut addresses: Vec<_> = controller.devices().into_keys().collect();
            addresses.sort_by_key(|address| (address.device(), address.function()));
            for address in addresses {
                let args = controller
                    .devices()
                    .get(&address)
                    .map(|device| device.qemu_args(&pcie_bus, address.clone()))
                    .unwrap_or_default();

                if let Some(device) = render_pcie_device_from_args(
                    pcie_bus,
                    address.device(),
                    address.function(),
                    &args,
                    resources,
                    storage_resource_ids,
                    network_resource_ids,
                )? {
                    rendered.push(device);
                }
            }
        }
    }

    let mut ide_bus_ids: Vec<u8> = busses.ide_busses().keys().copied().collect();
    ide_bus_ids.sort_unstable();
    for ide_bus in ide_bus_ids {
        if let Some(controller) = busses.ide_busses().get(&ide_bus) {
            let mut addresses: Vec<_> = controller.devices().into_keys().collect();
            addresses.sort_by_key(|address| address.address);
            for address in addresses {
                let args = controller
                    .devices()
                    .get(&address)
                    .map(|device| device.qemu_args(&ide_bus, address.clone()))
                    .unwrap_or_default();
                if let Some(device) = render_storage_device_from_args(
                    "ide",
                    ide_bus,
                    address.address,
                    &args,
                    resources,
                    storage_resource_ids,
                    network_resource_ids,
                )? {
                    rendered.push(device);
                }
            }
        }
    }

    let mut sata_bus_ids: Vec<u8> = busses.sata_busses().keys().copied().collect();
    sata_bus_ids.sort_unstable();
    for sata_bus in sata_bus_ids {
        if let Some(controller) = busses.sata_busses().get(&sata_bus) {
            let mut addresses: Vec<_> = controller.devices().into_keys().collect();
            addresses.sort_by_key(|address| address.address);
            for address in addresses {
                let args = controller
                    .devices()
                    .get(&address)
                    .map(|device| device.qemu_args(&sata_bus, address.clone()))
                    .unwrap_or_default();
                if let Some(device) = render_storage_device_from_args(
                    "sata",
                    sata_bus,
                    address.address,
                    &args,
                    resources,
                    storage_resource_ids,
                    network_resource_ids,
                )? {
                    rendered.push(device);
                }
            }
        }
    }

    let mut scsi_bus_ids: Vec<u8> = busses.scsi_busses().keys().copied().collect();
    scsi_bus_ids.sort_unstable();
    for scsi_bus in scsi_bus_ids {
        if let Some(controller) = busses.scsi_busses().get(&scsi_bus) {
            let mut addresses: Vec<_> = controller.devices().into_keys().collect();
            addresses.sort_by_key(|address| (address.target, address.lun));
            for address in addresses {
                let args = controller
                    .devices()
                    .get(&address)
                    .map(|device| device.qemu_args(&scsi_bus, address.clone()))
                    .unwrap_or_default();
                if let Some(device) = render_scsi_device_from_args(
                    scsi_bus,
                    address.target,
                    address.lun,
                    &args,
                    resources,
                    storage_resource_ids,
                    network_resource_ids,
                )? {
                    rendered.push(device);
                }
            }
        }
    }

    Ok(rendered)
}

fn render_pcie_device_from_args(
    bus: u8,
    address_device: u8,
    address_function: u8,
    args: &[String],
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> Result<Option<Device>, String> {
    if args.iter().any(|value| value.contains("pvscsi")) {
        let value = json!({
            "pcie": {
                "bus": bus,
                "device": address_device,
                "function": address_function,
                "type": "pv_scsi"
            }
        });
        let device: Device = serde_json::from_value(value)
            .map_err(|e| format!("failed to render pv_scsi device: {e}"))?;
        return Ok(Some(device));
    }

    if let Some(netdev) = find_netdev_arg(args) {
        let resource = parse_network_resource_from_netdev_arg(netdev).map(|network| {
            network_resource_id(
                &network,
                resources,
                storage_resource_ids,
                network_resource_ids,
            )
        });
        let value = json!({
            "pcie": {
                "bus": bus,
                "device": address_device,
                "function": address_function,
                "type": "virtio_net",
                "resource": resource
            }
        });
        let device: Device = serde_json::from_value(value)
            .map_err(|e| format!("failed to render virtio_net device: {e}"))?;
        return Ok(Some(device));
    }

    Ok(None)
}

fn render_storage_device_from_args(
    bus_kind: &str,
    bus: u8,
    address: u8,
    args: &[String],
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> Result<Option<Device>, String> {
    let Some(drive_arg) = find_drive_arg(args) else {
        return Ok(None);
    };
    let Some(storage) = parse_storage_from_drive_arg(drive_arg) else {
        return Ok(None);
    };
    let resource = storage_resource_id(
        &storage,
        resources,
        storage_resource_ids,
        network_resource_ids,
    );

    let device_type = if drive_arg.contains("media=cdrom") {
        "cdrom"
    } else {
        "hdd"
    };

    let value = match bus_kind {
        "ide" => json!({
            "ide": {
                "bus": bus,
                "address": address,
                "type": device_type,
                "resource": resource
            }
        }),
        "sata" => json!({
            "sata": {
                "bus": bus,
                "address": address,
                "type": device_type,
                "resource": resource
            }
        }),
        _ => return Ok(None),
    };

    let device: Device = serde_json::from_value(value)
        .map_err(|e| format!("failed to render {bus_kind} device: {e}"))?;
    Ok(Some(device))
}

fn render_scsi_device_from_args(
    bus: u8,
    target: u8,
    lun: u8,
    args: &[String],
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> Result<Option<Device>, String> {
    let Some(drive_arg) = find_drive_arg(args) else {
        return Ok(None);
    };
    let Some(storage) = parse_storage_from_drive_arg(drive_arg) else {
        return Ok(None);
    };
    let resource = storage_resource_id(
        &storage,
        resources,
        storage_resource_ids,
        network_resource_ids,
    );
    let device_type = if drive_arg.contains("media=cdrom") {
        "cdrom"
    } else {
        "hdd"
    };

    let value = json!({
        "scsi": {
            "bus": bus,
            "address": { "target": target, "lun": lun },
            "type": device_type,
            "resource": resource
        }
    });

    let device: Device =
        serde_json::from_value(value).map_err(|e| format!("failed to render scsi device: {e}"))?;
    Ok(Some(device))
}

fn find_drive_arg(args: &[String]) -> Option<&str> {
    args.iter()
        .find(|value| value.contains("file=") && value.contains("if=none"))
        .map(String::as_str)
}

fn find_netdev_arg(args: &[String]) -> Option<&str> {
    args.iter()
        .find(|value| {
            value.starts_with("bridge,") || value.starts_with("tap,") || value.starts_with("user,")
        })
        .map(String::as_str)
}

fn parse_storage_from_drive_arg(drive_arg: &str) -> Option<StorageResource> {
    for entry in drive_arg.split(',') {
        if let Some(file) = entry.strip_prefix("file=") {
            if file.starts_with("/dev/") {
                return Some(StorageResource::BlockDevice {
                    block_device: file.to_string(),
                });
            }
            return Some(StorageResource::File {
                file: file.to_string(),
            });
        }
    }
    None
}

fn parse_network_resource_from_netdev_arg(netdev: &str) -> Option<NetworkResource> {
    for entry in netdev.split(',') {
        if let Some(bridge) = entry.strip_prefix("br=") {
            return Some(NetworkResource::Bridge {
                bridge: bridge.to_string(),
            });
        }
        if let Some(tap) = entry.strip_prefix("ifname=") {
            return Some(NetworkResource::Tap {
                tap: tap.to_string(),
            });
        }
    }
    None
}

fn parse_storage_from_tpm_display(display: &str) -> Option<StorageResource> {
    if let Some(value) = parse_quoted_after(display, "file: \"") {
        return Some(StorageResource::File { file: value });
    }
    if let Some(value) = parse_quoted_after(display, "block_device: \"") {
        return Some(StorageResource::BlockDevice {
            block_device: value,
        });
    }
    None
}

fn parse_tpm_version_from_display(display: &str) -> Option<f32> {
    let start = display.find("v")? + 1;
    let end = display[start..].find(',')? + start;
    display[start..end].parse::<f32>().ok()
}

fn parse_quoted_after(input: &str, prefix: &str) -> Option<String> {
    let start = input.find(prefix)? + prefix.len();
    let remainder = &input[start..];
    let end = remainder.find('"')?;
    Some(remainder[..end].to_string())
}

fn storage_resource_id(
    storage: &StorageResource,
    resources: &mut Vec<Resource>,
    storage_resource_ids: &mut BTreeMap<String, String>,
    _network_resource_ids: &mut BTreeMap<String, String>,
) -> String {
    let key = match storage {
        StorageResource::File { file } => format!("file:{file}"),
        StorageResource::BlockDevice { block_device } => format!("block:{block_device}"),
    };

    if let Some(id) = storage_resource_ids.get(&key) {
        return id.clone();
    }

    let id = format!("storage{}", storage_resource_ids.len());
    storage_resource_ids.insert(key, id.clone());
    resources.push(Resource::Storage {
        id: id.clone(),
        storage: storage.clone(),
    });
    id
}

fn network_resource_id(
    network: &NetworkResource,
    resources: &mut Vec<Resource>,
    _storage_resource_ids: &mut BTreeMap<String, String>,
    network_resource_ids: &mut BTreeMap<String, String>,
) -> String {
    let key = match network {
        NetworkResource::Tap { tap } => format!("tap:{tap}"),
        NetworkResource::Bridge { bridge } => format!("bridge:{bridge}"),
    };

    if let Some(id) = network_resource_ids.get(&key) {
        return id.clone();
    }

    let id = format!("net{}", network_resource_ids.len());
    network_resource_ids.insert(key, id.clone());
    resources.push(Resource::Network {
        id: id.clone(),
        network: network.clone(),
    });
    id
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::EzkvmRuntimeModelRenderer;
    use crate::config_format::ezkvm::{EzkvmConfigSchema, builder::EzkvmRuntimeModelBuilder};

    #[test]
    fn render_requires_runtime_model_and_host_schema() {
        let err = EzkvmRuntimeModelRenderer::new()
            .render()
            .expect_err("renderer should fail when runtime model is missing");
        assert_eq!(err, "missing runtime model");
    }

    #[test]
    fn render_reconstructs_resources_and_devices() {
        let config: EzkvmConfigSchema = serde_json::from_value(json!({
            "metadata": {
                "schema_version": "1.0.0",
                "vm_name": "demo"
            },
            "virtual_machine": {
                "machine": {
                    "family": "pc",
                    "chipset": "q35"
                },
                "memory": {
                    "size": 8589934592u64
                },
                "boot": {
                    "uefi": {
                        "resource": "firmware0"
                    }
                },
                "swtpm": {
                    "version": 2.0,
                    "resource": "tpmstate0"
                },
                "devices": [
                    {
                        "pcie": {
                            "type": "pv_scsi"
                        }
                    },
                    {
                        "scsi": {
                            "type": "hdd",
                            "resource": "disk0"
                        }
                    },
                    {
                        "pcie": {
                            "type": "virtio_net",
                            "resource": "net0"
                        }
                    },
                    {
                        "ide": {
                            "type": "cdrom",
                            "resource": "iso0"
                        }
                    }
                ]
            },
            "resources": [
                {
                    "id": "firmware0",
                    "storage": {
                        "file": "/var/lib/ezkvm/efivars.fd"
                    }
                },
                {
                    "id": "tpmstate0",
                    "storage": {
                        "file": "/var/lib/ezkvm/tpmstate"
                    }
                },
                {
                    "id": "disk0",
                    "storage": {
                        "block_device": "/dev/vm/disk0"
                    }
                },
                {
                    "id": "iso0",
                    "storage": {
                        "file": "/iso/debian.iso"
                    }
                },
                {
                    "id": "net0",
                    "network": {
                        "bridge": "vmbr0"
                    }
                }
            ]
        }))
        .expect("json should parse as ezkvm config schema");

        let runtime_model = EzkvmRuntimeModelBuilder::new()
            .with_vm_config(config)
            .build()
            .expect("runtime model should build");

        let rendered = EzkvmRuntimeModelRenderer::new()
            .with_runtime_model(runtime_model)
            .with_host_schema(crate::config_format::ezkvm::builder::EzkvmHostSchema::new(
                "host".to_string(),
            ))
            .render()
            .expect("renderer should produce output schema");

        assert!(rendered.resources.len() >= 4);
        assert!(rendered.virtual_machine.devices.len() >= 3);
        assert!(rendered.virtual_machine.tpm.is_some());
    }
}
