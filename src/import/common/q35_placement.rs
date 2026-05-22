#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportRuntimeTarget {
    #[default]
    PortableLinux,
    ProxmoxParity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostPciBusAllocation {
    Assigned(String),
    Fallback(String),
}

pub struct Q35PlacementPlanner {
    runtime_target: ImportRuntimeTarget,
    needs_q35_compat: bool,
    next_root_port: u8,
    max_portable_root_ports: u8,
}

impl Q35PlacementPlanner {
    pub fn new(runtime_target: ImportRuntimeTarget, needs_q35_compat: bool) -> Self {
        Self {
            runtime_target,
            needs_q35_compat,
            next_root_port: 1,
            max_portable_root_ports: 4,
        }
    }

    pub fn apply_machine_and_readconfig(
        &self,
        machine: &str,
        readconfig: &mut Vec<String>,
        proxmox_q35_cfg: &str,
        portable_q35_cfg: &str,
    ) -> String {
        if !self.needs_q35_compat {
            return machine.to_string();
        }

        match self.runtime_target {
            ImportRuntimeTarget::ProxmoxParity => {
                push_unique(readconfig, proxmox_q35_cfg);
                if machine.contains("+pve") {
                    machine.to_string()
                } else if machine == "q35" {
                    "pc-q35-8.1+pve0".to_string()
                } else {
                    format!("{}+pve0", machine)
                }
            }
            ImportRuntimeTarget::PortableLinux => {
                push_unique(readconfig, portable_q35_cfg);
                machine.to_string()
            }
        }
    }

    pub fn legacy_root_bus(&self) -> &'static str {
        if self.runtime_target == ImportRuntimeTarget::PortableLinux {
            if self.needs_q35_compat {
                "pci.0"
            } else {
                "pcie.0"
            }
        } else {
            "pci.0"
        }
    }

    pub fn audio_controller_bus(&self) -> &'static str {
        if self.runtime_target == ImportRuntimeTarget::PortableLinux
            || self.runtime_target == ImportRuntimeTarget::ProxmoxParity
        {
            "pci.2"
        } else {
            "pci.0"
        }
    }

    pub fn allocate_hostpci_default_bus(
        &mut self,
        has_explicit_bus: bool,
        pcie_enabled: bool,
    ) -> Option<HostPciBusAllocation> {
        if has_explicit_bus
            || !self.needs_q35_compat
            || self.runtime_target != ImportRuntimeTarget::PortableLinux
            || !pcie_enabled
        {
            return None;
        }

        if self.next_root_port <= self.max_portable_root_ports {
            let bus = format!("ich9-pcie-port-{}", self.next_root_port);
            self.next_root_port += 1;
            return Some(HostPciBusAllocation::Assigned(bus));
        }

        Some(HostPciBusAllocation::Fallback("pcie.0".to_string()))
    }
}

fn push_unique(readconfig: &mut Vec<String>, path: &str) {
    if !readconfig.iter().any(|entry| entry == path) {
        readconfig.push(path.to_string());
    }
}
