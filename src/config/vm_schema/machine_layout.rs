use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MachineLayoutConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buses: Vec<MachineLayoutBus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<MachineLayoutNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MachineLayoutBus {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<MachineLayoutDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MachineLayoutDevice {
    pub id: String,
    pub driver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buses: Vec<MachineLayoutBus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MachineLayoutNode {
    pub id: String,
    pub kind: MachineLayoutNodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MachineLayoutNodeKind {
    Bus,
    Device,
}

#[derive(Debug, Clone)]
pub struct FlatPlacementEntry {
    pub id: String,
    pub driver: String,
    pub bus: Option<String>,
    pub addr: Option<String>,
}

impl MachineLayoutConfig {
    pub fn is_empty(&self) -> bool {
        self.buses.is_empty() && self.nodes.is_empty()
    }
}

pub(crate) fn normalize_declared_layout(
    layout: &MachineLayoutConfig,
) -> Result<MachineLayoutConfig> {
    if !layout.buses.is_empty() {
        validate_tree_layout(&layout.buses)?;
        return Ok(MachineLayoutConfig {
            buses: layout.buses.clone(),
            nodes: Vec::new(),
        });
    }

    if layout.nodes.is_empty() {
        return Ok(MachineLayoutConfig::default());
    }

    let buses = normalize_nodes_into_tree(&layout.nodes)?;
    Ok(MachineLayoutConfig {
        buses,
        nodes: Vec::new(),
    })
}

pub(crate) fn normalize_from_flat_placements(
    entries: &[FlatPlacementEntry],
) -> MachineLayoutConfig {
    if entries.is_empty() {
        return MachineLayoutConfig::default();
    }

    let mut bus_map: BTreeMap<String, Vec<MachineLayoutDevice>> = BTreeMap::new();
    for entry in entries {
        if entry.id.trim().is_empty() || entry.driver.trim().is_empty() {
            continue;
        }
        let bus = entry.bus.clone().unwrap_or_else(|| "pcie.0".to_string());
        let device = MachineLayoutDevice {
            id: entry.id.clone(),
            driver: entry.driver.clone(),
            addr: entry.addr.clone(),
            buses: Vec::new(),
        };
        bus_map.entry(bus).or_default().push(device);
    }

    let all_buses = bus_map.keys().cloned().collect::<Vec<_>>();
    let mut child_buses = HashSet::new();
    for devices in bus_map.values() {
        for device in devices {
            let prefix = format!("{}.", device.id);
            for bus in &all_buses {
                if bus.starts_with(&prefix) {
                    child_buses.insert(bus.clone());
                }
            }
        }
    }

    let mut roots = all_buses
        .into_iter()
        .filter(|bus| !child_buses.contains(bus))
        .collect::<Vec<_>>();
    if roots.is_empty() {
        roots.push("pcie.0".to_string());
    }

    let mut visiting = HashSet::new();
    let mut emitted = HashSet::new();
    let mut root_buses = Vec::new();
    for root in roots {
        build_bus_tree(
            &root,
            &bus_map,
            &mut visiting,
            &mut emitted,
            &mut root_buses,
        );
    }

    MachineLayoutConfig {
        buses: root_buses,
        nodes: Vec::new(),
    }
}

fn build_bus_tree(
    bus_id: &str,
    bus_map: &BTreeMap<String, Vec<MachineLayoutDevice>>,
    visiting: &mut HashSet<String>,
    emitted: &mut HashSet<String>,
    out: &mut Vec<MachineLayoutBus>,
) {
    if emitted.contains(bus_id) || !visiting.insert(bus_id.to_string()) {
        return;
    }

    let mut bus = MachineLayoutBus {
        id: bus_id.to_string(),
        devices: bus_map.get(bus_id).cloned().unwrap_or_default(),
    };

    for device in &mut bus.devices {
        let prefix = format!("{}.", device.id);
        let child_ids = bus_map
            .keys()
            .filter(|candidate| candidate.starts_with(&prefix))
            .cloned()
            .collect::<Vec<_>>();
        for child_id in child_ids {
            build_bus_tree(&child_id, bus_map, visiting, emitted, &mut device.buses);
        }
    }

    visiting.remove(bus_id);
    emitted.insert(bus_id.to_string());
    out.push(bus);
}

fn validate_tree_layout(buses: &[MachineLayoutBus]) -> Result<()> {
    let mut ids = HashSet::new();
    let mut bus_path = HashSet::new();
    for bus in buses {
        validate_bus(bus, &mut ids, &mut bus_path)?;
    }
    Ok(())
}

fn validate_bus(
    bus: &MachineLayoutBus,
    ids: &mut HashSet<String>,
    bus_path: &mut HashSet<String>,
) -> Result<()> {
    if bus.id.trim().is_empty() {
        return Err(anyhow!("machine_layout contains bus with empty id"));
    }
    let key = format!("bus:{}", bus.id);
    if !ids.insert(key.clone()) {
        return Err(anyhow!("machine_layout duplicate id detected: {}", bus.id));
    }
    if !bus_path.insert(bus.id.clone()) {
        return Err(anyhow!(
            "machine_layout cycle detected through bus: {}",
            bus.id
        ));
    }

    for device in &bus.devices {
        validate_device(device, ids, bus_path)?;
    }

    bus_path.remove(&bus.id);
    Ok(())
}

fn validate_device(
    device: &MachineLayoutDevice,
    ids: &mut HashSet<String>,
    bus_path: &mut HashSet<String>,
) -> Result<()> {
    if device.id.trim().is_empty() {
        return Err(anyhow!("machine_layout contains device with empty id"));
    }
    if device.driver.trim().is_empty() {
        return Err(anyhow!(
            "machine_layout device '{}' has empty driver",
            device.id
        ));
    }

    let key = format!("device:{}", device.id);
    if !ids.insert(key) {
        return Err(anyhow!(
            "machine_layout duplicate id detected: {}",
            device.id
        ));
    }

    for bus in &device.buses {
        validate_bus(bus, ids, bus_path)?;
    }
    Ok(())
}

fn normalize_nodes_into_tree(nodes: &[MachineLayoutNode]) -> Result<Vec<MachineLayoutBus>> {
    let mut by_id: HashMap<String, &MachineLayoutNode> = HashMap::new();
    for node in nodes {
        if node.id.trim().is_empty() {
            return Err(anyhow!("machine_layout.nodes contains node with empty id"));
        }
        if by_id.insert(node.id.clone(), node).is_some() {
            return Err(anyhow!("machine_layout duplicate id detected: {}", node.id));
        }
    }

    for node in nodes {
        if let Some(parent_id) = &node.parent_id {
            if !by_id.contains_key(parent_id) {
                return Err(anyhow!(
                    "machine_layout broken parent link: '{}' references missing parent '{}'",
                    node.id,
                    parent_id
                ));
            }
            let parent = by_id[parent_id];
            match (&parent.kind, &node.kind) {
                (MachineLayoutNodeKind::Bus, MachineLayoutNodeKind::Device)
                | (MachineLayoutNodeKind::Device, MachineLayoutNodeKind::Bus) => {}
                _ => {
                    return Err(anyhow!(
                        "machine_layout invalid parent-child kinds: '{}' ({:?}) -> '{}' ({:?})",
                        parent.id,
                        parent.kind,
                        node.id,
                        node.kind
                    ));
                }
            }
        }
    }

    detect_node_cycles(nodes, &by_id)?;

    let mut children: HashMap<String, Vec<&MachineLayoutNode>> = HashMap::new();
    for node in nodes {
        if let Some(parent_id) = &node.parent_id {
            children.entry(parent_id.clone()).or_default().push(node);
        }
    }

    let mut roots = nodes
        .iter()
        .filter(|node| node.parent_id.is_none())
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.id.cmp(&right.id));

    let mut buses = Vec::new();
    for root in roots {
        if root.kind != MachineLayoutNodeKind::Bus {
            return Err(anyhow!(
                "machine_layout root node '{}' must be a bus",
                root.id
            ));
        }
        buses.push(build_bus_from_node(root, &children)?);
    }

    Ok(buses)
}

fn detect_node_cycles(
    nodes: &[MachineLayoutNode],
    by_id: &HashMap<String, &MachineLayoutNode>,
) -> Result<()> {
    for node in nodes {
        let mut seen = HashSet::new();
        let mut cursor = Some(node.id.clone());
        while let Some(id) = cursor {
            if !seen.insert(id.clone()) {
                return Err(anyhow!(
                    "machine_layout cycle detected involving node '{}'",
                    id
                ));
            }
            cursor = by_id.get(&id).and_then(|current| current.parent_id.clone());
        }
    }
    Ok(())
}

fn build_bus_from_node(
    bus_node: &MachineLayoutNode,
    children: &HashMap<String, Vec<&MachineLayoutNode>>,
) -> Result<MachineLayoutBus> {
    let mut bus = MachineLayoutBus {
        id: bus_node.id.clone(),
        devices: Vec::new(),
    };
    if let Some(bus_children) = children.get(&bus_node.id) {
        let mut sorted = bus_children.clone();
        sorted.sort_by(|left, right| left.id.cmp(&right.id));
        for child in sorted {
            if child.kind != MachineLayoutNodeKind::Device {
                return Err(anyhow!(
                    "machine_layout bus '{}' can only contain devices",
                    bus.id
                ));
            }
            bus.devices.push(build_device_from_node(child, children)?);
        }
    }
    Ok(bus)
}

fn build_device_from_node(
    device_node: &MachineLayoutNode,
    children: &HashMap<String, Vec<&MachineLayoutNode>>,
) -> Result<MachineLayoutDevice> {
    let driver = device_node
        .driver
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| anyhow!("machine_layout device '{}' requires driver", device_node.id))?
        .clone();
    let mut device = MachineLayoutDevice {
        id: device_node.id.clone(),
        driver,
        addr: device_node.addr.clone(),
        buses: Vec::new(),
    };

    if let Some(device_children) = children.get(&device_node.id) {
        let mut sorted = device_children.clone();
        sorted.sort_by(|left, right| left.id.cmp(&right.id));
        for child in sorted {
            if child.kind != MachineLayoutNodeKind::Bus {
                return Err(anyhow!(
                    "machine_layout device '{}' can only contain buses",
                    device.id
                ));
            }
            device.buses.push(build_bus_from_node(child, children)?);
        }
    }

    Ok(device)
}
