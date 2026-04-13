//! Network management for VMs
//!
//! Handles networking configuration including bridges, port forwarding, and isolation.

#![allow(dead_code)]

mod bridge;
mod firewall;
mod stats;
mod types;

#[allow(unused_imports)]
pub use bridge::{add_to_bridge, create_bridge, delete_bridge, list_bridges};
#[allow(unused_imports)]
pub use firewall::{add_port_forward, remove_port_forward, setup_network_isolation};
#[allow(unused_imports)]
pub use stats::get_network_stats;
#[allow(unused_imports)]
pub use types::{NetworkMode, NetworkStats, PortForwardRule};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_mode_display() {
        assert_eq!(NetworkMode::User.to_string(), "user");
        assert_eq!(NetworkMode::Bridge.to_string(), "bridge");
        assert_eq!(NetworkMode::Isolated.to_string(), "isolated");
    }

    #[test]
    fn test_port_forward_rule() {
        let rule = PortForwardRule {
            protocol: "tcp".to_string(),
            host_port: 8080,
            guest_port: 80,
            guest_ip: "192.168.1.10".to_string(),
        };
        assert_eq!(rule.host_port, 8080);
        assert_eq!(rule.guest_port, 80);
    }
}
