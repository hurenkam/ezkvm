use derive_getters::Getters;
use derive_new::new;
use crate::runtime::{RootDevice, RootDeviceKind};

/// CPU topology derived from Proxmox `cores`/`sockets`/`cpu` fields. No other Runtime
/// data source satisfies D-02's `-smp`/`-cpu` emission requirement.
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct CpuTopology {
    sockets: u8,
    cores: u8,
    cpu_type: String,
}

impl RootDevice for CpuTopology {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "cpu_topology" }
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::CpuTopology }
}
