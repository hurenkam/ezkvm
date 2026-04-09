use super::QemuDevice;
use serde::Deserialize;

/// A single NUMA node definition.
///
/// YAML example:
/// ```yaml
/// numa_nodes:
///   - nodeid: 0
///     cpus: "0-3"
///     mem: 8192
///   - nodeid: 1
///     cpus: "4-7"
///     mem: 8192
///
/// numa_distances:
///   - src: 0
///     dst: 1
///     val: 20
///   - src: 1
///     dst: 0
///     val: 20
/// ```
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct NumaNode {
    #[serde(default)]
    pub nodeid: Option<u32>,
    /// CPU range string, e.g. "0-3" or "4".
    #[serde(default)]
    pub cpus: Option<String>,
    /// Memory in MiB assigned to this node.
    #[serde(default)]
    pub mem: Option<u32>,
}

impl QemuDevice for NumaNode {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut parts = vec!["node".to_string()];
        if let Some(id) = self.nodeid {
            parts.push(format!("nodeid={}", id));
        }
        if let Some(ref cpus) = self.cpus {
            parts.push(format!("cpus={}", cpus));
        }
        if let Some(mem) = self.mem {
            parts.push(format!("mem={}M", mem));
        }
        vec![format!("-numa {}", parts.join(","))]
    }
}

/// NUMA inter-node distance entry.
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct NumaDistance {
    pub src: u32,
    pub dst: u32,
    pub val: u32,
}

impl QemuDevice for NumaDistance {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![format!(
            "-numa dist,src={},dst={},val={}",
            self.src, self.dst, self.val
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_node_full() {
        let node = NumaNode {
            nodeid: Some(0),
            cpus: Some("0-3".to_string()),
            mem: Some(8192),
        };
        assert_eq!(
            node.get_qemu_args(0),
            vec!["-numa node,nodeid=0,cpus=0-3,mem=8192M".to_string()]
        );
    }

    #[test]
    fn test_numa_node_minimal() {
        let node = NumaNode {
            nodeid: None,
            cpus: None,
            mem: None,
        };
        assert_eq!(node.get_qemu_args(0), vec!["-numa node".to_string()]);
    }

    #[test]
    fn test_numa_distance() {
        let dist = NumaDistance { src: 0, dst: 1, val: 20 };
        assert_eq!(
            dist.get_qemu_args(0),
            vec!["-numa dist,src=0,dst=1,val=20".to_string()]
        );
    }

    #[test]
    fn test_numa_node_from_yaml() {
        let yaml = r#"
            nodeid: 1
            cpus: "4-7"
            mem: 4096
        "#;
        let node: NumaNode = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            node.get_qemu_args(0),
            vec!["-numa node,nodeid=1,cpus=4-7,mem=4096M".to_string()]
        );
    }
}
