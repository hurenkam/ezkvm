//! RuntimeBuilder stage: `ProxmoxConfigSchema` → `RuntimeModel`.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/domain-knowledge/proxmox/

use crate::{
    config_format::{
        RuntimeBuilder,
        proxmox::{schema::ProxmoxConfigSchema, storage_resolver::ProxmoxStorageConfig},
    },
    runtime_model::RuntimeModel,
};

/// Builds a `RuntimeModel` from a `ProxmoxConfigSchema`.
///
/// `storage_config` provides the storage token/path resolver required during assembly.
#[derive(Default)]
pub struct ProxmoxRuntimeBuilder {
    pub storage_config: Option<ProxmoxStorageConfig>,
    pub schema: Option<ProxmoxConfigSchema>,
}

impl ProxmoxRuntimeBuilder {
    pub fn with_storage_config(self, storage_config: ProxmoxStorageConfig) -> Self {
        Self {
            storage_config: Some(storage_config),
            ..self
        }
    }
}

impl RuntimeBuilder for ProxmoxRuntimeBuilder {
    type Schema = ProxmoxConfigSchema;

    fn build(self) -> Result<RuntimeModel, String> {
        let schema = self.schema.as_ref().ok_or_else(|| {
            "ProxmoxRuntimeBuilder requires a schema to build runtime model".to_string()
        })?;
        let storage_config = self.storage_config.as_ref().ok_or_else(|| {
            "ProxmoxRuntimeBuilder requires a storage_config to build runtime model".to_string()
        })?;
        build_proxmox_runtime(schema, storage_config)
    }

    fn with_schema(self, schema: Self::Schema) -> Self {
        Self {
            schema: Some(schema),
            ..self
        }
    }
}

// ---------------------------------------------------------------------------
// Core build function
// ---------------------------------------------------------------------------

fn build_proxmox_runtime(
    config: &ProxmoxConfigSchema,
    storage_config: &ProxmoxStorageConfig,
) -> Result<RuntimeModel, String> {
    use parse_helpers::*;

    let global = &config.global.entries;

    let parsed = field_parsing::parse_global_config(global)?;

    let mut bus_register = crate::runtime_model::BusRegister::new();
    let chipset = match parse_machine_chipset(&parsed.machine) {
        Some(MachineChipset::Q35) => crate::runtime_model::Chipset::Q35(
            crate::runtime_model::Q35Chipset::new(&mut bus_register),
        ),
        Some(MachineChipset::I440fx) => crate::runtime_model::Chipset::I440FX(
            crate::runtime_model::I440fxChipset::new(&bus_register),
        ),
        None => return Err(format!("Unsupported chipset: {}", parsed.machine)),
    };

    let cpu =
        crate::runtime_model::Cpu::new(parsed.cpu_config.model, parsed.cores, 1, parsed.sockets)
            .with_features(
                parsed.cpu_config.enabled_features,
                parsed.cpu_config.disabled_features,
            );

    let memory = crate::runtime_model::Memory::megabytes(parsed.memory_mb as usize)
        .with_hugepages_kb(parsed.hugepages_kb)
        .with_numa_enabled(parsed.numa_enabled);

    let collected = resource_collection::collect_resources(global, storage_config)?;
    let storage_resources = collected.storage_resources;
    let scsi_disks = collected.scsi_disks;
    let ide_disks = collected.ide_disks;
    let net_devices = collected.net_devices;
    let hostpci_devices = collected.hostpci_devices;
    let usb_resources = collected.usb_resources;
    let usb_devices = collected.usb_devices;

    let bios_model = match extract_scalar_string(global, "bios") {
        Ok(bios_type) if bios_type.as_str() == "ovmf" => storage_resources
            .get("efidisk0")
            .cloned()
            .map(crate::runtime_model::UefiModel::new)
            .map(crate::runtime_model::BiosModel::Uefi)
            .unwrap_or_else(|| {
                crate::runtime_model::BiosModel::SeaBios(
                    crate::runtime_model::SeaBiosModel::default(),
                )
            }),
        _ => {
            crate::runtime_model::BiosModel::SeaBios(crate::runtime_model::SeaBiosModel::default())
        }
    };

    let boot = crate::runtime_model::BootModel::new(bios_model);

    let tpm = match storage_resources.get("tpmstate0") {
        Some(_) => Some(crate::runtime_model::TpmModelBuilder::build(
            crate::runtime_model::Tpm::Emulated {
                swtpm: serde_json::from_value(serde_json::json!({
                    "version": 2.0,
                    "resource": "tpmstate0"
                }))
                .map_err(|e| format!("failed to build TPM metadata: {e}"))?,
            },
            &storage_resources,
        )?),
        None => None,
    };

    device_registration::register_scsi_devices(&mut bus_register, &scsi_disks)?;
    device_registration::register_ide_devices(&bus_register, &ide_disks)?;
    device_registration::register_network_devices(&bus_register, &net_devices)?;
    device_registration::register_hostpci_devices(&bus_register, &hostpci_devices)?;
    device_registration::register_usb_devices(
        &bus_register,
        &usb_devices,
        &usb_resources,
        parsed.tablet_enabled,
    )?;
    device_registration::register_gpu_device(&bus_register, parsed.gpu)?;

    Ok(crate::runtime_model::RuntimeModel::new(
        parsed.name,
        cpu,
        memory,
        chipset,
        boot,
        parsed.smbios_uuid,
        parsed.vmgenid,
        tpm,
        parsed
            .display
            .map(crate::runtime_model::DisplayModelBuilder::build),
        None,
        parsed
            .guest_agent
            .map(crate::runtime_model::GuestAgentModelBuilder::build),
        bus_register,
    )
    .with_lifecycle_config(parsed.lifecycle_config)
    .with_balloon_config(parsed.balloon_config)
    .with_serial_config(parsed.serial_config))
}

// ---------------------------------------------------------------------------
// Private helper modules
// ---------------------------------------------------------------------------

mod device_registration {
    use std::sync::Arc;

    use crate::runtime_model::{
        BusRegister, BusRegistrationApi, Cdrom, Hdd, IdeAddress, PciDeviceApi, PciDeviceType,
        PcieAddress, PcieDeviceApi, PcieDeviceType, PvScsiController, ScsiAddress,
        ScsiControllerApi, Ssd, StorageDeviceOptions, StorageResource, UsbAddress,
        UsbDeviceBuilder, UsbDeviceResource, UsbDeviceType, VirtioNetController,
    };

    use super::parse_helpers::{ProxmoxIdeKind, ProxmoxNetDevice, ProxmoxScsiKind};

    pub(super) fn register_scsi_devices(
        bus_register: &mut BusRegister,
        scsi_disks: &[(
            usize,
            StorageResource,
            ProxmoxScsiKind,
            StorageDeviceOptions,
        )],
    ) -> Result<(), String> {
        if scsi_disks.is_empty() {
            return Ok(());
        }

        let scsi_controller = Arc::new(PvScsiController::default());
        let pcie_addr = PcieAddress::new(0, 0);

        match bus_register.pcie_busses().get(&0) {
            Some(root) => {
                let controller_as_pcie: Arc<dyn PcieDeviceApi> = scsi_controller.clone();
                root.register_pcie_device(controller_as_pcie, Some(pcie_addr))?;
            }
            None => return Err("PCIe root bus with id 0 does not exist".to_string()),
        }

        let _scsi_bus_id = bus_register.register_scsi_bus(scsi_controller.clone())?;

        for (index, storage, kind, options) in scsi_disks {
            let device = match kind {
                ProxmoxScsiKind::Cdrom => {
                    Arc::new(Cdrom::with_options(storage.clone(), options.clone())) as Arc<_>
                }
                ProxmoxScsiKind::Ssd => {
                    Arc::new(Ssd::with_options(storage.clone(), options.clone())) as Arc<_>
                }
                ProxmoxScsiKind::Hdd => {
                    Arc::new(Hdd::with_options(storage.clone(), options.clone())) as Arc<_>
                }
            };
            let address = ScsiAddress::new(0, *index as u8);
            scsi_controller.register_scsi_device(device, Some(address))?;
        }

        Ok(())
    }

    pub(super) fn register_network_devices(
        bus_register: &BusRegister,
        net_devices: &[(usize, ProxmoxNetDevice)],
    ) -> Result<(), String> {
        for (index, config) in net_devices {
            let pcie_device: Arc<dyn PcieDeviceApi> = Arc::new(VirtioNetController::new(
                Some(config.resource.clone()),
                config.mac_address.clone(),
                config.rx_queue_size,
                config.tx_queue_size,
                config.vhost,
            ));
            let preferred = Some(PcieAddress::new(0x10 + *index as u8, 0));
            match bus_register.pcie_busses().get(&0) {
                Some(root) => root.register_pcie_device(pcie_device, preferred)?,
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            }
        }

        Ok(())
    }

    pub(super) fn register_ide_devices(
        bus_register: &BusRegister,
        ide_disks: &[(usize, StorageResource, ProxmoxIdeKind, StorageDeviceOptions)],
    ) -> Result<(), String> {
        if ide_disks.is_empty() {
            return Ok(());
        }

        let Some(root) = bus_register.ide_busses().get(&0) else {
            return Err("IDE root bus with id 0 does not exist".to_string());
        };

        for (index, storage, kind, options) in ide_disks {
            let device = match kind {
                ProxmoxIdeKind::Cdrom => {
                    Arc::new(Cdrom::with_options(storage.clone(), options.clone())) as Arc<_>
                }
                ProxmoxIdeKind::Ssd => {
                    Arc::new(Ssd::with_options(storage.clone(), options.clone())) as Arc<_>
                }
                ProxmoxIdeKind::Hdd => {
                    Arc::new(Hdd::with_options(storage.clone(), options.clone())) as Arc<_>
                }
            };

            // Q35 IDE secondary channel maps to Proxmox ide2 (unit 0) and ide3 (unit 1).
            let unit = index.saturating_sub(2) as u8;
            root.register_ide_device(device, Some(IdeAddress::new(unit)))?;
        }

        Ok(())
    }

    pub(super) fn register_hostpci_devices(
        bus_register: &BusRegister,
        hostpci_devices: &[(usize, PcieDeviceType)],
    ) -> Result<(), String> {
        for (_index, passthrough) in hostpci_devices {
            match bus_register.pcie_busses().get(&0) {
                Some(root) => {
                    let device: Arc<dyn PcieDeviceApi> = passthrough.into();
                    root.register_pcie_device(device, None)?;
                }
                None => return Err("PCIe root bus with id 0 does not exist".to_string()),
            }
        }

        Ok(())
    }

    pub(super) fn register_usb_devices(
        bus_register: &BusRegister,
        usb_devices: &[(usize, String)],
        usb_resources: &std::collections::HashMap<String, UsbDeviceResource>,
        tablet_enabled: bool,
    ) -> Result<(), String> {
        if !tablet_enabled && usb_devices.is_empty() {
            return Ok(());
        }

        let Some(root) = bus_register.usb_busses().get(&0) else {
            return Err("USB root bus with id 0 does not exist".to_string());
        };

        if tablet_enabled {
            let tablet = UsbDeviceBuilder::build(&UsbDeviceType::Tablet, usb_resources)?;
            root.register_usb_device(tablet, Some(UsbAddress::new("tablet".to_string())))?;
        }

        for (index, resource_id) in usb_devices {
            let device = UsbDeviceBuilder::build(
                &UsbDeviceType::HostPassthrough {
                    resource: resource_id.clone(),
                },
                usb_resources,
            )?;
            root.register_usb_device(device, Some(UsbAddress::new((index + 1).to_string())))?;
        }

        Ok(())
    }

    pub(super) fn register_gpu_device(
        bus_register: &BusRegister,
        gpu: Option<super::parse_helpers::ProxmoxGpu>,
    ) -> Result<(), String> {
        match gpu {
            Some(super::parse_helpers::ProxmoxGpu::Standard) => {
                match bus_register.pcie_busses().get(&0) {
                    Some(root) => {
                        let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::StandardGpu).into();
                        root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))?;
                    }
                    None => return Err("PCIe root bus with id 0 does not exist".to_string()),
                }
            }
            Some(super::parse_helpers::ProxmoxGpu::Virtio) => {
                match bus_register.pcie_busses().get(&0) {
                    Some(root) => {
                        let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::VirtioGpu).into();
                        root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))?;
                    }
                    None => return Err("PCIe root bus with id 0 does not exist".to_string()),
                }
            }
            Some(super::parse_helpers::ProxmoxGpu::Qxl) => {
                if let Some(root) = bus_register.pci_busses().get(&0) {
                    let device: Arc<dyn PciDeviceApi> = (&PciDeviceType::QxlGpu).into();
                    root.register_pci_device(device, None)?;
                } else if let Some(root) = bus_register.pcie_busses().get(&0) {
                    let device: Arc<dyn PcieDeviceApi> = (&PcieDeviceType::StandardGpu).into();
                    root.register_pcie_device(device, Some(PcieAddress::new(1, 0)))?;
                } else {
                    return Err("Neither PCI nor PCIe root bus exists".to_string());
                }
            }
            Some(super::parse_helpers::ProxmoxGpu::Headless) | None => {}
        }

        Ok(())
    }
}

mod field_parsing {
    use crate::{
        config_format::proxmox::schema::{ProxmoxOption, ProxmoxValue},
        runtime_model::{
            BalloonConfig, CpuModel, Display, GuestAgent, LifecycleConfig, SerialConfig,
        },
    };

    use super::parse_helpers::{
        ProxmoxGpu, extract_machine_token, extract_scalar_string, extract_scalar_u8,
        extract_scalar_u64, parse_balloon_config, parse_cpu_config, parse_display, parse_gpu,
        parse_guest_agent, parse_hugepages_kb, parse_lifecycle_config, parse_numa_enabled,
        parse_serial_config, parse_smbios_uuid, parse_tablet_enabled, parse_vmgenid,
    };

    pub(super) struct ParsedCpuConfig {
        pub model: CpuModel,
        pub enabled_features: Vec<String>,
        pub disabled_features: Vec<String>,
    }

    pub(super) struct ParsedGlobalConfig {
        pub name: String,
        pub memory_mb: u64,
        pub machine: String,
        pub cpu_config: ParsedCpuConfig,
        pub cores: u8,
        pub sockets: u8,
        pub hugepages_kb: Option<usize>,
        pub numa_enabled: bool,
        pub guest_agent: Option<GuestAgent>,
        pub tablet_enabled: bool,
        pub lifecycle_config: Option<LifecycleConfig>,
        pub balloon_config: Option<BalloonConfig>,
        pub serial_config: Option<SerialConfig>,
        pub gpu: Option<ProxmoxGpu>,
        pub smbios_uuid: Option<String>,
        pub vmgenid: Option<String>,
        pub display: Option<Display>,
    }

    pub(super) fn parse_global_config(
        global: &std::collections::BTreeMap<String, ProxmoxValue>,
    ) -> Result<ParsedGlobalConfig, String> {
        let name = extract_scalar_string(global, "name")?;
        let memory_mb = extract_scalar_u64(global, "memory")?;
        let machine = extract_machine_token(global)?;
        let cpu_model = match global.get("cpu") {
            Some(ProxmoxValue::Scalar { value }) => value.clone(),
            Some(ProxmoxValue::Compound(compound)) => {
                let mut tokens = vec![compound.head.clone()];
                for option in &compound.options {
                    match option {
                        ProxmoxOption::Flag { value } => tokens.push(value.clone()),
                        ProxmoxOption::KeyValue { key, value } => {
                            tokens.push(format!("{key}={value}"))
                        }
                    }
                }
                tokens.join(",")
            }
            None => "host".to_string(),
        };
        let cpu_config = parse_cpu_config(&cpu_model);

        let cores = extract_scalar_u8(global, "cores").unwrap_or(1);
        let sockets = extract_scalar_u8(global, "sockets").unwrap_or(1);

        let hugepages_kb = parse_hugepages_kb(global);
        let numa_enabled = parse_numa_enabled(global);

        let guest_agent = parse_guest_agent(global);
        let tablet_enabled = parse_tablet_enabled(global);
        let lifecycle_config = parse_lifecycle_config(global);
        let balloon_config = parse_balloon_config(global);
        let serial_config = parse_serial_config(global);
        let gpu = parse_gpu(global);
        let smbios_uuid = parse_smbios_uuid(global);
        let vmgenid = parse_vmgenid(global);

        let display = parse_display(global).or_else(|| {
            if matches!(gpu, Some(ProxmoxGpu::Qxl)) {
                serde_json::from_value(serde_json::json!({
                    "spice": {
                        "listen": "0.0.0.0",
                        "port": 5900,
                        "disable_ticketing": true
                    }
                }))
                .ok()
            } else {
                None
            }
        });

        Ok(ParsedGlobalConfig {
            name,
            memory_mb,
            machine,
            cpu_config,
            cores,
            sockets,
            hugepages_kb,
            numa_enabled,
            guest_agent,
            tablet_enabled,
            lifecycle_config,
            balloon_config,
            serial_config,
            gpu,
            smbios_uuid,
            vmgenid,
            display,
        })
    }
}

mod resource_collection {
    use std::collections::HashMap;

    use crate::{
        config_format::proxmox::{schema::ProxmoxValue, storage_resolver::ProxmoxStorageConfig},
        runtime_model::{PcieDeviceType, StorageDeviceOptions, StorageResource, UsbDeviceResource},
    };

    use super::parse_helpers::{
        ProxmoxIdeKind, ProxmoxNetDevice, ProxmoxScsiKind, collect_hostpci_devices,
        collect_ide_devices, collect_network_devices, collect_scsi_devices, collect_usb_devices,
        parse_storage_field,
    };

    pub(super) struct CollectedResources {
        pub storage_resources: HashMap<String, StorageResource>,
        pub scsi_disks: Vec<(
            usize,
            StorageResource,
            ProxmoxScsiKind,
            StorageDeviceOptions,
        )>,
        pub ide_disks: Vec<(usize, StorageResource, ProxmoxIdeKind, StorageDeviceOptions)>,
        pub net_devices: Vec<(usize, ProxmoxNetDevice)>,
        pub hostpci_devices: Vec<(usize, PcieDeviceType)>,
        pub usb_resources: HashMap<String, UsbDeviceResource>,
        pub usb_devices: Vec<(usize, String)>,
    }

    pub(super) fn collect_resources(
        global: &std::collections::BTreeMap<String, ProxmoxValue>,
        storage_config: &ProxmoxStorageConfig,
    ) -> Result<CollectedResources, String> {
        let mut storage_resources: HashMap<String, StorageResource> = HashMap::new();

        if let Some(storage) = parse_storage_field(global.get("efidisk0"), storage_config) {
            storage_resources.insert("efidisk0".to_string(), storage);
        }

        if let Some(storage) = parse_storage_field(global.get("tpmstate0"), storage_config) {
            storage_resources.insert("tpmstate0".to_string(), storage);
        }

        let mut scsi_disks = collect_scsi_devices(global, storage_config)?;
        scsi_disks.sort_by_key(|(idx, _, _, _)| *idx);
        for (index, storage, _, _) in &scsi_disks {
            storage_resources.insert(format!("scsi{index}"), storage.clone());
        }

        let mut ide_disks = collect_ide_devices(global, storage_config)?;
        ide_disks.sort_by_key(|(idx, _, _, _)| *idx);
        for (index, storage, _, _) in &ide_disks {
            storage_resources.insert(format!("ide{index}"), storage.clone());
        }

        let mut net_devices = collect_network_devices(global)?;
        net_devices.sort_by_key(|(idx, _)| *idx);

        let mut hostpci_devices = collect_hostpci_devices(global)?;
        hostpci_devices.sort_by_key(|(idx, _)| *idx);

        let mut usb_devices = collect_usb_devices(global)?;
        usb_devices.sort_by_key(|(idx, _)| *idx);
        let mut usb_resources: HashMap<String, UsbDeviceResource> = HashMap::new();
        let mut usb_runtime_devices: Vec<(usize, String)> = Vec::new();
        for (index, resource) in usb_devices {
            let resource_id = format!("usb{index}");
            usb_resources.insert(resource_id.clone(), resource);
            usb_runtime_devices.push((index, resource_id));
        }

        Ok(CollectedResources {
            storage_resources,
            scsi_disks,
            ide_disks,
            net_devices,
            hostpci_devices,
            usb_resources,
            usb_devices: usb_runtime_devices,
        })
    }
}

mod parse_helpers {
    use std::collections::BTreeMap;

    use crate::{
        config_format::proxmox::{
            schema::{ProxmoxCompoundValue, ProxmoxOption, ProxmoxValue},
            storage_resolver::ProxmoxStorageConfig,
        },
        runtime_model::{
            BalloonConfig, CpuModel, Display, GuestAgent, LifecycleConfig, NetworkResource,
            PcieDeviceType, SerialConfig, StorageCachePolicy, StorageDeviceOptions,
            StorageResource, UsbDeviceResource,
        },
    };

    // --- types ---

    #[derive(Clone, Copy)]
    pub(super) enum MachineChipset {
        Q35,
        I440fx,
    }

    #[derive(Clone, Copy)]
    pub(super) enum ProxmoxGpu {
        Standard,
        Virtio,
        Qxl,
        Headless,
    }

    #[derive(Clone, Copy)]
    pub(super) enum ProxmoxScsiKind {
        Hdd,
        Ssd,
        Cdrom,
    }

    #[derive(Clone, Copy)]
    pub(super) enum ProxmoxIdeKind {
        Hdd,
        Ssd,
        Cdrom,
    }

    #[derive(Clone)]
    pub(super) struct ProxmoxNetDevice {
        pub resource: NetworkResource,
        pub mac_address: Option<String>,
        pub rx_queue_size: Option<u16>,
        pub tx_queue_size: Option<u16>,
        pub vhost: Option<bool>,
    }

    // --- chipset / cpu / identity ---

    pub(super) fn parse_machine_chipset(machine: &str) -> Option<MachineChipset> {
        let token = machine.trim();

        if matches!(token, "q35" | "pc-q35") || token.starts_with("pc-q35-") {
            return Some(MachineChipset::Q35);
        }

        if matches!(token, "i440fx" | "pc-i440fx") || token.starts_with("pc-i440fx-") {
            return Some(MachineChipset::I440fx);
        }

        None
    }

    pub(super) fn parse_cpu_model(model_str: &str) -> CpuModel {
        match model_str {
            "host" => CpuModel::Host,
            _ => CpuModel::Named {
                name: model_str.to_string(),
            },
        }
    }

    pub(super) fn parse_cpu_config(model_str: &str) -> super::field_parsing::ParsedCpuConfig {
        let mut parts = model_str
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty());

        let model = parts.next().map(parse_cpu_model).unwrap_or(CpuModel::Host);

        let mut enabled_features = Vec::new();
        let mut disabled_features = Vec::new();
        for part in parts {
            if let Some(feature) = part.strip_prefix('+')
                && !feature.trim().is_empty()
            {
                enabled_features.push(feature.trim().to_string());
            }
            if let Some(feature) = part.strip_prefix('-')
                && !feature.trim().is_empty()
            {
                disabled_features.push(feature.trim().to_string());
            }
        }

        super::field_parsing::ParsedCpuConfig {
            model,
            enabled_features,
            disabled_features,
        }
    }

    pub(super) fn parse_guest_agent(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Option<GuestAgent> {
        let value = entries.get("agent")?;

        let enabled = match value {
            ProxmoxValue::Scalar { value } => {
                let token = value.trim();
                !(token == "0"
                    || token.eq_ignore_ascii_case("no")
                    || token.eq_ignore_ascii_case("false"))
            }
            ProxmoxValue::Compound(compound) => option_value(compound, "enabled")
                .map(|v| v != "0")
                .unwrap_or(true),
        };

        Some(GuestAgent { enabled })
    }

    pub(super) fn parse_tablet_enabled(entries: &BTreeMap<String, ProxmoxValue>) -> bool {
        let Some(value) = entries.get("tablet") else {
            return false;
        };

        let token = match value {
            ProxmoxValue::Scalar { value } => value.as_str(),
            ProxmoxValue::Compound(compound) => compound.head.as_str(),
        };

        parse_proxmox_bool(token).unwrap_or(false)
    }

    pub(super) fn parse_lifecycle_config(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Option<LifecycleConfig> {
        let pidfile = scalar_or_compound_head(entries.get("pidfile"));
        let daemonize = parse_bool_field(entries, "daemonize").unwrap_or(false);
        let no_shutdown = parse_bool_field(entries, "no-shutdown")
            .or_else(|| parse_bool_field(entries, "no_shutdown"))
            .unwrap_or(false);
        let qmp_socket = scalar_or_compound_head(entries.get("qmpsocket"))
            .or_else(|| scalar_or_compound_head(entries.get("qmp_socket")));
        let qmp_event_socket = scalar_or_compound_head(entries.get("qmp-event-socket"))
            .or_else(|| scalar_or_compound_head(entries.get("qmp_event_socket")));

        if pidfile.is_none()
            && !daemonize
            && !no_shutdown
            && qmp_socket.is_none()
            && qmp_event_socket.is_none()
        {
            return None;
        }

        Some(LifecycleConfig::new(
            pidfile,
            daemonize,
            no_shutdown,
            qmp_socket,
            qmp_event_socket,
        ))
    }

    pub(super) fn parse_balloon_config(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Option<BalloonConfig> {
        let value = entries.get("balloon")?;
        let (enabled, free_page_reporting) = match value {
            ProxmoxValue::Scalar { value } => {
                let enabled = value.trim() != "0";
                (enabled, false)
            }
            ProxmoxValue::Compound(compound) => {
                let enabled = compound.head.trim() != "0";
                let free_page_reporting = option_value(compound, "free-page-reporting")
                    .and_then(parse_proxmox_bool)
                    .unwrap_or(false);
                (enabled, free_page_reporting)
            }
        };

        Some(BalloonConfig::new(enabled, free_page_reporting))
    }

    pub(super) fn parse_serial_config(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Option<SerialConfig> {
        let value = entries.get("serial0")?;
        match value {
            ProxmoxValue::Scalar { value } => {
                let token = value.trim();
                if token.eq_ignore_ascii_case("socket") {
                    Some(SerialConfig::new(None, true, false))
                } else {
                    None
                }
            }
            ProxmoxValue::Compound(compound) => {
                let mode = compound.head.trim();
                if !mode.eq_ignore_ascii_case("socket") {
                    return None;
                }

                let socket_path = option_value(compound, "path")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string);

                let server = option_value(compound, "server")
                    .and_then(parse_proxmox_bool)
                    .unwrap_or(true);
                let wait = option_value(compound, "wait")
                    .and_then(parse_proxmox_bool)
                    .unwrap_or(false);

                Some(SerialConfig::new(socket_path, server, wait))
            }
        }
    }

    pub(super) fn parse_smbios_uuid(entries: &BTreeMap<String, ProxmoxValue>) -> Option<String> {
        let value = entries.get("smbios1")?;

        match value {
            ProxmoxValue::Scalar { value } => {
                if let Some(uuid) = parse_uuid_token(value.as_str()) {
                    return Some(uuid);
                }
            }
            ProxmoxValue::Compound(compound) => {
                if let Some(uuid) = parse_uuid_token(compound.head.as_str()) {
                    return Some(uuid);
                }
                if let Some(uuid) = option_value(compound, "uuid")
                    && !uuid.trim().is_empty()
                {
                    return Some(uuid.trim().to_string());
                }
            }
        }

        None
    }

    fn parse_uuid_token(token: &str) -> Option<String> {
        let raw = token.trim();
        let uuid = raw.strip_prefix("uuid=")?.trim();
        if uuid.is_empty() {
            return None;
        }
        Some(uuid.to_string())
    }

    pub(super) fn parse_vmgenid(entries: &BTreeMap<String, ProxmoxValue>) -> Option<String> {
        let value = entries.get("vmgenid")?;
        let vmgenid = match value {
            ProxmoxValue::Scalar { value } => value.trim(),
            ProxmoxValue::Compound(compound) => compound.head.trim(),
        };

        if vmgenid.is_empty() {
            return None;
        }

        Some(vmgenid.to_string())
    }

    // --- gpu / display ---

    pub(super) fn parse_gpu(entries: &BTreeMap<String, ProxmoxValue>) -> Option<ProxmoxGpu> {
        let value = entries.get("vga")?;
        let token = match value {
            ProxmoxValue::Scalar { value } => value.as_str(),
            ProxmoxValue::Compound(compound) => compound.head.as_str(),
        }
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

        if token.is_empty() {
            return Some(ProxmoxGpu::Standard);
        }
        if token == "none" || token == "serial0" {
            return Some(ProxmoxGpu::Headless);
        }
        if token.starts_with("qxl") {
            return Some(ProxmoxGpu::Qxl);
        }
        if token.starts_with("virtio") {
            return Some(ProxmoxGpu::Virtio);
        }

        Some(ProxmoxGpu::Standard)
    }

    pub(super) fn parse_display(entries: &BTreeMap<String, ProxmoxValue>) -> Option<Display> {
        if let Some(value) = entries.get("spice") {
            let spec = match value {
                ProxmoxValue::Scalar { value } => value.clone(),
                ProxmoxValue::Compound(compound) => {
                    let mut items = vec![compound.head.clone()];
                    for option in &compound.options {
                        match option {
                            ProxmoxOption::Flag { value } => items.push(value.clone()),
                            ProxmoxOption::KeyValue { key, value } => {
                                items.push(format!("{key}={value}"))
                            }
                        }
                    }
                    items.join(",")
                }
            };

            let mut listen = "0.0.0.0".to_string();
            let mut port: u16 = 5900;
            let mut tls_port: Option<u16> = None;
            let mut tls_ciphers: Option<String> = None;
            let mut disable_ticketing = false;
            let mut seamless_migration = false;

            for token in spec.split(',').map(str::trim) {
                if let Some(value) = token.strip_prefix("addr=") {
                    listen = value.to_string();
                }
                if let Some(value) = token.strip_prefix("port=")
                    && let Ok(parsed) = value.parse::<u16>()
                {
                    port = parsed;
                }
                if let Some(value) = token.strip_prefix("tls-port=")
                    && let Ok(parsed) = value.parse::<u16>()
                {
                    tls_port = Some(parsed);
                    if port == 5900 {
                        port = parsed;
                    }
                }
                if let Some(value) = token.strip_prefix("tls-ciphers=") {
                    let value = value.trim();
                    if !value.is_empty() {
                        tls_ciphers = Some(value.to_string());
                    }
                }
                if token == "disable-ticketing=on" {
                    disable_ticketing = true;
                }
                if token == "seamless-migration=on" {
                    seamless_migration = true;
                }
            }

            return Some(Display::Spice {
                spice: crate::runtime_model::Spice::new(listen, port, disable_ticketing)
                    .with_tls_port(tls_port)
                    .with_tls_ciphers(tls_ciphers)
                    .with_seamless_migration(seamless_migration),
            });
        }

        if let Some(value) = entries.get("vnc") {
            let spec = match value {
                ProxmoxValue::Scalar { value } => value.clone(),
                ProxmoxValue::Compound(compound) => {
                    let mut items = vec![compound.head.clone()];
                    for option in &compound.options {
                        match option {
                            ProxmoxOption::Flag { value } => items.push(value.clone()),
                            ProxmoxOption::KeyValue { key, value } => {
                                items.push(format!("{key}={value}"))
                            }
                        }
                    }
                    items.join(",")
                }
            };

            let mut password_auth = false;
            let target = spec.split(',').next().unwrap_or_default().trim();
            for token in spec.split(',').map(str::trim) {
                if token == "password=on" {
                    password_auth = true;
                }
            }

            if let Some(socket_path) = target.strip_prefix("unix:") {
                return Some(Display::Vnc {
                    vnc: crate::runtime_model::Vnc::new(String::new(), 0)
                        .with_socket_path(Some(socket_path.to_string()))
                        .with_password_auth(password_auth),
                });
            }

            if let Some((listen, port)) = target.rsplit_once(':')
                && let Ok(port) = port.parse::<u16>()
            {
                return Some(Display::Vnc {
                    vnc: crate::runtime_model::Vnc::new(listen.to_string(), port)
                        .with_password_auth(password_auth),
                });
            }
        }

        None
    }

    // --- system features ---

    pub(super) fn parse_hugepages_kb(entries: &BTreeMap<String, ProxmoxValue>) -> Option<usize> {
        let value = entries.get("hugepages")?;
        let token = match value {
            ProxmoxValue::Scalar { value } => value.trim(),
            ProxmoxValue::Compound(compound) => compound.head.trim(),
        };

        let hugepages_mb = token.parse::<usize>().ok()?;
        if hugepages_mb == 0 {
            return None;
        }

        Some(hugepages_mb * 1024)
    }

    pub(super) fn parse_numa_enabled(entries: &BTreeMap<String, ProxmoxValue>) -> bool {
        let Some(value) = entries.get("numa") else {
            return false;
        };

        let token = match value {
            ProxmoxValue::Scalar { value } => value.as_str(),
            ProxmoxValue::Compound(compound) => compound.head.as_str(),
        };

        parse_proxmox_bool(token).unwrap_or(false)
    }

    pub(super) fn parse_proxmox_bool(value: &str) -> Option<bool> {
        let token = value.trim();
        if token.eq_ignore_ascii_case("1")
            || token.eq_ignore_ascii_case("on")
            || token.eq_ignore_ascii_case("yes")
            || token.eq_ignore_ascii_case("true")
        {
            return Some(true);
        }
        if token.eq_ignore_ascii_case("0")
            || token.eq_ignore_ascii_case("off")
            || token.eq_ignore_ascii_case("no")
            || token.eq_ignore_ascii_case("false")
        {
            return Some(false);
        }
        None
    }

    // --- resource collectors ---

    pub(super) fn collect_scsi_devices(
        entries: &BTreeMap<String, ProxmoxValue>,
        storage_config: &ProxmoxStorageConfig,
    ) -> Result<
        Vec<(
            usize,
            StorageResource,
            ProxmoxScsiKind,
            StorageDeviceOptions,
        )>,
        String,
    > {
        let mut out = Vec::new();
        for (key, value) in entries {
            let Some(suffix) = key.strip_prefix("scsi") else {
                continue;
            };
            if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            let index = suffix
                .parse::<usize>()
                .map_err(|_| format!("invalid scsi index in key '{key}'"))?;
            let Some(storage) = parse_storage_field(Some(value), storage_config) else {
                continue;
            };

            let kind = match value {
                ProxmoxValue::Compound(compound) => {
                    if has_option(compound, "media", "cdrom") {
                        ProxmoxScsiKind::Cdrom
                    } else if has_option(compound, "ssd", "1") {
                        ProxmoxScsiKind::Ssd
                    } else {
                        ProxmoxScsiKind::Hdd
                    }
                }
                ProxmoxValue::Scalar { .. } => ProxmoxScsiKind::Hdd,
            };

            let options = parse_storage_device_options(value);

            out.push((index, storage, kind, options));
        }
        Ok(out)
    }

    pub(super) fn collect_ide_devices(
        entries: &BTreeMap<String, ProxmoxValue>,
        storage_config: &ProxmoxStorageConfig,
    ) -> Result<Vec<(usize, StorageResource, ProxmoxIdeKind, StorageDeviceOptions)>, String> {
        let mut out = Vec::new();
        for (key, value) in entries {
            let Some(suffix) = key.strip_prefix("ide") else {
                continue;
            };
            if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            let index = suffix
                .parse::<usize>()
                .map_err(|_| format!("invalid ide index in key '{key}'"))?;

            // Current runtime IDE renderer models secondary channel units only.
            if index < 2 {
                continue;
            }

            let Some(storage) = parse_storage_field(Some(value), storage_config) else {
                continue;
            };

            let kind = match value {
                ProxmoxValue::Compound(compound) => {
                    if has_option(compound, "media", "cdrom") {
                        ProxmoxIdeKind::Cdrom
                    } else if has_option(compound, "ssd", "1") {
                        ProxmoxIdeKind::Ssd
                    } else {
                        ProxmoxIdeKind::Hdd
                    }
                }
                ProxmoxValue::Scalar { .. } => ProxmoxIdeKind::Hdd,
            };

            let options = parse_storage_device_options(value);

            out.push((index, storage, kind, options));
        }
        Ok(out)
    }

    fn parse_storage_device_options(value: &ProxmoxValue) -> StorageDeviceOptions {
        let ProxmoxValue::Compound(compound) = value else {
            return StorageDeviceOptions::default();
        };

        let rotation_rate =
            option_value(compound, "rotation_rate").and_then(|raw| raw.parse::<u16>().ok());
        let boot_index =
            option_value(compound, "bootindex").and_then(|raw| raw.parse::<u16>().ok());
        let device_id = option_value(compound, "serial")
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
            .map(ToString::to_string);
        let cache_policy =
            option_value(compound, "cache").and_then(StorageCachePolicy::from_proxmox_value);

        StorageDeviceOptions::from_parts(rotation_rate, boot_index, device_id, cache_policy)
    }

    pub(super) fn collect_network_devices(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Result<Vec<(usize, ProxmoxNetDevice)>, String> {
        let mut out = Vec::new();
        for (key, value) in entries {
            let Some(suffix) = key.strip_prefix("net") else {
                continue;
            };
            if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
            let index = suffix
                .parse::<usize>()
                .map_err(|_| format!("invalid network index in key '{key}'"))?;

            let config = match value {
                ProxmoxValue::Compound(compound) => {
                    let resource = if let Some(bridge) = option_value(compound, "bridge") {
                        Some(NetworkResource::Bridge {
                            bridge: bridge.to_string(),
                        })
                    } else {
                        option_value(compound, "ifname").map(|tap| NetworkResource::Tap {
                            tap: tap.to_string(),
                        })
                    };

                    let mac_address = compound
                        .head
                        .split_once('=')
                        .map(|(_, value)| value.trim())
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string);

                    let rx_queue_size = option_value(compound, "rx_queue_size")
                        .and_then(|value| value.parse::<u16>().ok());
                    let tx_queue_size = option_value(compound, "tx_queue_size")
                        .and_then(|value| value.parse::<u16>().ok());
                    let vhost = option_value(compound, "vhost").and_then(parse_proxmox_bool);

                    resource.map(|resource| ProxmoxNetDevice {
                        resource,
                        mac_address,
                        rx_queue_size,
                        tx_queue_size,
                        vhost,
                    })
                }
                ProxmoxValue::Scalar { .. } => None,
            };

            if let Some(config) = config {
                out.push((index, config));
            }
        }
        Ok(out)
    }

    pub(super) fn collect_hostpci_devices(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Result<Vec<(usize, PcieDeviceType)>, String> {
        let mut out = Vec::new();

        for (key, value) in entries {
            let Some(suffix) = key.strip_prefix("hostpci") else {
                continue;
            };
            if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }

            let index = suffix
                .parse::<usize>()
                .map_err(|_| format!("invalid hostpci index in key '{key}'"))?;

            let (host, resource, multifunction, rombar, romfile) = match value {
                ProxmoxValue::Scalar { value } => {
                    let host = value.trim().to_string();
                    if host.is_empty() {
                        continue;
                    }
                    (host, Some(key.clone()), None, None, None)
                }
                ProxmoxValue::Compound(compound) => {
                    let host = compound.head.trim().to_string();
                    if host.is_empty() {
                        continue;
                    }

                    let multifunction =
                        option_value(compound, "multifunction").and_then(parse_proxmox_bool);
                    let rombar = option_value(compound, "rombar").and_then(parse_proxmox_bool);
                    let romfile = option_value(compound, "romfile")
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string);

                    (host, Some(key.clone()), multifunction, rombar, romfile)
                }
            };

            out.push((
                index,
                PcieDeviceType::Passthrough {
                    resource,
                    host: Some(host),
                    id: None,
                    multifunction,
                    rombar,
                    romfile,
                },
            ));
        }

        Ok(out)
    }

    pub(super) fn collect_usb_devices(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Result<Vec<(usize, UsbDeviceResource)>, String> {
        let mut out = Vec::new();

        for (key, value) in entries {
            let Some(suffix) = key.strip_prefix("usb") else {
                continue;
            };
            if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }

            let index = suffix
                .parse::<usize>()
                .map_err(|_| format!("invalid USB index in key '{key}'"))?;

            if let Some(resource) = parse_usb_host_resource(value)? {
                out.push((index, resource));
            }
        }

        Ok(out)
    }

    pub(super) fn parse_storage_field(
        value: Option<&ProxmoxValue>,
        storage_config: &ProxmoxStorageConfig,
    ) -> Option<StorageResource> {
        let token = match value {
            Some(ProxmoxValue::Scalar { value }) => value.as_str(),
            Some(ProxmoxValue::Compound(compound)) => compound.head.as_str(),
            None => return None,
        };

        storage_config.token_to_resource(token)
    }

    pub(super) fn extract_scalar_string(
        entries: &BTreeMap<String, ProxmoxValue>,
        key: &str,
    ) -> Result<String, String> {
        entries
            .get(key)
            .ok_or_else(|| format!("Required field '{}' not found", key))
            .and_then(|value| match value {
                ProxmoxValue::Scalar { value } => Ok(value.clone()),
                ProxmoxValue::Compound(_) => {
                    Err(format!("Expected scalar value for '{}', got compound", key))
                }
            })
    }

    pub(super) fn extract_scalar_u64(
        entries: &BTreeMap<String, ProxmoxValue>,
        key: &str,
    ) -> Result<u64, String> {
        extract_scalar_string(entries, key).and_then(|value| {
            value
                .parse::<u64>()
                .map_err(|_| format!("Field '{}' has non-numeric value: '{}'", key, value))
        })
    }

    pub(super) fn extract_machine_token(
        entries: &BTreeMap<String, ProxmoxValue>,
    ) -> Result<String, String> {
        entries
            .get("machine")
            .ok_or_else(|| "Required field 'machine' not found".to_string())
            .map(|value| match value {
                ProxmoxValue::Scalar { value } => value.clone(),
                ProxmoxValue::Compound(compound) => compound.head.clone(),
            })
    }

    pub(super) fn extract_scalar_u8(
        entries: &BTreeMap<String, ProxmoxValue>,
        key: &str,
    ) -> Result<u8, String> {
        extract_scalar_string(entries, key).and_then(|value| {
            value
                .parse::<u8>()
                .map_err(|_| format!("Field '{}' has non-numeric value: '{}'", key, value))
        })
    }

    pub(super) fn option_value<'a>(
        compound: &'a ProxmoxCompoundValue,
        key: &str,
    ) -> Option<&'a str> {
        compound.options.iter().find_map(|option| match option {
            ProxmoxOption::KeyValue { key: k, value } if k == key => Some(value.as_str()),
            _ => None,
        })
    }

    fn has_option(compound: &ProxmoxCompoundValue, key: &str, expected: &str) -> bool {
        option_value(compound, key).is_some_and(|value| value == expected)
    }

    fn parse_bool_field(entries: &BTreeMap<String, ProxmoxValue>, key: &str) -> Option<bool> {
        let value = entries.get(key)?;
        let token = match value {
            ProxmoxValue::Scalar { value } => value.as_str(),
            ProxmoxValue::Compound(compound) => compound.head.as_str(),
        };
        parse_proxmox_bool(token)
    }

    fn scalar_or_compound_head(value: Option<&ProxmoxValue>) -> Option<String> {
        let token = match value? {
            ProxmoxValue::Scalar { value } => value.trim(),
            ProxmoxValue::Compound(compound) => compound.head.trim(),
        };
        if token.is_empty() {
            return None;
        }
        Some(token.to_string())
    }

    fn parse_usb_host_resource(value: &ProxmoxValue) -> Result<Option<UsbDeviceResource>, String> {
        let host_value = match value {
            ProxmoxValue::Scalar { value } => value
                .strip_prefix("host=")
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            ProxmoxValue::Compound(compound) => option_value(compound, "host").or_else(|| {
                compound
                    .head
                    .strip_prefix("host=")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            }),
        };

        let Some(host) = host_value else {
            return Ok(None);
        };

        if let Some((vendor_hex, product_hex)) = host.split_once(':')
            && vendor_hex.len() == 4
            && product_hex.len() == 4
            && vendor_hex.chars().all(|ch| ch.is_ascii_hexdigit())
            && product_hex.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            let vendor_id = u16::from_str_radix(vendor_hex, 16)
                .map_err(|_| format!("invalid USB vendor id '{vendor_hex}'"))?;
            let device_id = u16::from_str_radix(product_hex, 16)
                .map_err(|_| format!("invalid USB product id '{product_hex}'"))?;
            return Ok(Some(UsbDeviceResource::Id {
                vendor_id,
                device_id,
            }));
        }

        if let Some((hostbus, hostport)) = host.split_once('-') {
            let hostbus = hostbus
                .trim()
                .parse::<u16>()
                .map_err(|_| format!("invalid USB hostbus '{hostbus}'"))?;
            let hostport = hostport.trim();
            if hostport.is_empty() {
                return Err("invalid USB hostport: empty value".to_string());
            }
            return Ok(Some(UsbDeviceResource::HostBusPort {
                hostbus,
                hostport: hostport.to_string(),
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::config_format::proxmox::{
        schema::ProxmoxConfigSchema, storage_resolver::ProxmoxStorageConfig,
    };
    use crate::config_format::qemu_cmd::runtime_render::render_qemu_command;

    use super::{ProxmoxRuntimeBuilder, RuntimeBuilder};

    fn storage_cfg() -> ProxmoxStorageConfig {
        ProxmoxStorageConfig::parse(
            r#"
dir: local
    path /var/lib/vz

lvmthin: vm1-pool
    vgname vm1

lvm: vm1
    vgname vm1
"#,
        )
        .expect("storage cfg should parse")
    }

    fn build(conf_text: &str) -> crate::runtime_model::RuntimeModel {
        let schema = ProxmoxConfigSchema::parse(conf_text).expect("config should parse");
        ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_cfg())
            .with_schema(schema)
            .build()
            .expect("runtime model should build")
    }

    #[test]
    fn builds_runtime_model_from_basic_proxmox_config() {
        let model = build(
            r#"
name: test-vm
memory: 4096
machine: q35
cpu: host
cores: 4
sockets: 1
"#,
        );
        assert_eq!(model.name(), "test-vm");
    }

    #[test]
    fn builds_i440fx_chipset_when_specified() {
        build(
            r#"
name: legacy-vm
memory: 2048
machine: i440fx
cpu: host
cores: 2
sockets: 1
"#,
        );
    }

    #[test]
    fn builds_q35_chipset_from_versioned_machine_value() {
        build(
            r#"
name: versioned-q35-vm
memory: 2048
machine: pc-q35-8.1
cpu: host
cores: 2
sockets: 1
"#,
        );
    }

    #[test]
    fn builds_q35_chipset_from_compound_machine_value() {
        build(
            r#"
name: compound-q35-vm
memory: 2048
machine: pc-q35-8.1,viommu=intel
cpu: host
cores: 2
sockets: 1
"#,
        );
    }

    #[test]
    fn rejects_missing_required_name_field() {
        let schema = ProxmoxConfigSchema::parse(
            r#"
memory: 4096
machine: q35
cpu: host
"#,
        )
        .expect("config should parse");
        let result = ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_cfg())
            .with_schema(schema)
            .build();
        assert!(result.is_err());
        match result {
            Err(e) => assert!(e.contains("name"), "Error should mention 'name': {}", e),
            Ok(_) => panic!("Should have failed due to missing name"),
        }
    }

    #[test]
    fn rejects_missing_required_memory_field() {
        let schema = ProxmoxConfigSchema::parse(
            r#"
name: incomplete-vm
machine: q35
cpu: host
cores: 2
"#,
        )
        .expect("config should parse");
        let result = ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_cfg())
            .with_schema(schema)
            .build();
        assert!(result.is_err());
        match result {
            Err(e) => assert!(e.contains("memory"), "Error should mention 'memory': {}", e),
            Ok(_) => panic!("Should have failed due to missing memory"),
        }
    }

    #[test]
    fn rejects_invalid_memory_value() {
        let schema = ProxmoxConfigSchema::parse(
            r#"
name: bad-memory-vm
memory: not-a-number
machine: q35
cpu: host
"#,
        )
        .expect("config should parse");
        let result = ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_cfg())
            .with_schema(schema)
            .build();
        assert!(result.is_err());
        match result {
            Err(e) => assert!(
                e.contains("non-numeric"),
                "Error should mention parsing issue: {}",
                e
            ),
            Ok(_) => panic!("Should have failed due to invalid memory value"),
        }
    }

    #[test]
    fn rejects_unsupported_chipset() {
        let schema = ProxmoxConfigSchema::parse(
            r#"
name: unsupported-chipset-vm
memory: 4096
machine: unsupported
cpu: host
cores: 2
"#,
        )
        .expect("config should parse");
        let result = ProxmoxRuntimeBuilder::default()
            .with_storage_config(storage_cfg())
            .with_schema(schema)
            .build();
        assert!(result.is_err());
        match result {
            Err(e) => assert!(
                e.contains("Unsupported chipset"),
                "Error should mention unsupported chipset: {}",
                e
            ),
            Ok(_) => panic!("Should have failed due to unsupported chipset"),
        }
    }

    #[test]
    fn uses_default_cpu_model_host() {
        build(
            r#"
name: default-cpu-vm
memory: 4096
machine: q35
cores: 2
sockets: 1
"#,
        );
    }

    #[test]
    fn uses_default_cpu_topology() {
        build(
            r#"
name: default-topology-vm
memory: 4096
machine: q35
cpu: host
"#,
        );
    }

    #[test]
    fn maps_uefi_tpm_scsi_and_network_into_runtime() {
        let model = build(
            r#"
name: rich-vm
memory: 8192
machine: q35
cpu: host
cores: 4
sockets: 1
bios: ovmf
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,size=4M
tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
scsihw: pvscsi
scsi0: vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1
net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0
"#,
        );

        let command = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(command.iter().any(|arg| arg.contains("if=pflash,unit=1")));
        assert!(command.iter().any(|arg| arg.contains("tpm-tis")));
        assert!(command.iter().any(|arg| arg.contains("pvscsi")));
        assert!(command.iter().any(|arg| arg.contains("scsi-hd")));
        assert!(command.iter().any(|arg| arg.contains("br=vmbr0")));
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("mac=BC:24:11:3A:21:B7"))
        );
    }

    #[test]
    fn imports_network_queue_sizes_and_vhost_into_runtime() {
        let model = build(
            r#"
name: network-queues-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
net0: virtio=AA:BB:CC:DD:EE:FF,ifname=tap301i0,vhost=on,rx_queue_size=1024,tx_queue_size=256
"#,
        );

        let command = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("tap,id=") && arg.contains("ifname=tap301i0"))
        );
        assert!(command.iter().any(|arg| arg.contains("vhost=on")));
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("mac=AA:BB:CC:DD:EE:FF"))
        );
        assert!(command.iter().any(|arg| arg.contains("rx_queue_size=1024")));
        assert!(command.iter().any(|arg| arg.contains("tx_queue_size=256")));
    }

    #[test]
    fn maps_guest_agent_and_virtio_gpu_from_proxmox_fields() {
        let model = build(
            r#"
name: agent-gpu-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
agent: 1
vga: virtio,memory=128
"#,
        );
        let command = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(command.iter().any(|arg| arg.contains("virtio-gpu-pci")));
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("org.qemu.guest_agent.0"))
        );
    }

    #[test]
    fn maps_qxl_gpu_and_spice_display_from_proxmox_fields() {
        let model = build(
            r#"
name: qxl-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
vga: qxl,memory=64
spice: port=5905,addr=127.0.0.1,disable-ticketing=on
"#,
        );
        let command = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(command.iter().any(|arg| arg == "qxl" || arg == "VGA"));
        assert!(
            command
                .iter()
                .any(|arg| arg.contains("port=5905,addr=127.0.0.1,disable-ticketing=on"))
        );
    }

    #[test]
    fn infers_spice_display_from_qxl_vga_when_no_spice_field() {
        let model = build(
            r#"
name: qxl-implicit-spice
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
vga: qxl,memory=64
"#,
        );
        let command = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            command.iter().any(|arg| arg.contains("port=5900")),
            "expected inferred SPICE at port 5900, got: {:?}",
            command
        );
    }

    #[test]
    fn imports_identity_fields_from_proxmox_config() {
        let model = build(
            r#"
name: identity-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
smbios1: uuid=1f0f0f0f-1111-2222-3333-444444444444
vmgenid: 55555555-6666-7777-8888-999999999999
"#,
        );
        assert_eq!(
            model.smbios_uuid().as_deref(),
            Some("1f0f0f0f-1111-2222-3333-444444444444")
        );
        assert_eq!(
            model.vmgenid().as_deref(),
            Some("55555555-6666-7777-8888-999999999999")
        );
    }

    #[test]
    fn imports_vnc_socket_display_from_proxmox_config() {
        let model = build(
            r#"
name: vnc-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
vnc: unix:/var/run/qemu-server/301.vnc,password=on
"#,
        );

        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("unix:/var/run/qemu-server/301.vnc"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("password=on")));
    }

    #[test]
    fn imports_spice_tls_display_from_proxmox_config() {
        let model = build(
            r#"
name: spice-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
spice: port=5905,tls-port=61005,addr=127.0.0.1,tls-ciphers=HIGH,seamless-migration=on,disable-ticketing=on
"#,
        );

        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("port=5905") && arg.contains("tls-port=61005"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("tls-ciphers=HIGH")));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("seamless-migration=on"))
        );
    }

    #[test]
    fn imports_smbios_uuid_when_not_first_token() {
        let model = build(
            r#"
name: identity-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
smbios1: manufacturer=acme,uuid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee
"#,
        );
        assert_eq!(
            model.smbios_uuid().as_deref(),
            Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
    }

    #[test]
    fn imports_hostpci_passthrough_devices() {
        let model = build(
            r#"
name: hostpci-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
hostpci0: 0000:0e:11.6,pcie=1,rombar=0
hostpci1: 0000:01:00.1,pcie=1
"#,
        );
        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("vfio-pci,host=0000:0e:11.6"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("vfio-pci,host=0000:01:00.1"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("rombar=0")));
    }

    #[test]
    fn imports_hugepages_and_numa_memory_backend() {
        let model = build(
            r#"
name: numa-vm
memory: 16384
machine: q35
cpu: host
cores: 8
sockets: 1
hugepages: 1024
numa: 1
"#,
        );
        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("memory-backend-file,id=ram-node0,size=16384M"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("mem-path=/run/hugepages/kvm/1048576kB"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("node,nodeid=0,cpus=0-7,memdev=ram-node0"))
        );
    }

    #[test]
    fn imports_usb_tablet_and_host_passthrough_devices() {
        let model = build(
            r#"
name: usb-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
tablet: 1
usb0: host=1-7.5.1
usb1: host=0451:16a0
"#,
        );

        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("qemu-xhci") && arg.contains("id=xhci0"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("usb-tablet") && arg.contains("bus=xhci0.0"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("usb-host")
            && arg.contains("hostbus=1")
            && arg.contains("hostport=7.5.1")));
        assert!(rendered.iter().any(|arg| {
            arg.contains("usb-host")
                && arg.contains("vendorid=0x0451")
                && arg.contains("productid=0x16a0")
        }));
    }

    #[test]
    fn imports_lifecycle_and_qmp_monitoring_fields() {
        let model = build(
            r#"
name: lifecycle-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
pidfile: /run/qemu/lifecycle-vm.pid
daemonize: 1
no-shutdown: 1
qmpsocket: /run/qemu/lifecycle-vm.qmp
qmp-event-socket: /run/qemu/lifecycle-vm.event
"#,
        );

        let lifecycle = model
            .lifecycle_config()
            .as_ref()
            .expect("lifecycle config should be imported");
        assert_eq!(
            lifecycle.pidfile().as_deref(),
            Some("/run/qemu/lifecycle-vm.pid")
        );
        assert!(*lifecycle.daemonize());
        assert!(*lifecycle.no_shutdown());
        assert_eq!(
            lifecycle.qmp_socket().as_deref(),
            Some("/run/qemu/lifecycle-vm.qmp")
        );
        assert_eq!(
            lifecycle.qmp_event_socket().as_deref(),
            Some("/run/qemu/lifecycle-vm.event")
        );

        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(rendered.iter().any(|arg| arg == "-daemonize"));
        assert!(rendered.iter().any(|arg| arg == "-no-shutdown"));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("socket,id=qmp,path=/run/qemu/lifecycle-vm.qmp"))
        );
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("socket,id=qmp-event,path=/run/qemu/lifecycle-vm.event"))
        );
    }

    #[test]
    fn imports_ide_balloon_and_serial_fields() {
        let model = build(
            r#"
name: devices-vm
memory: 4096
machine: q35
cpu: host
cores: 2
sockets: 1
ide2: local:iso/debian.iso,media=cdrom
balloon: 1,free-page-reporting=on
serial0: socket,path=/run/qemu/devices-vm.serial0,server=on,wait=off
"#,
        );

        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(rendered.iter().any(|arg| arg.contains("ide-cd")
            && arg.contains("bus=ide.1")
            && arg.contains("unit=0")));
        assert!(rendered.iter().any(
            |arg| arg.contains("virtio-balloon-pci") && arg.contains("free-page-reporting=on")
        ));
        assert!(rendered.iter().any(|arg| {
            arg.contains("socket,id=serial0")
                && arg.contains("path=/run/qemu/devices-vm.serial0")
                && arg.contains("server=on")
                && arg.contains("wait=off")
        }));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("isa-serial,chardev=serial0"))
        );
    }

    #[test]
    fn imports_cpu_feature_flags_and_scsi_advanced_options() {
        let model = build(
            r#"
name: phase4-vm
memory: 4096
machine: q35
cpu: host,+kvm_pv_eoi,-vmx
cores: 2
sockets: 1
scsihw: pvscsi
scsi0: vm1-pool:vm-108-boot,cache=none,rotation_rate=7200,bootindex=2,serial=DISK-SN-01
"#,
        );

        assert!(
            model
                .cpu()
                .enabled_features()
                .iter()
                .any(|feature| feature == "kvm_pv_eoi")
        );
        assert!(
            model
                .cpu()
                .disabled_features()
                .iter()
                .any(|feature| feature == "vmx")
        );

        let rendered = render_qemu_command(&model).expect("qemu render should succeed");
        assert!(rendered.iter().any(|arg| arg.contains("cache=none")));
        assert!(
            rendered
                .iter()
                .any(|arg| arg.contains("rotation_rate=7200"))
        );
        assert!(rendered.iter().any(|arg| arg.contains("bootindex=2")));
        assert!(rendered.iter().any(|arg| arg.contains("serial=DISK-SN-01")));
    }
}
