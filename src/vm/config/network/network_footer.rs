use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NetworkFooter {
    // bus=pci.<#n>,
    // addr=<#>.
    #[serde(default = "NetworkFooter::mac_default")]
    mac: String,
    /// Enable multiqueue networking. When > 1 this sets `queues=N` on the
    /// netdev and `vectors=<2*N+2>` on the device.
    #[serde(default)]
    queues: Option<u32>,
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
        if let Some(q) = self.queues.filter(|&q| q > 1) {
            result.push(format!("queues={}", q));
        }
        result.extend(self.extra_netdev_options.clone());
        result
    }

    pub fn get_device_options(&self, index: usize) -> Vec<String> {
        let mut result = vec![
            format!("netdev=netdev{}", index),
            format!("mac={}", self.mac),
        ];
        if let Some(q) = self.queues.filter(|&q| q > 1) {
            // virtio-net multiqueue: vectors = 2 * queues + 2
            result.push(format!("vectors={}", 2 * q + 2));
            result.push(format!("mq=on"));
        }
        result.extend(self.extra_device_options.clone());
        result
    }
}
