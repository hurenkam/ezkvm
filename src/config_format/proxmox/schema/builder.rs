//! SchemaBuilder stage: `RuntimeModel` → `ProxmoxConfigSchema`.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/
//! - doc/dev/architecture/design/config-format-ezkvm-proxmox-alignment-recommendations.md

use std::collections::BTreeMap;

use crate::{
    config_format::{
        SchemaBuilder,
        proxmox::{
            schema::{
                ProxmoxCompoundValue, ProxmoxConfigSchema, ProxmoxOption, ProxmoxSection,
                ProxmoxValue,
            },
            storage_resolver::ProxmoxStorageConfig,
        },
    },
    runtime_model::{
        Chipset, Display, NetworkResource, QxlGpuController, RuntimeModel, VirtioGpuController,
        VirtioNetController,
    },
};

/// Builds a `ProxmoxConfigSchema` from a `RuntimeModel`.
///
/// `storage_config` provides the storage token/path resolver required during rendering.
///
/// Field coverage per mapped section:
/// - `name`, `machine`, `memory`, `cpu`, `cores`, `sockets` - always written
/// - `bios`, `efidisk0` - written only when UEFI firmware is configured
/// - `scsiN` - one entry per SCSI-attached storage device, in bus/address order
/// - `netN` - one entry per PCIe-attached virtio network device, in bus/address order
/// - `agent` - written when guest-agent is configured in runtime model
/// - `vga` - written when a mapped GPU device exists in runtime model
/// - `spice`/`vnc` - written when display frontend is representable in Proxmox fields
///
/// TPM is not yet mapped.
#[derive(Default)]
pub struct ProxmoxSchemaBuilder {
    pub storage_config: Option<ProxmoxStorageConfig>,
    pub source_file: Option<std::path::PathBuf>,
    pub runtime_model: Option<RuntimeModel>,
}

impl ProxmoxSchemaBuilder {
    pub fn with_storage_config(self, storage_config: ProxmoxStorageConfig) -> Self {
        Self {
            storage_config: Some(storage_config),
            ..self
        }
    }

    pub fn with_source_file(self, source_file: std::path::PathBuf) -> Self {
        Self {
            source_file: Some(source_file),
            ..self
        }
    }

    fn build_from_runtime(
        &self,
        runtime: &RuntimeModel,
        storage_config: &ProxmoxStorageConfig,
    ) -> Result<ProxmoxConfigSchema, String> {
        let mut entries: BTreeMap<String, ProxmoxValue> = BTreeMap::new();

        entries.insert("name".to_string(), scalar(runtime.name()));

        entries.insert(
            "machine".to_string(),
            scalar(chipset_name(runtime.chipset())),
        );

        let memory_mb = (runtime.memory().size() / 1024 / 1024) as u64;
        entries.insert("memory".to_string(), scalar(&memory_mb.to_string()));

        let (cpu_model, cores, sockets) = cpu_topology(runtime);
        entries.insert("cpu".to_string(), scalar(&cpu_model));
        entries.insert("cores".to_string(), scalar(&cores.to_string()));
        entries.insert("sockets".to_string(), scalar(&sockets.to_string()));

        match runtime.boot().bios() {
            crate::runtime_model::BiosModel::SeaBios(_) => {}
            crate::runtime_model::BiosModel::Uefi(uefi) => {
                entries.insert("bios".to_string(), scalar("ovmf"));
                entries.insert(
                    "efidisk0".to_string(),
                    scalar(&storage_config.resource_to_token(uefi.storage())),
                );
            }
        }

        map_scsi_devices(runtime, storage_config, &mut entries);
        map_net_devices(runtime, &mut entries);
        map_guest_agent(runtime, &mut entries);
        map_vga(runtime, &mut entries);
        map_display(runtime, &mut entries);

        Ok(ProxmoxConfigSchema {
            global: ProxmoxSection { entries },
            snapshots: BTreeMap::new(),
        })
    }

    fn build_from_source_file(
        &self,
        source_file: std::path::PathBuf,
        _storage_config: &ProxmoxStorageConfig,
    ) -> Result<ProxmoxConfigSchema, String> {
        let source_text = std::fs::read_to_string(&source_file)
            .map_err(|e| format!("{}: {}", source_file.display(), e))?;

        ProxmoxConfigSchema::parse(&source_text).map_err(|e| e.to_string())
    }
}

impl SchemaBuilder for ProxmoxSchemaBuilder {
    type Schema = ProxmoxConfigSchema;

    fn with_runtime(self, runtime: RuntimeModel) -> Self {
        Self {
            runtime_model: Some(runtime),
            ..self
        }
    }

    fn build(self) -> Result<ProxmoxConfigSchema, String> {
        let storage_config = self.storage_config.as_ref().ok_or_else(|| {
            "ProxmoxSchemaBuilder requires a storage_config to map SCSI devices".to_string()
        })?;

        match &self.runtime_model {
            Some(runtime) => self.build_from_runtime(runtime, storage_config),
            None => match &self.source_file {
                Some(source_file) => {
                    self.build_from_source_file(source_file.clone(), storage_config)
                }
                None => Err(
                    "ProxmoxSchemaBuilder requires either a runtime_model or a source_file to build schema"
                        .to_string(),
                ),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Device mapping helpers
// ---------------------------------------------------------------------------

/// Maps all SCSI storage devices from the runtime model into Proxmox config entries.
fn map_scsi_devices(
    model: &RuntimeModel,
    storage: &ProxmoxStorageConfig,
    entries: &mut BTreeMap<String, ProxmoxValue>,
) {
    let mut scsi_idx: usize = 0;

    let mut scsi_bus_ids: Vec<u8> = model.busses().scsi_busses().keys().copied().collect();
    scsi_bus_ids.sort_unstable();

    for bus_id in scsi_bus_ids {
        if let Some(controller) = model.busses().scsi_busses().get(&bus_id) {
            let devices = controller.devices();
            let mut addresses: Vec<_> = devices.keys().cloned().collect();
            addresses.sort_by_key(|a| (a.target, a.lun));

            for address in addresses {
                if let Some(device) = devices.get(&address) {
                    let token = storage.resource_to_token(device.storage_resource());
                    let options = if device.storage_kind()
                        == crate::runtime_model::StorageDeviceKind::Cdrom
                    {
                        vec![ProxmoxOption::KeyValue {
                            key: "media".to_string(),
                            value: "cdrom".to_string(),
                        }]
                    } else {
                        vec![ProxmoxOption::KeyValue {
                            key: "cache".to_string(),
                            value: "writeback".to_string(),
                        }]
                    };
                    entries.insert(
                        format!("scsi{scsi_idx}"),
                        ProxmoxValue::Compound(ProxmoxCompoundValue {
                            head: token,
                            options,
                        }),
                    );
                    scsi_idx += 1;
                }
            }
        }
    }
}

/// Maps all PCIe network devices from the runtime model into Proxmox config entries.
fn map_net_devices(model: &RuntimeModel, entries: &mut BTreeMap<String, ProxmoxValue>) {
    let mut net_idx: usize = 0;

    let mut pcie_bus_ids: Vec<u8> = model.busses().pcie_busses().keys().copied().collect();
    pcie_bus_ids.sort_unstable();

    for bus_id in pcie_bus_ids {
        if let Some(controller) = model.busses().pcie_busses().get(&bus_id) {
            let devices = controller.devices();
            let mut addresses: Vec<_> = devices.keys().cloned().collect();
            addresses.sort_by_key(|a| (a.device(), a.function()));

            for address in addresses {
                if let Some(device) = devices.get(&address) {
                    if let Some(value) = net_value_from_pcie_device(device.as_ref()) {
                        entries.insert(format!("net{net_idx}"), value);
                        net_idx += 1;
                    }
                }
            }
        }
    }
}

/// Maps guest agent configuration from the runtime model to `agent` entry.
fn map_guest_agent(model: &RuntimeModel, entries: &mut BTreeMap<String, ProxmoxValue>) {
    let Some(agent) = model.guest_agent() else {
        return;
    };
    let value = if agent.config().enabled { "1" } else { "0" };
    entries.insert("agent".to_string(), scalar(value));
}

/// Maps GPU/video card configuration from the runtime model to `vga` entry.
fn map_vga(model: &RuntimeModel, entries: &mut BTreeMap<String, ProxmoxValue>) {
    if let Some(vga) = detect_vga(model) {
        entries.insert("vga".to_string(), scalar(vga));
    }
}

/// Maps display configuration (SPICE/VNC) from the runtime model.
fn map_display(model: &RuntimeModel, entries: &mut BTreeMap<String, ProxmoxValue>) {
    let Some(display) = model.display() else {
        return;
    };

    match display.config() {
        Display::Spice { spice } => {
            let listen = if spice.listen().is_empty() {
                "0.0.0.0".to_string()
            } else {
                spice.listen().clone()
            };
            let mut value = format!("port={},addr={listen}", spice.port());
            if *spice.disable_ticketing() {
                value.push_str(",disable-ticketing=on");
            }
            entries.insert("spice".to_string(), scalar(&value));
        }
        Display::Vnc { vnc } => {
            let listen = if vnc.listen().is_empty() {
                "0.0.0.0"
            } else {
                vnc.listen()
            };
            entries.insert(
                "vnc".to_string(),
                scalar(&format!("{listen}:{}", vnc.port())),
            );
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Schema helpers
// ---------------------------------------------------------------------------

fn scalar(value: &str) -> ProxmoxValue {
    ProxmoxValue::Scalar {
        value: value.to_string(),
    }
}

fn chipset_name(chipset: &Chipset) -> &'static str {
    match chipset {
        Chipset::Q35(_) => "q35",
        Chipset::I440FX(_) => "i440fx",
    }
}
fn cpu_topology(runtime: &RuntimeModel) -> (String, u8, u8) {
    let model = match runtime.cpu().model() {
        crate::runtime_model::CpuModel::Host => "host".to_string(),
    };

    (
        model,
        runtime.cpu().cores().max(1),
        runtime.cpu().sockets().max(1),
    )
}

fn detect_vga(model: &RuntimeModel) -> Option<&'static str> {
    let mut pci_bus_ids: Vec<_> = model.busses().pci_busses().keys().copied().collect();
    pci_bus_ids.sort_unstable();
    for bus_id in pci_bus_ids {
        if let Some(controller) = model.busses().pci_busses().get(&bus_id) {
            for device in controller.devices().values() {
                if device.as_any().is::<QxlGpuController>() {
                    return Some("qxl");
                }
            }
        }
    }

    let mut pcie_bus_ids: Vec<_> = model.busses().pcie_busses().keys().copied().collect();
    pcie_bus_ids.sort_unstable();
    for bus_id in pcie_bus_ids {
        if let Some(controller) = model.busses().pcie_busses().get(&bus_id) {
            for device in controller.devices().values() {
                if device.as_any().is::<VirtioGpuController>() {
                    return Some("virtio");
                }
            }
        }
    }
    None
}
fn net_value_from_pcie_device(
    device: &dyn crate::runtime_model::PcieDeviceApi,
) -> Option<ProxmoxValue> {
    let net = device.as_any().downcast_ref::<VirtioNetController>()?;
    let mut options = Vec::new();
    let mut head = "virtio".to_string();

    match net.resource() {
        Some(NetworkResource::Bridge { bridge }) => {
            options.push(ProxmoxOption::KeyValue {
                key: "bridge".to_string(),
                value: bridge.clone(),
            });
        }
        Some(NetworkResource::Tap { tap }) => {
            options.push(ProxmoxOption::KeyValue {
                key: "ifname".to_string(),
                value: tap.clone(),
            });
        }
        None => return None,
    }

    if let Some(vhost) = net.vhost() {
        options.push(ProxmoxOption::KeyValue {
            key: "vhost".to_string(),
            value: if vhost { "on" } else { "off" }.to_string(),
        });
    }

    if let Some(mac) = net.mac_address() {
        head = format!("virtio={mac}");
    }

    if let Some(rx_queue_size) = net.rx_queue_size() {
        options.push(ProxmoxOption::KeyValue {
            key: "rx_queue_size".to_string(),
            value: rx_queue_size.to_string(),
        });
    }

    if let Some(tx_queue_size) = net.tx_queue_size() {
        options.push(ProxmoxOption::KeyValue {
            key: "tx_queue_size".to_string(),
            value: tx_queue_size.to_string(),
        });
    }

    Some(ProxmoxValue::Compound(ProxmoxCompoundValue {
        head,
        options,
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::config_format::SchemaBuilder;
    use crate::config_format::proxmox::storage_resolver::ProxmoxStorageConfig;
    use crate::runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Memory, NetworkResource,
        PcieAddress, PcieDeviceApi, Q35Chipset, RuntimeModel, SeaBiosModel, VirtioNetController,
    };

    fn storage_config() -> ProxmoxStorageConfig {
        ProxmoxStorageConfig::parse(
            r#"
dir: local
    path /var/lib/vz
"#,
        )
        .expect("storage config should parse")
    }

    #[test]
    fn builds_proxmox_schema_from_runtime_model() {
        let mut bus_register = BusRegister::new();
        let model = RuntimeModel::new(
            "test-vm".to_string(),
            Cpu::new(CpuModel::Host, 4, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            None,
            None,
            None,
            bus_register,
        );

        let result = super::ProxmoxSchemaBuilder::default()
            .with_storage_config(storage_config())
            .with_runtime(model)
            .build();

        assert!(result.is_ok(), "schema build should succeed: {:?}", result);
        let schema = result.unwrap();
        assert_eq!(
            schema.global.entries.get("name").and_then(|v| {
                if let crate::config_format::proxmox::schema::ProxmoxValue::Scalar { value } = v {
                    Some(value.as_str())
                } else {
                    None
                }
            }),
            Some("test-vm")
        );
    }

    #[test]
    fn exports_network_mac_queue_and_vhost_to_proxmox_net_entry() {
        let mut bus_register = BusRegister::new();
        let model = RuntimeModel::new(
            "test-net-vm".to_string(),
            Cpu::new(CpuModel::Host, 4, 1, 1),
            Memory::megabytes(2048),
            Chipset::Q35(Q35Chipset::new(&mut bus_register)),
            BootModel::new(BiosModel::SeaBios(SeaBiosModel::default())),
            None,
            None,
            None,
            None,
            None,
            None,
            bus_register,
        );

        let net: Arc<dyn PcieDeviceApi> = Arc::new(VirtioNetController::new(
            Some(NetworkResource::Tap {
                tap: "tap301i0".to_string(),
            }),
            Some("AA:BB:CC:DD:EE:FF".to_string()),
            Some(1024),
            Some(256),
            Some(true),
        ));
        model
            .register_pcie_device(0, net, Some(PcieAddress::new(0x10, 0)))
            .expect("pcie network registration should succeed");

        let schema = super::ProxmoxSchemaBuilder::default()
            .with_storage_config(storage_config())
            .with_runtime(model)
            .build()
            .expect("schema build should succeed");

        let Some(crate::config_format::proxmox::schema::ProxmoxValue::Compound(net0)) =
            schema.global.entries.get("net0")
        else {
            panic!("net0 should be exported as a compound value");
        };

        assert_eq!(net0.head, "virtio=AA:BB:CC:DD:EE:FF");
        assert!(net0.options.iter().any(|option| {
            matches!(
                option,
                crate::config_format::proxmox::schema::ProxmoxOption::KeyValue { key, value }
                if key == "ifname" && value == "tap301i0"
            )
        }));
        assert!(net0.options.iter().any(|option| {
            matches!(
                option,
                crate::config_format::proxmox::schema::ProxmoxOption::KeyValue { key, value }
                if key == "vhost" && value == "on"
            )
        }));
        assert!(net0.options.iter().any(|option| {
            matches!(
                option,
                crate::config_format::proxmox::schema::ProxmoxOption::KeyValue { key, value }
                if key == "rx_queue_size" && value == "1024"
            )
        }));
        assert!(net0.options.iter().any(|option| {
            matches!(
                option,
                crate::config_format::proxmox::schema::ProxmoxOption::KeyValue { key, value }
                if key == "tx_queue_size" && value == "256"
            )
        }));
    }
}
