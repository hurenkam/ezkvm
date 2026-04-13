/// Network mode for a VM
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkMode {
    /// User mode networking (NAT)
    User,
    /// Bridge mode (direct network access)
    Bridge,
    /// Isolated network
    Isolated,
}

impl std::fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMode::User => write!(f, "user"),
            NetworkMode::Bridge => write!(f, "bridge"),
            NetworkMode::Isolated => write!(f, "isolated"),
        }
    }
}

/// Represent a port forwarding rule
#[derive(Debug, Clone)]
pub struct PortForwardRule {
    pub protocol: String, // tcp/udp
    pub host_port: u16,
    pub guest_port: u16,
    pub guest_ip: String,
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub interface: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}
