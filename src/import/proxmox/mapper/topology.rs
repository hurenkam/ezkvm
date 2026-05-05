use super::super::RuntimeTarget;
use super::super::model::{ProxmoxHostPciEntry, ProxmoxVmConfig};
use super::{MappingWarning, helpers};

const PROXMOX_Q35_CFG: &str = "/usr/share/qemu-server/pve-q35-4.0.cfg";
const EZKVM_Q35_CFG: &str = "/usr/share/ezkvm/ezkvm-q35.cfg";
const MAX_PORTABLE_ROOT_PORTS: u8 = 4;

pub(super) struct Q35TopologyPlanner {
    runtime_target: RuntimeTarget,
    needs_q35_compat: bool,
    next_root_port: u8,
}

impl Q35TopologyPlanner {
    pub(super) fn new(
        proxmox: &ProxmoxVmConfig,
        machine: &str,
        runtime_target: RuntimeTarget,
    ) -> Self {
        let needs_q35_compat = detect_q35_compat_requirements(proxmox, machine);

        Self {
            runtime_target,
            needs_q35_compat,
            next_root_port: 1,
        }
    }

    pub(super) fn apply_machine_and_readconfig(
        &self,
        machine: &str,
        readconfig: &mut Vec<String>,
    ) -> String {
        if !self.needs_q35_compat {
            return machine.to_string();
        }

        match self.runtime_target {
            RuntimeTarget::ProxmoxParity => {
                push_unique(readconfig, PROXMOX_Q35_CFG);
                if machine.contains("+pve") {
                    machine.to_string()
                } else if machine == "q35" {
                    "pc-q35-8.1+pve0".to_string()
                } else {
                    format!("{}+pve0", machine)
                }
            }
            RuntimeTarget::PortableLinux => {
                push_unique(readconfig, EZKVM_Q35_CFG);
                machine.to_string()
            }
        }
    }

    pub(super) fn legacy_root_bus(&self) -> &'static str {
        if self.runtime_target == RuntimeTarget::PortableLinux {
            "pcie.0"
        } else {
            "pci.0"
        }
    }

    pub(super) fn audio_controller_bus(&self) -> &'static str {
        if self.runtime_target == RuntimeTarget::PortableLinux {
            "pcie.0"
        } else if self.runtime_target == RuntimeTarget::ProxmoxParity {
            "pci.2"
        } else {
            "pci.0"
        }
    }

    pub(super) fn allocate_hostpci_default_bus(
        &mut self,
        entry: &ProxmoxHostPciEntry,
        warnings: &mut Vec<MappingWarning>,
    ) -> Option<String> {
        if entry.options.contains_key("bus")
            || !self.needs_q35_compat
            || self.runtime_target != RuntimeTarget::PortableLinux
        {
            return None;
        }

        if !helpers::is_enabled(entry.options.get("pcie")) {
            return None;
        }

        if self.next_root_port <= MAX_PORTABLE_ROOT_PORTS {
            let bus = format!("ich9-pcie-port-{}", self.next_root_port);
            self.next_root_port += 1;
            return Some(bus);
        }

        warnings.push(MappingWarning {
            source_field: entry.key.clone(),
            message: format!(
                "no dedicated root port left in portable q35 template ({}); falling back to bus=pcie.0",
                EZKVM_Q35_CFG
            ),
        });
        Some("pcie.0".to_string())
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

fn push_unique(readconfig: &mut Vec<String>, path: &str) {
    if !readconfig.iter().any(|entry| entry == path) {
        readconfig.push(path.to_string());
    }
}
