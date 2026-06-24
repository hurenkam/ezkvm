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
    runtime_model::{BiosModel, Chipset, RuntimeModel, StorageResource},
};

/// Builds a `ProxmoxConfigSchema` from a `RuntimeModel`.
///
/// `storage_config` provides the storage token/path resolver required during rendering.
///
/// Field coverage per mapped section:
/// - `name`, `machine`, `memory`, `cpu`, `cores`, `sockets` — always written
/// - `bios`, `efidisk0` — written only when UEFI firmware is configured
/// - `scsiN` — one entry per SCSI-attached storage device, in bus/address order
/// - `netN` — one entry per PCIe-attached virtio network device, in bus/address order
///
/// TPM is not yet mapped.
pub struct ProxmoxSchemaBuilder {
    pub storage_config: ProxmoxStorageConfig,
}

impl SchemaBuilder for ProxmoxSchemaBuilder {
    type Schema = ProxmoxConfigSchema;

    fn build(&self, runtime: RuntimeModel) -> Result<ProxmoxConfigSchema, String> {
        let mut entries: BTreeMap<String, ProxmoxValue> = BTreeMap::new();

        entries.insert("name".to_string(), scalar(runtime.name()));

        entries.insert(
            "machine".to_string(),
            scalar(chipset_name(runtime.chipset())),
        );

        let memory_mb = memory_megabytes(runtime.memory().qemu_args())?;
        entries.insert("memory".to_string(), scalar(&memory_mb.to_string()));

        let (cpu_model, cores, sockets) = cpu_topology(runtime.cpu().qemu_args());
        entries.insert("cpu".to_string(), scalar(&cpu_model));
        entries.insert("cores".to_string(), scalar(&cores.to_string()));
        entries.insert("sockets".to_string(), scalar(&sockets.to_string()));

        match runtime.boot().bios() {
            BiosModel::SeaBios(_) => {}
            BiosModel::Uefi(uefi) => {
                entries.insert("bios".to_string(), scalar("ovmf"));
                entries.insert(
                    "efidisk0".to_string(),
                    scalar(&self.storage_config.resource_to_token(uefi.storage())),
                );
            }
        }

        map_scsi_devices(&runtime, &self.storage_config, &mut entries);
        map_net_devices(&runtime, &mut entries);

        Ok(ProxmoxConfigSchema {
            global: ProxmoxSection { entries },
            snapshots: BTreeMap::new(),
        })
    }
}

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

/// Constructs a scalar Proxmox value.
fn scalar(value: &str) -> ProxmoxValue {
    ProxmoxValue::Scalar {
        value: value.to_string(),
    }
}

/// Returns the canonical chipset name for a `Chipset` variant.
///
/// Uses direct enum matching rather than parsing qemu args, which avoids
/// relying on the textual qemu arg format and works for chipsets whose
/// `qemu_args()` implementation is incomplete.
fn chipset_name(chipset: &Chipset) -> &'static str {
    match chipset {
        Chipset::Q35(_) => "q35",
        Chipset::I440FX(_) => "i440fx",
    }
}

/// Extracts the memory size in megabytes from the memory's qemu args.
///
/// The memory qemu arg renders as `["-m", "4096M"]`.
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

/// Extracts the CPU model, core count, and socket count from the cpu's qemu args.
///
/// The cpu qemu args render as `["-cpu", "host", "-smp", "8,sockets=1,cores=8,threads=1"]`.
/// Defaults to model `host`, 1 core, 1 socket when parsing fails.
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

/// Extracts a Proxmox disk value from SCSI drive qemu args.
///
/// The drive arg renders as:
/// `id=drive-scsiN,file=<path>,if=none,format=raw,...[,media=cdrom,readonly=on]`
///
/// Produces a compound value with `cache=writeback` for block storage or
/// `media=cdrom` for optical media.
fn scsi_value_from_drive_args(
    args: &[String],
    storage: &ProxmoxStorageConfig,
) -> Option<ProxmoxValue> {
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

/// Extracts a Proxmox network value from PCIe device qemu args.
///
/// Returns `None` for non-network PCIe devices (e.g. PvScsi controller).
/// The netdev arg renders as `bridge,id=<id>,br=<bridge>` or `tap,id=<id>,ifname=<tap>`.
///
/// Produces a compound value with head `virtio` and a `bridge=` or `ifname=` option.
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
    use crate::{
        config_format::{
            proxmox::{
                ProxmoxConfigSchema,
                schema::{ProxmoxCompoundValue, ProxmoxValue},
                storage_resolver::ProxmoxStorageConfig,
            },
            stages::SchemaBuilder,
        },
        runtime_model::RuntimeModelBuilder,
    };

    use super::ProxmoxSchemaBuilder;

    fn storage_cfg() -> ProxmoxStorageConfig {
        ProxmoxStorageConfig::parse(
            r#"
dir: local
    path /var/lib/vz

lvmthin: vm-pool
    vgname vm

lvmthin: vm1-pool
    vgname vm1
"#,
        )
        .expect("storage cfg should parse")
    }

    fn parse_and_build(conf_text: &str) -> crate::runtime_model::RuntimeModel {
        let schema = ProxmoxConfigSchema::parse(conf_text).expect("config should parse");
        let storage = storage_cfg();
        RuntimeModelBuilder::build_from_proxmox_config(&schema, &storage)
            .expect("runtime model should build")
    }

    fn scalar_value(schema: &ProxmoxConfigSchema, key: &str) -> String {
        match schema.global.entries.get(key) {
            Some(ProxmoxValue::Scalar { value }) => value.clone(),
            other => panic!("expected scalar for '{key}', got {other:?}"),
        }
    }

    fn compound_value(schema: &ProxmoxConfigSchema, key: &str) -> ProxmoxCompoundValue {
        match schema.global.entries.get(key) {
            Some(ProxmoxValue::Compound(compound)) => compound.clone(),
            other => panic!("expected compound for '{key}', got {other:?}"),
        }
    }

    #[test]
    fn maps_core_fields_from_minimal_config() {
        let model = parse_and_build(
            r#"
name: test-vm
machine: q35
memory: 4096
cpu: host
cores: 4
sockets: 2
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");

        assert_eq!(scalar_value(&schema, "name"), "test-vm");
        assert_eq!(scalar_value(&schema, "machine"), "q35");
        assert_eq!(scalar_value(&schema, "memory"), "4096");
        assert_eq!(scalar_value(&schema, "cpu"), "host");
        assert_eq!(scalar_value(&schema, "cores"), "4");
        assert_eq!(scalar_value(&schema, "sockets"), "2");
    }

    #[test]
    fn maps_i440fx_chipset_correctly() {
        let model = parse_and_build(
            r#"
name: legacy-vm
machine: i440fx
memory: 2048
cpu: host
cores: 2
sockets: 1
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");
        assert_eq!(scalar_value(&schema, "machine"), "i440fx");
    }

    #[test]
    fn maps_ovmf_bios_and_efidisk() {
        let model = parse_and_build(
            r#"
name: uefi-vm
machine: q35
memory: 8192
cpu: host
cores: 2
sockets: 1
bios: ovmf
efidisk0: /var/lib/pve/efivars.fd
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");

        assert_eq!(scalar_value(&schema, "bios"), "ovmf");
        assert!(schema.global.entries.contains_key("efidisk0"));
    }

    #[test]
    fn seabios_produces_no_bios_field() {
        let model = parse_and_build(
            r#"
name: seabios-vm
machine: q35
memory: 2048
cpu: host
cores: 1
sockets: 1
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");

        assert!(!schema.global.entries.contains_key("bios"));
        assert!(!schema.global.entries.contains_key("efidisk0"));
    }

    #[test]
    fn maps_scsi_block_device() {
        let model = parse_and_build(
            r#"
name: scsi-vm
machine: q35
memory: 4096
cpu: host
cores: 2
sockets: 1
scsi0: vm-pool:vm-100-disk-0,cache=writeback,size=32G
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");

        assert!(
            schema.global.entries.contains_key("scsi0"),
            "scsi0 should be present"
        );
        let disk = compound_value(&schema, "scsi0");
        assert_eq!(disk.head, "vm-pool:vm-100-disk-0");
    }

    #[test]
    fn maps_bridge_network_device() {
        let model = parse_and_build(
            r#"
name: net-vm
machine: q35
memory: 4096
cpu: host
cores: 2
sockets: 1
net0: virtio=DE:AD:BE:EF:00:01,bridge=vmbr0
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");

        assert!(
            schema.global.entries.contains_key("net0"),
            "net0 should be present"
        );
        let net = compound_value(&schema, "net0");
        assert_eq!(net.head, "virtio");
        assert!(net
            .options
            .iter()
            .any(|o| matches!(o, crate::config_format::proxmox::schema::ProxmoxOption::KeyValue { key, value }
                if key == "bridge" && value == "vmbr0")));
    }

    #[test]
    fn produced_schema_parses_back_without_error() {
        let model = parse_and_build(
            r#"
name: roundtrip-vm
machine: q35
memory: 8192
cpu: host
cores: 4
sockets: 1
scsi0: vm-pool:vm-200-disk-1,cache=writeback,size=64G
net0: virtio=AA:BB:CC:DD:EE:FF,bridge=vmbr1
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");
        let rendered = schema.render();
        ProxmoxConfigSchema::parse(&rendered)
            .expect("re-parsed rendered schema should be valid Proxmox conf");
    }

    #[test]
    fn multiple_scsi_devices_are_numbered_sequentially() {
        let model = parse_and_build(
            r#"
name: multi-disk-vm
machine: q35
memory: 4096
cpu: host
cores: 2
sockets: 1
scsi0: vm-pool:vm-100-disk-0,cache=writeback,size=32G
scsi1: vm-pool:vm-100-disk-1,cache=writeback,size=64G
"#,
        );

        let schema = ProxmoxSchemaBuilder {
            storage_config: storage_cfg(),
        }
        .build(model)
        .expect("mapping should succeed");

        assert!(
            schema.global.entries.contains_key("scsi0"),
            "scsi0 should be present"
        );
        assert!(
            schema.global.entries.contains_key("scsi1"),
            "scsi1 should be present"
        );
    }
}
