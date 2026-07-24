use crate::{
    config::ezkvm::schema::{
        AudioSchema, ConfigSchema, DeviceSchema, DisplaySchema, HostSchema, MachineSchema,
        MetadataSchema, NetworkResourceSchema, PcieResourceSchema, ResourceSchema,
        StorageResourceSchema, VirtualMachineSchema,
    },
    serde_yaml,
};
use saphyr::{ScalarOwned, Yaml, YamlEmitter, YamlOwned};
use serde::Serialize;
/*
use crate::config_format::ezkvm::schema::{
    AudioSchema, Device, DisplaySchema, EzkvmConfigSchema, HostSchema, Machine, MetadataSchema,
    VirtualMachine,
};
use crate::runtime_model::{NetworkResource, PcieDeviceResource, Resource, StorageResource};
*/
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

                if matches!(&value.node, StyledNode::Sequence(items) if items.is_empty()) {
                    out.push(format!("{}{}: []", " ".repeat(indent), key));
                    continue;
                }

                let nested = render_with_style(value, indent)?;
                let is_block_child = matches!(
                    value.node,
                    StyledNode::Mapping(_) | StyledNode::Sequence(_)
                );
                if nested.contains('\n') || is_block_child {
                    out.push(format!("{}{}:", " ".repeat(indent), key));
                    out.push(indent_lines(&nested, indent + 2));
                } else {
                    out.push(format!("{}{}: {}", " ".repeat(indent), key, nested));
                }

                if let Some(sep) = seperator {
                    out.push(sep.to_string());
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

                if let Some(sep) = seperator {
                    out.push(sep.to_string());
                }
            }
            Ok(out.join("\n"))
        }
    }
}

impl ToStyledYaml for ConfigSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        mapping_node(
            vec![
                ("metadata".to_string(), self.metadata().to_styled_yaml()),
                ("host".to_string(), self.host().to_styled_yaml()),
                (
                    "virtual_machine".to_string(),
                    self.virtual_machine().to_styled_yaml(),
                ),
            ],
            StyleHint::TopBlock,
        )
    }
}

impl ToStyledYaml for MetadataSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        mapping_node(
            vec![
                (
                    "schema_version".to_string(),
                    raw_node(scalar_string(self.schema_version()), StyleHint::Block),
                ),
                (
                    "vm_name".to_string(),
                    raw_node(scalar_string(self.vm_name()), StyleHint::Block),
                ),
            ],
            StyleHint::Block,
        )
    }
}

impl ToStyledYaml for HostSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        let mut entries: Vec<(String, StyledYaml)> = Vec::new();

        if let Some(display) = self.display() {
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

        if let Some(audio) = self.audio() {
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
            .resources()
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

impl ToStyledYaml for ResourceSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        let node = match self {
            ResourceSchema::Storage { id, storage } => {
                let storage_value = match storage {
                    StorageResourceSchema::File { file } => mapping_node(
                        vec![(
                            "file".to_string(),
                            raw_node(scalar_string(file), StyleHint::FlowPreferred),
                        )],
                        StyleHint::FlowPreferred,
                    ),
                    StorageResourceSchema::BlockDevice { block_device } => mapping_node(
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
            ResourceSchema::Network { id, network } => {
                let network_value = match network {
                    NetworkResourceSchema::Tap { tap } => mapping_node(
                        vec![(
                            "tap".to_string(),
                            raw_node(scalar_string(tap), StyleHint::FlowPreferred),
                        )],
                        StyleHint::FlowPreferred,
                    ),
                    NetworkResourceSchema::Bridge { bridge } => mapping_node(
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
            ResourceSchema::PcieDevice { id, pcie } => {
                let pcie_value = match pcie {
                    PcieResourceSchema::HostAddress {
                        address,
                        functions,
                        rombar,
                        romfile,
                    } => {
                        let mut entries = vec![(
                            "address".to_string(),
                            raw_node(scalar_string(address), StyleHint::FlowPreferred),
                        )];
                        if !functions.is_empty() {
                            entries.push((
                                "functions".to_string(),
                                raw_node(from_serde(functions), StyleHint::FlowPreferred),
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
                    PcieResourceSchema::Address { .. } => {
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
            ResourceSchema::PciDevice { .. } | ResourceSchema::UsbDevice { .. } | ResourceSchema::Memory { .. } => {
                raw_node(from_serde(self), StyleHint::FlowPreferred)
            }
        };

        StyledYaml {
            node: node.node,
            style: StyleHint::FlowPreferred,
        }
    }
}

impl ToStyledYaml for VirtualMachineSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        let mut entries: Vec<(String, StyledYaml)> = Vec::new();

        entries.push((
            "machine".to_string(),
            raw_node(from_serde(self.machine()), StyleHint::FlowPreferred),
        ));

        if let Some(cpu) = self.cpu() {
            entries.push((
                "cpu".to_string(),
                raw_node(from_serde(cpu), StyleHint::FlowPreferred),
            ));
        }

        entries.push((
            "memory".to_string(),
            raw_node(from_serde(self.memory()), StyleHint::FlowPreferred),
        ));

        entries.push((
            "boot".to_string(),
            raw_node(from_serde(self.boot()), StyleHint::FlowPreferred),
        ));

        if let Some(value) = self.smbios_uuid() {
            entries.push((
                "smbios_uuid".to_string(),
                raw_node(scalar_string(value), StyleHint::FlowPreferred),
            ));
        }

        if let Some(value) = self.vmgenid() {
            entries.push((
                "vmgenid".to_string(),
                raw_node(scalar_string(value), StyleHint::FlowPreferred),
            ));
        }

        if let Some(tpm) = self.tpm() {
            extend_flattened_mapping(&mut entries, from_serde(tpm), StyleHint::FlowPreferred);
        }

        if let Some(audio_device) = self.audio_device() {
            entries.push((
                "audio_device".to_string(),
                raw_node(from_serde(audio_device), StyleHint::FlowPreferred),
            ));
        }

        if let Some(raw_args) = self.raw_args() {
            entries.push((
                "raw_args".to_string(),
                raw_node(from_serde(raw_args), StyleHint::FlowPreferred),
            ));
        }

        if let Some(guest_agent) = self.guest_agent() {
            entries.push((
                "guest_agent".to_string(),
                raw_node(from_serde(guest_agent), StyleHint::FlowPreferred),
            ));
        }

        let devices = self
            .devices()
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

fn device_to_styled(device: &DeviceSchema) -> StyledYaml {
    raw_node(from_serde(device), StyleHint::FlowPreferred)
}

impl ToStyledYaml for MachineSchema {
    fn to_styled_yaml(&self) -> StyledYaml {
        let mut entries = vec![(
            "chipset".to_string(),
            raw_node(from_serde(self.chipset()), StyleHint::FlowPreferred),
        )];
        if let Some(version) = self.version() {
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
