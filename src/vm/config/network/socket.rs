use super::network_payload::{network_pci_addr, NetworkPayload};
use crate::required_value_getter;
use paste::paste;
use serde::{Deserialize, Serialize};

/// UDP socket (peer-to-peer) network backend.
///
/// Connects two QEMU instances via UDP sockets.  Useful for isolated
/// multi-VM testbeds without a real bridge.
///
/// YAML example (VM side A):
/// ```yaml
/// network:
///   - type: socket
///     localaddr: "127.0.0.1"
///     localport: 11001
///     remoteaddr: "127.0.0.1"
///     remoteport: 11002
/// ```
///
/// Alternatively use `listen` / `connect` for a star topology:
/// ```yaml
/// network:
///   - type: socket
///     listen: "127.0.0.1:12000"
/// ```
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SocketNet {
    #[serde(default = "SocketNet::driver_default")]
    driver: String,
    /// Local UDP address for peer-to-peer mode.
    #[serde(default)]
    localaddr: Option<String>,
    /// Local UDP port for peer-to-peer mode.
    #[serde(default)]
    localport: Option<u16>,
    /// Remote peer UDP address.
    #[serde(default)]
    remoteaddr: Option<String>,
    /// Remote peer UDP port.
    #[serde(default)]
    remoteport: Option<u16>,
    /// TCP listen address:port (listener side).
    #[serde(default)]
    listen: Option<String>,
    /// TCP connect address:port (client side).
    #[serde(default)]
    connect: Option<String>,
}

impl SocketNet {
    required_value_getter!(driver("driver"): String = "virtio-net-pci".to_string());
}

#[typetag::deserialize(name = "socket")]
impl NetworkPayload for SocketNet {
    fn get_netdev_options(&self, _index: usize) -> Vec<String> {
        let mut parts = vec!["type=socket".to_string()];
        if let (Some(ref la), Some(lp), Some(ref ra), Some(rp)) = (
            &self.localaddr,
            self.localport,
            &self.remoteaddr,
            self.remoteport,
        ) {
            parts.push(format!("localaddr={}:{}", la, lp));
            parts.push(format!("remoteaddr={}:{}", ra, rp));
        } else if let Some(ref listen) = self.listen {
            parts.push(format!("listen={}", listen));
        } else if let Some(ref connect) = self.connect {
            parts.push(format!("connect={}", connect));
        }
        vec![parts.join(",")]
    }

    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![format!(
            "{},id=net{},bus=pci.1,addr={}",
            self.driver,
            index,
            network_pci_addr(index)
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_peer() {
        let net = SocketNet {
            driver: SocketNet::driver_default(),
            localaddr: Some("127.0.0.1".to_string()),
            localport: Some(11001),
            remoteaddr: Some("127.0.0.1".to_string()),
            remoteport: Some(11002),
            listen: None,
            connect: None,
        };
        let opts = net.get_netdev_options(0);
        assert!(opts[0].contains("localaddr=127.0.0.1:11001"));
        assert!(opts[0].contains("remoteaddr=127.0.0.1:11002"));
    }

    #[test]
    fn test_tcp_listen() {
        let net = SocketNet {
            driver: SocketNet::driver_default(),
            localaddr: None,
            localport: None,
            remoteaddr: None,
            remoteport: None,
            listen: Some("127.0.0.1:12000".to_string()),
            connect: None,
        };
        let opts = net.get_netdev_options(0);
        assert!(opts[0].contains("listen=127.0.0.1:12000"));
    }

    #[test]
    fn test_tcp_connect() {
        let net = SocketNet {
            driver: SocketNet::driver_default(),
            localaddr: None,
            localport: None,
            remoteaddr: None,
            remoteport: None,
            listen: None,
            connect: Some("127.0.0.1:12000".to_string()),
        };
        let opts = net.get_netdev_options(0);
        assert!(opts[0].contains("connect=127.0.0.1:12000"));
    }
}
