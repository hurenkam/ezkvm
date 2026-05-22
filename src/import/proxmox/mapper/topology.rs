use super::super::RuntimeTarget;
use super::super::model::{ProxmoxHostPciEntry, ProxmoxVmConfig};
use super::{MappingWarning, helpers};
use crate::import::common::q35_placement::{
    HostPciBusAllocation, ImportRuntimeTarget, Q35PlacementPlanner,
};

const PROXMOX_Q35_CFG: &str = "/usr/share/qemu-server/pve-q35-4.0.cfg";
const EZKVM_Q35_CFG: &str = "/usr/share/ezkvm/ezkvm-q35.cfg";

pub(super) struct Q35TopologyPlanner {
    planner: Q35PlacementPlanner,
}

impl Q35TopologyPlanner {
    pub(super) fn new(
        proxmox: &ProxmoxVmConfig,
        machine: &str,
        runtime_target: RuntimeTarget,
    ) -> Self {
        let needs_q35_compat = detect_q35_compat_requirements(proxmox, machine);
        let runtime_target = match runtime_target {
            RuntimeTarget::PortableLinux => ImportRuntimeTarget::PortableLinux,
            RuntimeTarget::ProxmoxParity => ImportRuntimeTarget::ProxmoxParity,
        };

        Self {
            planner: Q35PlacementPlanner::new(runtime_target, needs_q35_compat),
        }
    }

    pub(super) fn apply_machine_and_readconfig(
        &self,
        machine: &str,
        readconfig: &mut Vec<String>,
    ) -> String {
        self.planner.apply_machine_and_readconfig(
            machine,
            readconfig,
            PROXMOX_Q35_CFG,
            EZKVM_Q35_CFG,
        )
    }

    pub(super) fn legacy_root_bus(&self) -> &'static str {
        self.planner.legacy_root_bus()
    }

    pub(super) fn audio_controller_bus(&self) -> &'static str {
        self.planner.audio_controller_bus()
    }

    pub(super) fn allocate_hostpci_default_bus(
        &mut self,
        entry: &ProxmoxHostPciEntry,
        warnings: &mut Vec<MappingWarning>,
    ) -> Option<String> {
        match self.planner.allocate_hostpci_default_bus(
            entry.options.contains_key("bus"),
            helpers::is_enabled(entry.options.get("pcie")),
        ) {
            Some(HostPciBusAllocation::Assigned(bus)) => Some(bus),
            Some(HostPciBusAllocation::Fallback(bus)) => {
                warnings.push(MappingWarning {
                    source_field: entry.key.clone(),
                    message: format!(
                        "no dedicated root port left in portable q35 template ({}); falling back to bus={}",
                        EZKVM_Q35_CFG, bus
                    ),
                });
                Some(bus)
            }
            None => None,
        }
    }
}

fn detect_q35_compat_requirements(proxmox: &ProxmoxVmConfig, machine: &str) -> bool {
    if !helpers::is_q35_machine(machine) {
        return false;
    }

    let has_pve_machine_hint = proxmox
        .scalars
        .get("machine")
        .is_some_and(|value| value.contains("+pve"));

    let has_topology_bus_hints = !proxmox.host_pci.is_empty()
        || proxmox.host_pci.iter().any(|entry| {
            entry.options.get("bus").is_some_and(|bus| {
                bus.starts_with("pci.")
                    || bus.starts_with("pcie.")
                    || bus.starts_with("ich9-pcie-port")
            })
        })
        || proxmox.scalars.get("args").is_some_and(|args| {
            args.contains("bus=pci.")
                || args.contains("bus=pcie.")
                || args.contains("ich9-pcie-port")
        });

    has_pve_machine_hint || has_topology_bus_hints
}
