use saphyr::{ScalarOwned, Yaml, YamlEmitter, YamlOwned};
use serde::Serialize;

use crate::config_format::ezkvm::schema::{
    AudioSchema, Device, DisplaySchema, EzkvmConfigSchema, HostSchema, Machine, Metadata,
    VirtualMachine,
};
use crate::runtime_model::{NetworkResource, PcieDeviceResource, Resource, StorageResource};
use crate::serde_yaml;

#[derive(Debug, Clone, Copy)]
pub enum StyleHint {
    TopBlock,
    Block,
    FlowPreferred,
}

#[derive(Debug, Clone)]
pub struct StyledYaml {
    node: StyledNode,
    style: StyleHint,
}

#[derive(Debug, Clone)]
enum StyledNode {
    Raw(YamlOwned),
    Mapping(Vec<(String, StyledYaml)>),
    Sequence(Vec<StyledYaml>),
}

pub trait ToStyledYaml {
    fn to_styled_yaml(&self) -> StyledYaml;
}

fn scalar_string(value: impl Into<String>) -> YamlOwned {
    YamlOwned::Value(ScalarOwned::String(value.into()))
}

fn scalar_bool(value: bool) -> YamlOwned {
    YamlOwned::Value(ScalarOwned::Boolean(value))
}

fn from_serde<T: Serialize>(value: &T) -> YamlOwned {
    serde_yaml::to_value(value).expect("failed to serialize value for styled yaml rendering")
}

fn raw_node(value: YamlOwned, style: StyleHint) -> StyledYaml {
    StyledYaml {
        node: StyledNode::Raw(value),
        style,
    }
}

fn mapping_node(entries: Vec<(String, StyledYaml)>, style: StyleHint) -> StyledYaml {
    StyledYaml {
        node: StyledNode::Mapping(entries),
        style,
    }
}

fn sequence_node(items: Vec<StyledYaml>, style: StyleHint) -> StyledYaml {
    StyledYaml {
        node: StyledNode::Sequence(items),
        style,
    }
}

fn emit_owned(value: &YamlOwned, compact: bool) -> Result<String, String> {
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);
    emitter.compact(compact);
    let yaml_ref: Yaml = value.into();
    emitter
        .dump(&yaml_ref)
        .map_err(|e| format!("YAML emit error: {e:?}"))?;

    Ok(output
        .strip_prefix("---\n")
        .unwrap_or(&output)
        .trim_end_matches('\n')
        .to_string())
}

fn render_flow_inline(styled: &StyledYaml) -> Result<String, String> {
    match &styled.node {
        StyledNode::Raw(value) => render_flow_owned(value),
        StyledNode::Mapping(entries) => {
            let mut parts = Vec::new();
            for (key, value) in entries {
                let rendered = render_flow_inline(value)?;
                parts.push(format!("{}: {}", key, rendered));
            }
            Ok(format!("{{{}}}", parts.join(", ")))
        }
        StyledNode::Sequence(items) => {
            let mut parts = Vec::new();
            for item in items {
                parts.push(render_flow_inline(item)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
    }
}

fn render_flow_owned(value: &YamlOwned) -> Result<String, String> {
    match value {
        YamlOwned::Mapping(map) => {
            let mut parts = Vec::new();
            for (key, val) in map {
                let rendered_key = emit_owned(key, true)?;
                let rendered_val = render_flow_owned(val)?;
                parts.push(format!("{}: {}", rendered_key, rendered_val));
            }
            Ok(format!("{{{}}}", parts.join(", ")))
        }
        YamlOwned::Sequence(items) => {
            let mut parts = Vec::new();
            for item in items {
                parts.push(render_flow_owned(item)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        YamlOwned::Tagged(_, inner) => render_flow_owned(inner),
        _ => emit_owned(value, true),
    }
}

fn extend_flattened_mapping(
    entries: &mut Vec<(String, StyledYaml)>,
    yaml: YamlOwned,
    style: StyleHint,
) {
    if let YamlOwned::Mapping(map) = yaml {
        for (key, value) in map {
            if let YamlOwned::Value(ScalarOwned::String(key_string)) = key {
                entries.push((key_string, raw_node(value, style)));
            }
        }
    }
}

fn indent_lines(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_with_style(styled: &StyledYaml, indent: usize) -> Result<String, String> {
    match styled.style {
        StyleHint::FlowPreferred => render_flow_inline(styled),
        StyleHint::Block => render_block(styled, indent, None),
        StyleHint::TopBlock => render_block(styled, indent, Some("")),
    }
}

fn render_block(
    styled: &StyledYaml,
    indent: usize,
    seperator: Option<&str>,
) -> Result<String, String> {
    let mut out = Vec::new();
    match &styled.node {
        StyledNode::Raw(value) => emit_owned(value, false),
        StyledNode::Mapping(entries) => {
            for (key, value) in entries {
                if matches!(value.style, StyleHint::FlowPreferred) {
                    let inline = render_flow_inline(value)?;
                    out.push(format!("{}{}: {}", " ".repeat(indent), key, inline));
                    continue;
                }

                let nested = render_with_style(value, indent)?;
                let is_block_sequence = matches!(value.node, StyledNode::Sequence(_));
                if nested.contains('\n') || is_block_sequence {
                    out.push(format!("{}{}:", " ".repeat(indent), key));
                    out.push(indent_lines(&nested, indent + 2));
                } else {
                    out.push(format!("{}{}: {}", " ".repeat(indent), key, nested));
                }

                match seperator {
                    Some(sep) => out.push(sep.to_string()),
                    None => {}
                }
            }
            Ok(out.join("\n"))
        }
        StyledNode::Sequence(items) => {
            for item in items {
                if matches!(item.style, StyleHint::FlowPreferred) {
                    let inline = render_flow_inline(item)?;
                    out.push(format!("{}- {}", " ".repeat(indent), inline));
                    continue;
                }

                let nested = render_with_style(item, indent + 2)?;
                if nested.contains('\n') {
                    let mut lines = nested.lines();
                    if let Some(first) = lines.next() {
                        out.push(format!("{}- {}", " ".repeat(indent), first.trim_start()));
                    } else {
                        out.push(format!("{}-", " ".repeat(indent)));
                    }
                    for line in lines {
                        out.push(line.to_string());
                    }
                } else {
                    out.push(format!("{}- {}", " ".repeat(indent), nested));
                }

                match seperator {
                    Some(sep) => out.push(sep.to_string()),
                    None => {}
                }
            }
            Ok(out.join("\n"))
        }
    }
}

impl ToStyledYaml for EzkvmConfigSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        mapping_node(
            vec![
                ("metadata".to_string(), self.metadata.to_styled_yaml()),
                ("host".to_string(), self.host.to_styled_yaml()),
                (
                    "virtual_machine".to_string(),
                    self.virtual_machine.to_styled_yaml(),
                ),
            ],
            StyleHint::TopBlock,
        )
    }
}

impl ToStyledYaml for Metadata {
    fn to_styled_yaml(&self) -> StyledYaml {
        mapping_node(
            vec![
                (
                    "schema_version".to_string(),
                    raw_node(scalar_string(&self.schema_version), StyleHint::Block),
                ),
                (
                    "vm_name".to_string(),
                    raw_node(scalar_string(&self.vm_name), StyleHint::Block),
                ),
            ],
            StyleHint::Block,
        )
    }
}

impl ToStyledYaml for HostSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        let mut entries: Vec<(String, StyledYaml)> = Vec::new();

        if let Some(display) = &self.display {
            let display_entry = match display {
                DisplaySchema::Vnc { vnc } => (
                    "vnc".to_string(),
                    raw_node(from_serde(vnc), StyleHint::FlowPreferred),
                ),
                DisplaySchema::Spice { spice } => (
                    "spice".to_string(),
                    raw_node(from_serde(spice), StyleHint::FlowPreferred),
                ),
                DisplaySchema::LookingGlass { looking_glass } => (
                    "looking_glass".to_string(),
                    raw_node(from_serde(looking_glass), StyleHint::FlowPreferred),
                ),
                DisplaySchema::Gtk { gtk } => (
                    "gtk".to_string(),
                    raw_node(from_serde(gtk), StyleHint::FlowPreferred),
                ),
                DisplaySchema::Sdl { sdl } => (
                    "sdl".to_string(),
                    raw_node(from_serde(sdl), StyleHint::FlowPreferred),
                ),
                DisplaySchema::EglHeadless { egl_headless } => (
                    "egl_headless".to_string(),
                    raw_node(from_serde(egl_headless), StyleHint::FlowPreferred),
                ),
            };
            entries.push(display_entry);
        }

        if let Some(audio) = &self.audio {
            let audio_entry = match audio {
                AudioSchema::Alsa { alsa } => (
                    "alsa".to_string(),
                    raw_node(from_serde(alsa), StyleHint::FlowPreferred),
                ),
                AudioSchema::PulseAudio { pulse_audio } => (
                    "pulse_audio".to_string(),
                    raw_node(from_serde(pulse_audio), StyleHint::FlowPreferred),
                ),
                AudioSchema::PipeWire { pipe_wire } => (
                    "pipe_wire".to_string(),
                    raw_node(from_serde(pipe_wire), StyleHint::FlowPreferred),
                ),
            };
            entries.push(audio_entry);
        }

        let resources = self
            .resources
            .iter()
            .map(ToStyledYaml::to_styled_yaml)
            .collect();
        entries.push((
            "resources".to_string(),
            sequence_node(resources, StyleHint::Block),
        ));

        mapping_node(entries, StyleHint::Block)
    }
}

impl ToStyledYaml for Resource {
    fn to_styled_yaml(&self) -> StyledYaml {
        let node = match self {
            Resource::Storage { id, storage } => {
                let storage_value = match storage {
                    StorageResource::File { file } => mapping_node(
                        vec![(
                            "file".to_string(),
                            raw_node(scalar_string(file), StyleHint::FlowPreferred),
                        )],
                        StyleHint::FlowPreferred,
                    ),
                    StorageResource::BlockDevice { block_device } => mapping_node(
                        vec![(
                            "block_device".to_string(),
                            raw_node(scalar_string(block_device), StyleHint::FlowPreferred),
                        )],
                        StyleHint::FlowPreferred,
                    ),
                };

                mapping_node(
                    vec![
                        (
                            "id".to_string(),
                            raw_node(scalar_string(id), StyleHint::Block),
                        ),
                        ("storage".to_string(), storage_value),
                    ],
                    StyleHint::Block,
                )
            }
            Resource::Network { id, network } => {
                let network_value = match network {
                    NetworkResource::Tap { tap } => mapping_node(
                        vec![(
                            "tap".to_string(),
                            raw_node(scalar_string(tap), StyleHint::FlowPreferred),
                        )],
                        StyleHint::FlowPreferred,
                    ),
                    NetworkResource::Bridge { bridge } => mapping_node(
                        vec![(
                            "bridge".to_string(),
                            raw_node(scalar_string(bridge), StyleHint::FlowPreferred),
                        )],
                        StyleHint::FlowPreferred,
                    ),
                };

                mapping_node(
                    vec![
                        (
                            "id".to_string(),
                            raw_node(scalar_string(id), StyleHint::Block),
                        ),
                        ("network".to_string(), network_value),
                    ],
                    StyleHint::Block,
                )
            }
            Resource::PcieDevice { id, pcie } => {
                let pcie_value = match pcie {
                    PcieDeviceResource::HostAddress {
                        address,
                        multifunction,
                        rombar,
                        romfile,
                    } => {
                        let mut entries = vec![(
                            "address".to_string(),
                            raw_node(scalar_string(address), StyleHint::FlowPreferred),
                        )];
                        if let Some(value) = multifunction {
                            entries.push((
                                "multifunction".to_string(),
                                raw_node(scalar_bool(*value), StyleHint::FlowPreferred),
                            ));
                        }
                        if let Some(value) = rombar {
                            entries.push((
                                "rombar".to_string(),
                                raw_node(scalar_bool(*value), StyleHint::FlowPreferred),
                            ));
                        }
                        if let Some(value) = romfile {
                            entries.push((
                                "romfile".to_string(),
                                raw_node(scalar_string(value), StyleHint::FlowPreferred),
                            ));
                        }
                        mapping_node(entries, StyleHint::FlowPreferred)
                    }
                    PcieDeviceResource::Address { .. } => {
                        raw_node(from_serde(pcie), StyleHint::FlowPreferred)
                    }
                };

                mapping_node(
                    vec![
                        (
                            "id".to_string(),
                            raw_node(scalar_string(id), StyleHint::Block),
                        ),
                        ("pcie".to_string(), pcie_value),
                    ],
                    StyleHint::Block,
                )
            }
            Resource::PciDevice { .. } | Resource::UsbDevice { .. } => {
                raw_node(from_serde(self), StyleHint::FlowPreferred)
            }
        };

        StyledYaml {
            node: node.node,
            style: StyleHint::FlowPreferred,
        }
    }
}

impl ToStyledYaml for VirtualMachine {
    fn to_styled_yaml(&self) -> StyledYaml {
        let mut entries: Vec<(String, StyledYaml)> = Vec::new();

        entries.push((
            "machine".to_string(),
            raw_node(from_serde(&self.machine), StyleHint::FlowPreferred),
        ));

        if let Some(cpu) = &self.cpu {
            entries.push((
                "cpu".to_string(),
                raw_node(from_serde(cpu), StyleHint::FlowPreferred),
            ));
        }

        entries.push((
            "memory".to_string(),
            raw_node(from_serde(&self.memory), StyleHint::FlowPreferred),
        ));

        entries.push((
            "boot".to_string(),
            raw_node(from_serde(&self.boot), StyleHint::FlowPreferred),
        ));

        if let Some(value) = &self.smbios_uuid {
            entries.push((
                "smbios_uuid".to_string(),
                raw_node(scalar_string(value), StyleHint::FlowPreferred),
            ));
        }

        if let Some(value) = &self.vmgenid {
            entries.push((
                "vmgenid".to_string(),
                raw_node(scalar_string(value), StyleHint::FlowPreferred),
            ));
        }

        if let Some(tpm) = &self.tpm {
            extend_flattened_mapping(&mut entries, from_serde(tpm), StyleHint::FlowPreferred);
        }

        if let Some(display) = &self.display {
            entries.push((
                "display".to_string(),
                raw_node(from_serde(display), StyleHint::FlowPreferred),
            ));
        }

        if let Some(audio) = &self.audio {
            entries.push((
                "audio".to_string(),
                raw_node(from_serde(audio), StyleHint::FlowPreferred),
            ));
        }

        if let Some(guest_agent) = &self.guest_agent {
            entries.push((
                "guest_agent".to_string(),
                raw_node(from_serde(guest_agent), StyleHint::FlowPreferred),
            ));
        }

        let devices = self
            .devices
            .iter()
            .map(device_to_styled)
            .collect::<Vec<_>>();
        if devices.is_empty() {
            entries.push((
                "devices".to_string(),
                raw_node(YamlOwned::Sequence(vec![]), StyleHint::FlowPreferred),
            ));
        } else {
            entries.push((
                "devices".to_string(),
                sequence_node(devices, StyleHint::Block),
            ));
        }

        mapping_node(entries, StyleHint::Block)
    }
}

fn device_to_styled(device: &Device) -> StyledYaml {
    raw_node(from_serde(device), StyleHint::FlowPreferred)
}

impl ToStyledYaml for Machine {
    fn to_styled_yaml(&self) -> StyledYaml {
        let mut entries = vec![
            (
                "family".to_string(),
                raw_node(scalar_string(&self.family), StyleHint::FlowPreferred),
            ),
            (
                "chipset".to_string(),
                raw_node(scalar_string(&self.chipset), StyleHint::FlowPreferred),
            ),
        ];
        if let Some(version) = &self.version {
            entries.push((
                "version".to_string(),
                raw_node(scalar_string(version), StyleHint::FlowPreferred),
            ));
        }

        StyledYaml {
            node: StyledNode::Mapping(entries),
            style: StyleHint::FlowPreferred,
        }
    }
}

pub fn emit_styled_yaml(styled: &StyledYaml) -> Result<String, String> {
    let rendered = render_with_style(styled, 0)?;
    Ok(format!("{}\n", rendered))
}

#[cfg(test)]
mod tests {
    use super::{ToStyledYaml, emit_styled_yaml};
    use crate::config_format::ezkvm::schema::EzkvmConfigSchema;

    const SAMPLE: &str = include_str!("../../../name.yaml");
    const BAKURA: &str = include_str!("../../../bakura_compact_improved.yaml");

    #[test]
    fn compact_output_keeps_resources_and_devices_on_one_line() {
        let schema: EzkvmConfigSchema = SAMPLE.parse().expect("sample yaml should parse");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        assert!(
            rendered.contains("- {id: storage0, storage: {block_device: /dev/vm1/vm-108-efidisk}}")
        );
        assert!(rendered.contains("- {pcie: {bus: 0, device: 0, function: 0, type: pv_scsi}}"));
        assert!(!rendered.contains("\n---"));
    }

    #[test]
    fn compact_output_round_trips_to_schema() {
        let schema: EzkvmConfigSchema = SAMPLE.parse().expect("sample yaml should parse");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        let reparsed: EzkvmConfigSchema = rendered
            .parse()
            .expect("rendered compact yaml should parse back");

        assert_eq!(reparsed.metadata.vm_name, schema.metadata.vm_name);
        assert_eq!(reparsed.host.resources.len(), schema.host.resources.len());
        assert_eq!(
            reparsed.virtual_machine.devices.len(),
            schema.virtual_machine.devices.len()
        );
    }

    #[test]
    fn compact_host_display_spice_renders_inline() {
        let schema: EzkvmConfigSchema = BAKURA.parse().expect("bakura yaml should parse");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        assert!(
            rendered.contains(
                "spice: {port: 5900, listen: 0.0.0.0, disable_ticketing: true, gl_enabled: false, seamless_migration: false}"
            ),
            "spice display should be inline; got:\n{rendered}"
        );
        assert!(
            rendered.contains("boot: {seabios: {firmware: \"\"}}"),
            "seabios boot should be inline; got:\n{rendered}"
        );
        assert!(
            rendered.contains("- {pcie: {bus: 0, device: 1, function: 0, type: standard_gpu}}"),
            "standard_gpu device should be inline; got:\n{rendered}"
        );
    }

    #[test]
    fn compact_host_display_spice_round_trips_to_schema() {
        let schema: EzkvmConfigSchema = BAKURA.parse().expect("bakura yaml should parse");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        let reparsed: EzkvmConfigSchema = rendered
            .parse()
            .expect("rendered compact yaml should parse back");

        assert_eq!(reparsed.metadata.vm_name, "bakura");
        assert_eq!(reparsed.host.resources.len(), schema.host.resources.len());
        assert_eq!(
            reparsed.virtual_machine.devices.len(),
            schema.virtual_machine.devices.len()
        );
        assert!(
            reparsed.host.display.is_some(),
            "host display should survive round-trip"
        );
    }

    #[test]
    fn compact_sata_ide_devices_render_inline() {
        const SATA_IDE_YAML: &str = r#"
metadata:
  schema_version: 1.0.0
  vm_name: test_vm
host:
  resources:
    - {id: storage0, storage: {file: /var/lib/vm/disk.qcow2}}
    - {id: net0, network: {tap: tap0}}
virtual_machine:
  machine: {family: pc, chipset: q35}
  memory: {size: 2147483648, numa_enabled: false}
  boot: {seabios: {firmware: ''}}
  devices:
    - {sata: {bus: 0, address: 0, type: hdd, resource: storage0}}
    - {ide: {bus: 0, address: 1, type: cdrom, resource: storage0}}
"#;
        let schema: EzkvmConfigSchema = SATA_IDE_YAML.parse().expect("sata/ide yaml should parse");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        assert!(
            rendered.contains("- {id: storage0, storage: {file: /var/lib/vm/disk.qcow2}}"),
            "file storage resource should be inline; got:\n{rendered}"
        );
        assert!(
            rendered.contains("- {id: net0, network: {tap: tap0}}"),
            "tap network resource should be inline; got:\n{rendered}"
        );
        assert!(
            rendered.contains("- {sata: {"),
            "sata device should be inline; got:\n{rendered}"
        );
        assert!(
            rendered.contains("- {ide: {"),
            "ide device should be inline; got:\n{rendered}"
        );
    }

    #[test]
    fn compact_sata_ide_round_trips_to_schema() {
        const SATA_IDE_YAML: &str = r#"
metadata:
  schema_version: 1.0.0
  vm_name: test_vm
host:
  resources:
    - {id: storage0, storage: {file: /var/lib/vm/disk.qcow2}}
    - {id: net0, network: {tap: tap0}}
virtual_machine:
  machine: {family: pc, chipset: q35}
  memory: {size: 2147483648, numa_enabled: false}
  boot: {seabios: {firmware: ''}}
  devices:
    - {sata: {bus: 0, address: 0, type: hdd, resource: storage0}}
    - {ide: {bus: 0, address: 1, type: cdrom, resource: storage0}}
"#;
        let schema: EzkvmConfigSchema = SATA_IDE_YAML.parse().expect("sata/ide yaml should parse");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        let reparsed: EzkvmConfigSchema = rendered
            .parse()
            .expect("rendered compact yaml should parse back");

        assert_eq!(reparsed.metadata.vm_name, "test_vm");
        assert_eq!(reparsed.host.resources.len(), 2);
        assert_eq!(reparsed.virtual_machine.devices.len(), 2);
    }

    #[test]
    fn compact_host_audio_renders_inline() {
        const AUDIO_YAML: &str = r#"
metadata:
  schema_version: 1.0.0
  vm_name: audio_vm
host:
  pipe_wire: {}
  resources:
    - {id: storage0, storage: {block_device: /dev/sda}}
virtual_machine:
  machine: {family: pc, chipset: q35}
  memory: {size: 2147483648, numa_enabled: false}
  boot: {seabios: {firmware: ''}}
  devices: []
"#;
        let schema: EzkvmConfigSchema = AUDIO_YAML.parse().expect("audio yaml should parse");

        assert!(schema.host.audio.is_some(), "host audio should be parsed");

        let rendered =
            emit_styled_yaml(&schema.to_styled_yaml()).expect("compact rendering should succeed");

        assert!(
            rendered.contains("pipe_wire: {}"),
            "pipe_wire audio should be rendered inline; got:\n{rendered}"
        );

        let reparsed: EzkvmConfigSchema =
            rendered.parse().expect("rendered yaml should parse back");

        assert!(
            reparsed.host.audio.is_some(),
            "host audio should survive round-trip"
        );
    }
}
