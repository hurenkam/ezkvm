use crate::config::types::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Cpu {
    #[serde(default = "default_cpu_model")]
    model: String,
    #[serde(default = "default_cpu_sockets")]
    sockets: u32,
    #[serde(default = "default_cpu_cores")]
    cores: u32,
    #[serde(default = "default_cpu_flags")]
    flags: String,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            model: default_cpu_model(),
            sockets: default_cpu_sockets(),
            cores: default_cpu_cores(),
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

fn default_cpu_flags() -> String {
    "+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce".to_string()
}

impl Cpu {
    pub fn new(model: String, sockets: u32, cores: u32, flags: String) -> Self {
        Self {
            model,
            sockets,
            cores,
            flags,
        }
    }
}

impl QemuDevice for Cpu {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let total = self.sockets * self.cores;
        vec![
            format!(
                "-smp {},sockets={},cores={},maxcpus={}",
                total, self.sockets, self.cores, total
            ),
            format!("-cpu {},{}", self.model, self.flags),
        ]
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
            cores: 6,
            flags: "my_flags".to_string(),
        };
        let expected: Vec<String> = vec![
            "-smp 12,sockets=2,cores=6,maxcpus=12".to_string(),
            "-cpu my_model,my_flags".to_string(),
        ];
        assert_eq!(cpu.get_qemu_args(0), expected);
    }
}
