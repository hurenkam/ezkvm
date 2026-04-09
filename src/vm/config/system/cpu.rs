use super::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Cpu {
    #[serde(default = "default_cpu_model")]
    model: String,
    #[serde(default = "default_cpu_sockets")]
    sockets: u32,
    #[serde(default)]
    dies: Option<u32>,
    #[serde(default)]
    clusters: Option<u32>,
    #[serde(default = "default_cpu_cores")]
    cores: u32,
    #[serde(default = "default_cpu_threads")]
    threads: u32,
    #[serde(default = "default_cpu_flags")]
    flags: String,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            model: default_cpu_model(),
            sockets: default_cpu_sockets(),
            dies: None,
            clusters: None,
            cores: default_cpu_cores(),
            threads: default_cpu_threads(),
            flags: default_cpu_flags(),
        }
    }
}

fn default_cpu_model() -> String {
    "qemu64".to_string()
}

fn default_cpu_sockets() -> u32 {
    1
}

fn default_cpu_cores() -> u32 {
    4
}

fn default_cpu_threads() -> u32 {
    1
}

fn default_cpu_flags() -> String {
    "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce".to_string()
}

impl Cpu {
    pub fn new(model: String, sockets: u32, cores: u32, flags: String) -> Self {
        Self {
            model,
            sockets,
            dies: None,
            clusters: None,
            cores,
            threads: default_cpu_threads(),
            flags,
        }
    }
}

impl QemuDevice for Cpu {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let dies = self.dies.unwrap_or(1);
        let clusters = self.clusters.unwrap_or(1);
        let total = self.sockets * dies * clusters * self.cores * self.threads;

        let mut smp_parts = vec![
            total.to_string(),
            format!("sockets={}", self.sockets),
        ];
        if let Some(d) = self.dies.filter(|&d| d > 1) {
            smp_parts.push(format!("dies={}", d));
        }
        if let Some(c) = self.clusters.filter(|&c| c > 1) {
            smp_parts.push(format!("clusters={}", c));
        }
        smp_parts.push(format!("cores={}", self.cores));
        if self.threads > 1 {
            smp_parts.push(format!("threads={}", self.threads));
        }
        smp_parts.push(format!("maxcpus={}", total));

        let cpu = if self.flags.trim().is_empty() {
            format!("-cpu {}", self.model)
        } else {
            format!("-cpu {},{}", self.model, self.flags)
        };

        vec![format!("-smp {}", smp_parts.join(",")), cpu]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let cpu: Cpu = serde_yaml::from_str("").unwrap();
        let expected: Vec<String> = vec![
            "-smp 4,sockets=1,cores=4,maxcpus=4".to_string(),
            "-cpu qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce".to_string(),
        ];
        assert_eq!(cpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_yaml_input() {
        let input = r#"
            model: "qemu64"
            sockets: 1
            cores: 8
            flags: "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce"
        "#;

        let cpu: Cpu = serde_yaml::from_str(input).unwrap();
        let expected: Vec<String> = vec![
            "-smp 8,sockets=1,cores=8,maxcpus=8".to_string(),
            "-cpu qemu64,+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce".to_string(),
        ];
        assert_eq!(cpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_get_qemu_args() {
        let cpu = Cpu {
            model: "my_model".to_string(),
            sockets: 2,
            dies: None,
            clusters: None,
            cores: 6,
            threads: 1,
            flags: "my_flags".to_string(),
        };
        let expected: Vec<String> = vec![
            "-smp 12,sockets=2,cores=6,maxcpus=12".to_string(),
            "-cpu my_model,my_flags".to_string(),
        ];
        assert_eq!(cpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_get_qemu_args_threads() {
        let cpu = Cpu {
            model: "host".to_string(),
            sockets: 2,
            dies: None,
            clusters: None,
            cores: 4,
            threads: 2,
            flags: "".to_string(),
        };
        let expected: Vec<String> = vec![
            "-smp 16,sockets=2,cores=4,threads=2,maxcpus=16".to_string(),
            "-cpu host".to_string(),
        ];
        assert_eq!(cpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_get_qemu_args_dies_and_clusters() {
        let cpu = Cpu {
            model: "host".to_string(),
            sockets: 2,
            dies: Some(2),
            clusters: Some(2),
            cores: 2,
            threads: 1,
            flags: "".to_string(),
        };
        let expected: Vec<String> = vec![
            "-smp 16,sockets=2,dies=2,clusters=2,cores=2,maxcpus=16".to_string(),
            "-cpu host".to_string(),
        ];
        assert_eq!(cpu.get_qemu_args(0), expected);
    }

    #[test]
    fn test_get_qemu_args_without_flags() {
        let cpu = Cpu {
            model: "host".to_string(),
            sockets: 1,
            dies: None,
            clusters: None,
            cores: 8,
            threads: 1,
            flags: "".to_string(),
        };

        let expected: Vec<String> = vec![
            "-smp 8,sockets=1,cores=8,maxcpus=8".to_string(),
            "-cpu host".to_string(),
        ];

        assert_eq!(cpu.get_qemu_args(0), expected);
    }
}
