use super::QemuDevice;
use serde::Deserialize;

/// A single NUMA node definition.
///
/// When `hugepages` is enabled at the system level, set `mem` on each node and
/// optionally `host_nodes`/`policy` for host-NUMA affinity. The System
/// coordinator will then generate `-object memory-backend-file,...` args and
/// use `memdev=ram-nodeN` in `-numa node` instead of `mem=`.
///
/// YAML example (plain mem mode):
/// ```yaml
/// numa_nodes:
///   - nodeid: 0
///     cpus: "0-3"
///     mem: 8192
/// ```
///
/// YAML example (hugepages mode – host-nodes and policy are optional):
/// ```yaml
/// numa_nodes:
///   - nodeid: 0
///     cpus: "0-11"
///     mem: 32768
///     host_nodes: 0
///     policy: bind
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
    /// Host NUMA node(s) for memory-backend affinity (e.g. `0`).
    #[serde(default)]
    pub host_nodes: Option<u32>,
    /// Memory policy: `bind`, `preferred`, `interleave`.
    #[serde(default)]
    pub policy: Option<String>,
}

impl NumaNode {
    /// Emit the `-object memory-backend-file` + `-numa node,memdev=` pair used
    /// when hugepages-backed NUMA is active.
    pub fn get_qemu_args_with_memdev(&self, index: usize, mem_path: &str) -> Vec<String> {
        let id = self.nodeid.unwrap_or(index as u32);
        let size = self.mem.unwrap_or(0);
        let backend_id = format!("ram-node{}", id);

        let mut obj_parts = vec![
            "memory-backend-file".to_string(),
            format!("id={}", backend_id),
            format!("size={}M", size),
            format!("mem-path={}", mem_path),
            "share=on".to_string(),
            "prealloc=yes".to_string(),
        ];
        if let Some(hn) = self.host_nodes {
            obj_parts.push(format!("host-nodes={}", hn));
        }
        if let Some(ref pol) = self.policy {
            obj_parts.push(format!("policy={}", pol));
        }

        let mut numa_parts = vec!["node".to_string(), format!("nodeid={}", id)];
        if let Some(ref cpus) = self.cpus {
            numa_parts.push(format!("cpus={}", cpus));
        }
        numa_parts.push(format!("memdev={}", backend_id));

        vec![
            format!("-object {}", obj_parts.join(",")),
            format!("-numa {}", numa_parts.join(",")),
        ]
    }
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
            host_nodes: None,
            policy: None,
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
            host_nodes: None,
            policy: None,
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

    #[test]
    fn test_numa_node_with_memdev_basic() {
        let node = NumaNode {
            nodeid: Some(0),
            cpus: Some("0-11".to_string()),
            mem: Some(32768),
            host_nodes: None,
            policy: None,
        };
        let args = node.get_qemu_args_with_memdev(0, "/run/hugepages/kvm/1048576kB");
        assert_eq!(args.len(), 2);
        assert_eq!(
            args[0],
            "-object memory-backend-file,id=ram-node0,size=32768M,mem-path=/run/hugepages/kvm/1048576kB,share=on,prealloc=yes"
        );
        assert_eq!(args[1], "-numa node,nodeid=0,cpus=0-11,memdev=ram-node0");
    }

    #[test]
    fn test_numa_node_with_memdev_host_affinity() {
        let node = NumaNode {
            nodeid: Some(0),
            cpus: Some("0-11".to_string()),
            mem: Some(32768),
            host_nodes: Some(0),
            policy: Some("bind".to_string()),
        };
        let args = node.get_qemu_args_with_memdev(0, "/run/hugepages/kvm/1048576kB");
        assert_eq!(
            args[0],
            "-object memory-backend-file,id=ram-node0,size=32768M,mem-path=/run/hugepages/kvm/1048576kB,share=on,prealloc=yes,host-nodes=0,policy=bind"
        );
        assert_eq!(args[1], "-numa node,nodeid=0,cpus=0-11,memdev=ram-node0");
    }

    #[test]
    fn test_numa_node_host_nodes_deserialize() {
        let yaml = r#"
            nodeid: 0
            cpus: "0-11"
            mem: 32768
            host_nodes: 0
            policy: bind
        "#;
        let node: NumaNode = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(node.host_nodes, Some(0));
        assert_eq!(node.policy.as_deref(), Some("bind"));
    }
}
