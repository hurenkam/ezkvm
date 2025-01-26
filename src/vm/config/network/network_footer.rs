use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NetworkFooter {
    // bus=pci.<#n>,
    // addr=<#>.
    #[serde(default = "NetworkFooter::mac_default")]
    mac: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_netdev_options: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_device_options: Vec<String>,
}

impl NetworkFooter {
    pub fn mac_default() -> String {
        let mut rng = rand::thread_rng();
        let mut result: Vec<String> = vec![];
        for _ in 0..=5 {
            result.push(format!("{:02X}", rng.gen_range(0..=255)));
        }
        result.join(":")
    }

    pub fn get_netdev_options(&self, index: usize) -> Vec<String> {
        let mut result = vec![format!("id=netdev{}", index)];
        result.extend(self.extra_netdev_options.clone());
        result
    }

    pub fn get_device_options(&self, index: usize) -> Vec<String> {
        let mut result = vec![
            format!("netdev=netdev{}", index),
            format!("mac={}", self.mac),
        ];
        result.extend(self.extra_device_options.clone());
        result
    }
}
