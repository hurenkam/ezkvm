//! SchemaBuilder stage: `RuntimeModel` → `ProxmoxConfigSchema`.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/
//! - doc/dev/architecture/design/config-format-ezkvm-proxmox-alignment-recommendations.md

use std::collections::BTreeMap;

use crate::{
    config_format::{
        proxmox::{
            schema::{
                ProxmoxCompoundValue, ProxmoxConfigSchema, ProxmoxOption, ProxmoxSection,
                ProxmoxValue,
            },
            storage_resolver::ProxmoxStorageConfig,
        },
        stages::SchemaBuilder,
    },
    runtime_model::RuntimeModel,
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

        let memory_mb = memory_megabytes(runtime.memory().qemu_args(runtime.cpu()))?;
        entries.insert("memory".to_string(), scalar(&memory_mb.to_string()));

        let (cpu_model, cores, sockets) = cpu_topology(runtime.cpu().qemu_args());
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
                    let args = device.qemu_args(&bus_id, address);
                    if let Some(value) = scsi_value_from_drive_args(&args, storage) {
                        entries.insert(format!("scsi{scsi_idx}"), value);
                        scsi_idx += 1;
                    }
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
                    let args = device.qemu_args(&bus_id, address);
                    if let Some(value) = net_value_from_pcie_args(&args) {
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

    let args = display.qemu_args();
    for window in args.windows(2) {
        if let [flag, value] = window {
            if flag == "-spice" {
                entries.insert("spice".to_string(), scalar(value));
                return;
            }
            if flag == "-vnc" && value != "none" {
                entries.insert("vnc".to_string(), scalar(value));
                return;
            }
        }
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

fn chipset_name(chipset: &crate::runtime_model::Chipset) -> &'static str {
    match chipset {
        crate::runtime_model::Chipset::Q35(_) => "q35",
        crate::runtime_model::Chipset::I440FX(_) => "i440fx",
    }
}

fn memory_megabytes(memory_args: Vec<String>) -> Result<u64, String> {
    for window in memory_args.windows(2) {
        if let [flag, value] = window
            && flag == "-m"
        {
            let stripped = value.trim_end_matches('M');
            return stripped
                .parse::<u64>()
                .map_err(|_| format!("failed to parse memory value '{value}'"));
        }
    }
    Err("memory size not found in qemu args".to_string())
}

fn cpu_topology(cpu_args: Vec<String>) -> (String, u8, u8) {
    let mut model = "host".to_string();
    let mut cores: u8 = 1;
    let mut sockets: u8 = 1;

    let mut iter = cpu_args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-cpu"
            && let Some(next) = iter.next()
        {
            model = next.clone();
        }
        if arg == "-smp"
            && let Some(next) = iter.next()
        {
            for token in next.split(',') {
                if let Some(v) = token.strip_prefix("cores=") {
                    cores = v.parse().unwrap_or(1);
                }
                if let Some(v) = token.strip_prefix("sockets=") {
                    sockets = v.parse().unwrap_or(1);
                }
            }
        }
    }

    (model, cores, sockets)
}

fn detect_vga(model: &RuntimeModel) -> Option<&'static str> {
    if let Some(d) = model.display() {
        for window in d.qemu_args().windows(2) {
            if let [flag, value] = window
                && flag == "-vga"
            {
                return match value.as_str() {
                    "virtio" => Some("virtio"),
                    "qxl" => Some("qxl"),
                    "std" => Some("std"),
                    "vmware" => Some("vmware"),
                    _ => Some("std"),
                };
            }
        }
    }
    None
}

fn scsi_value_from_drive_args(
    args: &[String],
    storage: &ProxmoxStorageConfig,
) -> Option<ProxmoxValue> {
    use crate::runtime_model::StorageResource;

    let drive_value = args
        .iter()
        .find(|a| a.contains("file=") && a.contains("if=none"))?;

    let file_path = drive_value
        .split(',')
        .find_map(|token| token.strip_prefix("file="))
        .map(str::to_string)?;
    let token = storage.resource_to_token(&if file_path.starts_with("/dev/") {
        StorageResource::BlockDevice {
            block_device: file_path,
        }
    } else {
        StorageResource::File { file: file_path }
    });

    let is_cdrom = drive_value.contains("media=cdrom");

    let options = if is_cdrom {
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

    Some(ProxmoxValue::Compound(ProxmoxCompoundValue {
        head: token,
        options,
    }))
}

fn net_value_from_pcie_args(args: &[String]) -> Option<ProxmoxValue> {
    let netdev_value = args
        .iter()
        .find(|a| a.starts_with("bridge,") || a.starts_with("tap,") || a.starts_with("user,"))?;

    let option = if let Some(bridge) = netdev_value.split(',').find_map(|t| t.strip_prefix("br=")) {
        ProxmoxOption::KeyValue {
            key: "bridge".to_string(),
            value: bridge.to_string(),
        }
    } else if let Some(tap) = netdev_value
        .split(',')
        .find_map(|t| t.strip_prefix("ifname="))
    {
        ProxmoxOption::KeyValue {
            key: "ifname".to_string(),
            value: tap.to_string(),
        }
    } else {
        return None;
    };

    Some(ProxmoxValue::Compound(ProxmoxCompoundValue {
        head: "virtio".to_string(),
        options: vec![option],
    }))
}

#[cfg(test)]
mod tests {
    use crate::config_format::proxmox::storage_resolver::ProxmoxStorageConfig;
    use crate::config_format::stages::SchemaBuilder;
    use crate::runtime_model::{
        BiosModel, BootModel, BusRegister, Chipset, Cpu, CpuModel, Memory, Q35Chipset,
        RuntimeModel, SeaBiosModel,
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
}
