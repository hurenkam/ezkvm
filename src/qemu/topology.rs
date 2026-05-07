use crate::qemu::types::QemuArgs;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeSource {
    ReadConfig,
    CommandArg,
}

#[derive(Debug, Clone)]
struct TopologyNode {
    id: String,
    driver: String,
    bus: Option<String>,
    addr: Option<String>,
    source: NodeSource,
    explicit_id: bool,
}

pub fn render_machine_layout(
    machine: &str,
    args: &QemuArgs,
    readconfig_paths: &[String],
) -> String {
    let mut nodes = readconfig_paths
        .iter()
        .flat_map(|path| parse_readconfig_devices(path))
        .collect::<Vec<_>>();
    nodes.extend(parse_command_device_nodes(args));

    let mut lines = vec![format!("machine {}", machine)];
    if nodes.is_empty() {
        lines.push("\\- (no devices found in generated command)".to_string());
        return lines.join("\n");
    }

    let bus_to_children = build_bus_to_children(&nodes);
    let root_buses = detect_root_buses(&nodes);
    let top_level_devices = nodes
        .iter()
        .enumerate()
        .filter_map(
            |(idx, node)| {
                if node.bus.is_none() { Some(idx) } else { None }
            },
        )
        .collect::<Vec<_>>();

    let mut top_entries = Vec::new();
    for bus in &root_buses {
        top_entries.push(TopEntry::Bus(bus.clone()));
    }
    for idx in top_level_devices {
        top_entries.push(TopEntry::Device(idx));
    }

    if top_entries.is_empty() {
        lines.push("\\- (no root buses discovered)".to_string());
        return lines.join("\n");
    }

    let mut visited_buses = HashSet::new();
    for (position, entry) in top_entries.iter().enumerate() {
        let is_last = position + 1 == top_entries.len();
        match entry {
            TopEntry::Bus(bus) => {
                lines.push(tree_line("", is_last, &format!("bus {}", bus)));
                render_bus_children(
                    bus,
                    &nodes,
                    &bus_to_children,
                    &mut visited_buses,
                    &mut lines,
                    child_prefix("", is_last),
                );
            }
            TopEntry::Device(idx) => {
                let node = &nodes[*idx];
                lines.push(tree_line("", is_last, &format_node(node)));
                let next_prefix = child_prefix("", is_last);
                for child_bus in produced_bus_names(node) {
                    if bus_to_children.contains_key(&child_bus) {
                        render_bus_children(
                            &child_bus,
                            &nodes,
                            &bus_to_children,
                            &mut visited_buses,
                            &mut lines,
                            next_prefix.clone(),
                        );
                    }
                }
            }
        }
    }

    lines.join("\n")
}

#[derive(Debug, Clone)]
enum TopEntry {
    Bus(String),
    Device(usize),
}

fn parse_command_device_nodes(args: &QemuArgs) -> Vec<TopologyNode> {
    let mut nodes = Vec::new();
    let mut idx = 0usize;
    let mut fallback_counter = 0usize;

    while idx < args.len() {
        if args[idx] == "-device"
            && let Some(spec) = args.get(idx + 1)
        {
            nodes.push(parse_device_spec(
                spec,
                NodeSource::CommandArg,
                &mut fallback_counter,
            ));
            idx += 2;
            continue;
        }

        idx += 1;
    }

    nodes
}

fn parse_readconfig_devices(path: &str) -> Vec<TopologyNode> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut nodes = Vec::new();
    let mut current: Option<RawReadConfigDevice> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = parse_device_section_name(line) {
            if let Some(device) = current.take() {
                nodes.push(device.into_node());
            }
            current = Some(RawReadConfigDevice::new(name));
            continue;
        }

        let Some(device) = current.as_mut() else {
            continue;
        };

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = unquote(value.trim());
        match key {
            "driver" => device.driver = Some(value),
            "bus" => device.bus = Some(value),
            "addr" => device.addr = Some(value),
            _ => {}
        }
    }

    if let Some(device) = current.take() {
        nodes.push(device.into_node());
    }

    nodes
}

#[derive(Debug, Clone)]
struct RawReadConfigDevice {
    id: String,
    driver: Option<String>,
    bus: Option<String>,
    addr: Option<String>,
}

impl RawReadConfigDevice {
    fn new(id: String) -> Self {
        Self {
            id,
            driver: None,
            bus: None,
            addr: None,
        }
    }

    fn into_node(self) -> TopologyNode {
        TopologyNode {
            id: self.id,
            driver: self.driver.unwrap_or_else(|| "device".to_string()),
            bus: self.bus,
            addr: self.addr,
            source: NodeSource::ReadConfig,
            explicit_id: true,
        }
    }
}

fn parse_device_section_name(line: &str) -> Option<String> {
    if !line.starts_with("[device \"") || !line.ends_with("\"]") {
        return None;
    }

    Some(line[9..line.len() - 2].to_string())
}

fn parse_device_spec(spec: &str, source: NodeSource, fallback_counter: &mut usize) -> TopologyNode {
    let mut parts = spec.split(',').map(str::trim).filter(|p| !p.is_empty());
    let first = parts.next().unwrap_or("device");

    let mut driver = first.to_string();
    let mut id = None;
    let mut bus = None;
    let mut addr = None;

    if let Some((key, value)) = first.split_once('=')
        && key.trim() == "driver"
    {
        driver = unquote(value.trim());
    }

    for token in parts {
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = unquote(value.trim());
        match key {
            "id" => id = Some(value),
            "bus" => bus = Some(value),
            "addr" => addr = Some(value),
            "driver" => driver = value,
            _ => {}
        }
    }

    let explicit_id = id.is_some();
    let id = id.unwrap_or_else(|| {
        *fallback_counter += 1;
        format!("{}#{}", driver, fallback_counter)
    });

    TopologyNode {
        id,
        driver,
        bus,
        addr,
        source,
        explicit_id,
    }
}

fn unquote(value: &str) -> String {
    value.trim_matches('"').to_string()
}

fn build_bus_to_children(nodes: &[TopologyNode]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        if let Some(bus) = &node.bus {
            map.entry(bus.clone()).or_default().push(idx);
        }
    }

    for children in map.values_mut() {
        children.sort_by(|left, right| {
            node_sort_key(&nodes[*left]).cmp(&node_sort_key(&nodes[*right]))
        });
    }

    map
}

fn detect_root_buses(nodes: &[TopologyNode]) -> Vec<String> {
    let mut referenced_buses = BTreeSet::new();
    let mut produced_buses = HashSet::new();

    for node in nodes {
        if let Some(bus) = &node.bus {
            referenced_buses.insert(bus.clone());
        }

        if node.explicit_id {
            produced_buses.insert(node.id.clone());
            produced_buses.insert(format!("{}.0", node.id));
        }
    }

    referenced_buses
        .into_iter()
        .filter(|bus| !produced_buses.contains(bus))
        .collect()
}

fn render_bus_children(
    bus: &str,
    nodes: &[TopologyNode],
    bus_to_children: &HashMap<String, Vec<usize>>,
    visited_buses: &mut HashSet<String>,
    lines: &mut Vec<String>,
    prefix: String,
) {
    if !visited_buses.insert(bus.to_string()) {
        return;
    }

    let Some(children) = bus_to_children.get(bus) else {
        return;
    };

    for (position, child_idx) in children.iter().enumerate() {
        let is_last_child = position + 1 == children.len();
        let node = &nodes[*child_idx];
        lines.push(tree_line(&prefix, is_last_child, &format_node(node)));

        let child_prefix = child_prefix(&prefix, is_last_child);
        for child_bus in produced_bus_names(node) {
            if bus_to_children.contains_key(&child_bus) {
                render_bus_children(
                    &child_bus,
                    nodes,
                    bus_to_children,
                    visited_buses,
                    lines,
                    child_prefix.clone(),
                );
            }
        }
    }
}

fn produced_bus_names(node: &TopologyNode) -> Vec<String> {
    if !node.explicit_id {
        return Vec::new();
    }

    vec![node.id.clone(), format!("{}.0", node.id)]
}

fn format_node(node: &TopologyNode) -> String {
    let mut details = vec![node.driver.clone()];
    if let Some(addr) = &node.addr {
        details.push(format!("@{}", addr));
    }
    if node.source == NodeSource::ReadConfig {
        details.push("readconfig".to_string());
    }

    format!("{} ({})", node.id, details.join(" "))
}

fn node_sort_key(node: &TopologyNode) -> (String, String) {
    (node.addr.clone().unwrap_or_default(), node.id.clone())
}

fn tree_line(prefix: &str, is_last: bool, text: &str) -> String {
    let branch = if is_last { "\\- " } else { "|- " };
    format!("{}{}{}", prefix, branch, text)
}

fn child_prefix(prefix: &str, is_last: bool) -> String {
    let branch = if is_last { "   " } else { "|  " };
    format!("{}{}", prefix, branch)
}

#[cfg(test)]
mod tests {
    use super::{NodeSource, parse_device_spec, parse_readconfig_devices, render_machine_layout};
    use crate::config::{CentralConfig, RuntimeCliOverrides, VmConfig};
    use crate::qemu::QemuManager;
    use crate::qemu::types::QemuArgs;
    use std::io::Write;

    fn write_temp_readconfig(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "ezkvm-q35-topology-{}.cfg",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("timestamp should be after UNIX epoch")
                .as_nanos()
        ));

        let mut file = std::fs::File::create(&path).expect("should create temp readconfig file");
        file.write_all(contents.as_bytes())
            .expect("should write temp readconfig file");
        path.to_string_lossy().to_string()
    }

    #[test]
    fn parse_device_spec_extracts_id_bus_and_addr() {
        let mut fallback = 0usize;
        let node = parse_device_spec(
            "vfio-pci,host=0000:03:00.0,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0.0",
            NodeSource::CommandArg,
            &mut fallback,
        );

        assert_eq!(node.id, "hostpci0");
        assert_eq!(node.driver, "vfio-pci");
        assert_eq!(node.bus.as_deref(), Some("ich9-pcie-port-1"));
        assert_eq!(node.addr.as_deref(), Some("0x0.0"));
    }

    #[test]
    fn readconfig_devices_are_parsed_from_ini_sections() {
        let readconfig = write_temp_readconfig(
            r#"
[device "ich9-pcie-port-1"]
driver = "pcie-root-port"
bus = "pcie.0"
addr = "1c.0"
"#,
        );

        let nodes = parse_readconfig_devices(&readconfig);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, "ich9-pcie-port-1");
        assert_eq!(nodes[0].driver, "pcie-root-port");
        assert_eq!(nodes[0].bus.as_deref(), Some("pcie.0"));
        assert_eq!(nodes[0].addr.as_deref(), Some("1c.0"));
    }

    #[test]
    fn render_machine_layout_links_devices_under_parent_bus() {
        let args = QemuArgs::from_vec(vec![
            "-device".to_string(),
            "pcie-root-port,id=ich9-pcie-port-1,bus=pcie.0,addr=1c.0".to_string(),
            "-device".to_string(),
            "vfio-pci,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0.0".to_string(),
        ]);

        let tree = render_machine_layout("q35", &args, &[]);
        assert!(tree.contains("machine q35"));
        assert!(tree.contains("bus pcie.0"));
        assert!(tree.contains("ich9-pcie-port-1 (pcie-root-port @1c.0)"));
        assert!(tree.contains("hostpci0 (vfio-pci @0x0.0)"));
    }

    #[test]
    fn render_machine_layout_shows_children_of_top_level_devices() {
        let args = QemuArgs::from_vec(vec![
            "-device".to_string(),
            "qemu-xhci,id=xhci".to_string(),
            "-device".to_string(),
            "usb-host,id=usb0,bus=xhci.0,port=1".to_string(),
            "-device".to_string(),
            "pvscsi,id=scsihw0".to_string(),
            "-device".to_string(),
            "scsi-hd,id=scsi0,bus=scsihw0.0,scsi-id=0".to_string(),
        ]);

        let tree = render_machine_layout("q35", &args, &[]);
        assert!(tree.contains("xhci (qemu-xhci)"));
        assert!(tree.contains("usb0 (usb-host)"));
        assert!(tree.contains("scsihw0 (pvscsi)"));
        assert!(tree.contains("scsi0 (scsi-hd)"));
    }

    #[test]
    fn render_machine_layout_shows_default_hostpci_root_ports_from_built_command() {
        let readconfig = write_temp_readconfig(
            r#"
            [device "ich9-pcie-port-1"]
            driver = "pcie-root-port"
            bus = "pcie.0"
            addr = "1c.0"

            [device "ich9-pcie-port-2"]
            driver = "pcie-root-port"
            bus = "pcie.0"
            addr = "1d.0"
            "#,
        );

        let vm = VmConfig::from_str(&format!(
            r#"
            name: "vm-q35-topology-hostpci"
            backend: "qemu"

            system:
                architecture: "x86_64"
                machine: "q35"
                readconfig:
                    - "{readconfig}"
                memory:
                    size: 4096
                cpu:
                    vcpus: 2
                    model: "host"

            host:
                pci:
                  - device: "0000:07:00.0"
                    pcie: true
                  - device: "0000:41:00.0"
                    pcie: true
                    multifunction: true
                  - device: "0000:41:00.1"
            "#
        ))
        .expect("vm config should parse");

        let manager = QemuManager::new_with_overrides(
            vm,
            CentralConfig::default(),
            RuntimeCliOverrides::default(),
        );
        let command = manager.build_command().expect("qemu command should build");
        let tree = render_machine_layout(
            &manager.config.system.machine,
            &command,
            &manager.config.system.readconfig,
        );

        assert!(tree.contains("ich9-pcie-port-1 (pcie-root-port @1c.0 readconfig)"));
        assert!(tree.contains("hostpci0 (vfio-pci)"));
        assert!(tree.contains("ich9-pcie-port-2 (pcie-root-port @1d.0 readconfig)"));
        assert!(tree.contains("hostpci1 (vfio-pci @0x0.0)"));
        assert!(tree.contains("hostpci2 (vfio-pci @0x0.1)"));
    }
}
