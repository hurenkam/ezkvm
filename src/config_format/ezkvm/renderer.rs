use std::collections::BTreeMap;

use serde_json::json;

use crate::{
    config_format::ezkvm::{
        Boot, Device, EZKVM_CONFIG_SCHEMA_VERSION, EzkvmConfigSchema, HostSchema, Machine,
        Metadata, VirtualMachine, builder::EzkvmHostSchema,
    },
    runtime_model::{
        Audio, BiosModel, Chipset, Display, GuestAgent, NetworkResource, PcieDeviceKind, Resource,
        RuntimeModel, StorageDeviceKind, StorageResource,
    },
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

        let chipset = machine_chipset(model.chipset());

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
        let display = render_display(model);
        let audio = render_audio(model);
        let guest_agent = render_guest_agent(model);
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
            host: HostSchema {
                display: None,
                audio: None,
                resources,
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
                display,
                audio,
                guest_agent,
                devices,
            },
        })
    }
}

fn machine_chipset(chipset: &Chipset) -> String {
    match chipset {
        Chipset::Q35(_) => "q35".to_string(),
        Chipset::I440FX(_) => "i440fx".to_string(),
    }
}

fn render_display(model: &RuntimeModel) -> Option<Display> {
    model.display().as_ref().map(|d| d.config().clone())
}

fn render_audio(model: &RuntimeModel) -> Option<Audio> {
    model.audio().as_ref().map(|a| a.config().clone())
}

fn render_guest_agent(model: &RuntimeModel) -> Option<GuestAgent> {
    model.guest_agent().as_ref().map(|ga| ga.config().clone())
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

    let Some(storage) = tpm.storage_resource().cloned() else {
        return Ok(None);
    };

    let version = tpm.swtpm_version().unwrap_or(2.0);
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

    let mut ctx = RenderContext {
        resources,
        storage_resource_ids,
        network_resource_ids,
    };

    let busses = model.busses();

    let mut pcie_bus_ids: Vec<u8> = busses.pcie_busses().keys().copied().collect();
    pcie_bus_ids.sort_unstable();
    for pcie_bus in pcie_bus_ids {
        if let Some(controller) = busses.pcie_busses().get(&pcie_bus) {
            let mut addresses: Vec<_> = controller.devices().into_keys().collect();
            addresses.sort_by_key(|address| (address.device(), address.function()));
            for address in addresses {
                if let Some(device_impl) = controller.devices().get(&address)
                    && let Some(device) = render_pcie_device(
                        pcie_bus,
                        address.device(),
                        address.function(),
                        device_impl.as_ref(),
                        &mut ctx,
                    )?
                {
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
                if let Some(device_impl) = controller.devices().get(&address)
                    && let Some(device) = render_storage_device(
                        "ide",
                        ide_bus,
                        address.address,
                        device_impl.storage_resource(),
                        device_impl.storage_kind(),
                        &mut ctx,
                    )?
                {
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
                if let Some(device_impl) = controller.devices().get(&address)
                    && let Some(device) = render_storage_device(
                        "sata",
                        sata_bus,
                        address.address,
                        device_impl.storage_resource(),
                        device_impl.storage_kind(),
                        &mut ctx,
                    )?
                {
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
                if let Some(device_impl) = controller.devices().get(&address)
                    && let Some(device) = render_scsi_device(
                        scsi_bus,
                        address.target,
                        address.lun,
                        device_impl.storage_resource(),
                        device_impl.storage_kind(),
                        &mut ctx,
                    )?
                {
                    rendered.push(device);
                }
            }
        }
    }

    Ok(rendered)
}

fn render_pcie_device(
    bus: u8,
    address_device: u8,
    address_function: u8,
    device: &dyn crate::runtime_model::PcieDeviceApi,
    ctx: &mut RenderContext<'_>,
) -> Result<Option<Device>, String> {
    match device.device_kind() {
        PcieDeviceKind::PvScsi => {
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
            Ok(Some(device))
        }
        PcieDeviceKind::VirtioNet => {
            let resource = device.network_resource().map(|network| {
                network_resource_id(
                    network,
                    ctx.resources,
                    ctx.storage_resource_ids,
                    ctx.network_resource_ids,
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
            Ok(Some(device))
        }
        PcieDeviceKind::StandardGpu => {
            let value = json!({
                "pcie": {
                    "bus": bus,
                    "device": address_device,
                    "function": address_function,
                    "type": "standard_gpu"
                }
            });
            let device: Device = serde_json::from_value(value)
                .map_err(|e| format!("failed to render standard_gpu device: {e}"))?;
            Ok(Some(device))
        }
        PcieDeviceKind::VirtioGpu => {
            let value = json!({
                "pcie": {
                    "bus": bus,
                    "device": address_device,
                    "function": address_function,
                    "type": "virtio_gpu"
                }
            });
            let device: Device = serde_json::from_value(value)
                .map_err(|e| format!("failed to render virtio_gpu device: {e}"))?;
            Ok(Some(device))
        }
        PcieDeviceKind::PassthroughGpu => {
            let resource = device
                .resource_id()
                .ok_or_else(|| "missing passthrough_gpu resource id".to_string())?;
            let value = json!({
                "pcie": {
                    "bus": bus,
                    "device": address_device,
                    "function": address_function,
                    "type": "passthrough_gpu",
                    "resource": resource
                }
            });
            let device: Device = serde_json::from_value(value)
                .map_err(|e| format!("failed to render passthrough_gpu device: {e}"))?;
            Ok(Some(device))
        }
        PcieDeviceKind::Ich9IntelHda => {
            let value = json!({
                "pcie": {
                    "bus": bus,
                    "device": address_device,
                    "function": address_function,
                    "type": "ich9_intel_hda"
                }
            });
            let device: Device = serde_json::from_value(value)
                .map_err(|e| format!("failed to render ich9_intel_hda device: {e}"))?;
            Ok(Some(device))
        }
        PcieDeviceKind::IvshmemPlain => {
            let resource = device
                .resource_id()
                .ok_or_else(|| "missing ivshmem_plain resource id".to_string())?;
            let value = json!({
                "pcie": {
                    "bus": bus,
                    "device": address_device,
                    "function": address_function,
                    "type": "ivshmem_plain",
                    "resource": resource
                }
            });
            let device: Device = serde_json::from_value(value)
                .map_err(|e| format!("failed to render ivshmem_plain device: {e}"))?;
            Ok(Some(device))
        }
    }
}

fn render_storage_device(
    bus_kind: &str,
    bus: u8,
    address: u8,
    storage: &StorageResource,
    kind: StorageDeviceKind,
    ctx: &mut RenderContext<'_>,
) -> Result<Option<Device>, String> {
    let resource = storage_resource_id(
        storage,
        ctx.resources,
        ctx.storage_resource_ids,
        ctx.network_resource_ids,
    );

    let device_type = storage_device_type(kind);

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

fn render_scsi_device(
    bus: u8,
    target: u8,
    lun: u8,
    storage: &StorageResource,
    kind: StorageDeviceKind,
    ctx: &mut RenderContext<'_>,
) -> Result<Option<Device>, String> {
    let resource = storage_resource_id(
        storage,
        ctx.resources,
        ctx.storage_resource_ids,
        ctx.network_resource_ids,
    );
    let device_type = storage_device_type(kind);

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

fn storage_device_type(kind: StorageDeviceKind) -> &'static str {
    match kind {
        StorageDeviceKind::Cdrom => "cdrom",
        StorageDeviceKind::Hdd | StorageDeviceKind::Ssd => "hdd",
    }
}

struct RenderContext<'a> {
    resources: &'a mut Vec<Resource>,
    storage_resource_ids: &'a mut BTreeMap<String, String>,
    network_resource_ids: &'a mut BTreeMap<String, String>,
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
            "host": {
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
            }
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

        assert!(rendered.host.resources.len() >= 4);
        assert!(rendered.virtual_machine.devices.len() >= 3);
        assert!(rendered.virtual_machine.tpm.is_some());
    }
}
