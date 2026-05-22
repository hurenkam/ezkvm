use crate::config::VmConfig;
use std::collections::BTreeMap;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementSource {
    Explicit,
    ProfileDefault,
    Normalized,
}

pub fn resolve_placement_precedence(
    explicit: Option<&str>,
    profile_default: Option<&str>,
    normalized: Option<&str>,
) -> Option<(String, PlacementSource)> {
    if let Some(value) = explicit {
        return Some((value.to_string(), PlacementSource::Explicit));
    }

    if let Some(value) = profile_default {
        return Some((value.to_string(), PlacementSource::ProfileDefault));
    }

    normalized.map(|value| (value.to_string(), PlacementSource::Normalized))
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

pub fn validate_placement_conflicts(config: &VmConfig) -> Result<(), String> {
    let mut conflicts = Vec::new();

    validate_pci_slot_conflicts(config, &mut conflicts);
    validate_drive_attachment_conflicts(config, &mut conflicts);

    if conflicts.is_empty() {
        return Ok(());
    }

    Err(format!(
        "placement conflict validation failed ({} conflict(s)): {}. precedence contract: explicit placement > profile defaults > normalization, and conflicts are rejected rather than auto-normalized",
        conflicts.len(),
        conflicts.join("; ")
    ))
}

fn validate_pci_slot_conflicts(config: &VmConfig, conflicts: &mut Vec<String>) {
    let mut occupied: BTreeMap<(String, String), String> = BTreeMap::new();

    for (owner, bus, addr) in collect_pci_placements(config) {
        let key = (
            normalize_placement_part(&bus),
            normalize_placement_part(&addr),
        );
        if let Some(existing) = occupied.get(&key) {
            conflicts.push(format!(
                "bus='{}', addr='{}' is assigned to both '{}' and '{}'",
                bus, addr, existing, owner
            ));
            continue;
        }
        occupied.insert(key, owner);
    }
}

fn validate_drive_attachment_conflicts(config: &VmConfig, conflicts: &mut Vec<String>) {
    let mut unit_assignments: BTreeMap<(String, u32), String> = BTreeMap::new();
    let mut scsi_id_assignments: BTreeMap<(String, u32), String> = BTreeMap::new();

    for (index, drive) in config.devices.drives.iter().enumerate() {
        let owner = format!("devices.drives[{}] id='{}'", index, drive.id);

        if let (Some(bus), Some(unit)) = (drive.bus.as_deref(), drive.unit) {
            let key = (normalize_placement_part(bus), unit);
            if let Some(existing) = unit_assignments.get(&key) {
                conflicts.push(format!(
                    "drive attachment bus='{}', unit='{}' is assigned to both '{}' and '{}'",
                    bus, unit, existing, owner
                ));
            } else {
                unit_assignments.insert(key, owner.clone());
            }
        }

        if let (Some(bus), Some(scsi_id)) = (drive.bus.as_deref(), drive.scsi_id) {
            let key = (normalize_placement_part(bus), scsi_id);
            if let Some(existing) = scsi_id_assignments.get(&key) {
                conflicts.push(format!(
                    "drive attachment bus='{}', scsi_id='{}' is assigned to both '{}' and '{}'",
                    bus, scsi_id, existing, owner
                ));
            } else {
                scsi_id_assignments.insert(key, owner);
            }
        }
    }
}

fn collect_pci_placements(config: &VmConfig) -> Vec<(String, String, String)> {
    let mut placements = Vec::new();

    for (index, hostpci) in config.host.pci.iter().enumerate() {
        if let Some((bus, addr)) = placement_pair(hostpci.bus.as_deref(), hostpci.addr.as_deref()) {
            let owner = format!(
                "host.pci[{}] id='{}' device='{}'",
                index, hostpci.id, hostpci.device
            );
            placements.push((owner, bus, addr));
        }
    }

    for (index, network) in config.devices.networks.iter().enumerate() {
        if let Some((bus, addr)) = placement_pair(network.bus.as_deref(), network.addr.as_deref()) {
            let owner = format!("devices.networks[{}] id='{}'", index, network.id);
            placements.push((owner, bus, addr));
        }
    }

    for (index, controller) in config.controllers.scsi.iter().enumerate() {
        if let Some((bus, addr)) =
            placement_pair(controller.bus.as_deref(), controller.addr.as_deref())
        {
            let owner = format!("controllers.scsi[{}] id='{}'", index, controller.id);
            placements.push((owner, bus, addr));
        }
    }

    for (index, controller) in config.controllers.sata.iter().enumerate() {
        if let Some((bus, addr)) =
            placement_pair(controller.bus.as_deref(), controller.addr.as_deref())
        {
            let owner = format!("controllers.sata[{}] id='{}'", index, controller.id);
            placements.push((owner, bus, addr));
        }
    }

    for (index, controller) in config.controllers.xhci.iter().enumerate() {
        if let Some((bus, addr)) =
            placement_pair(controller.bus.as_deref(), controller.addr.as_deref())
        {
            let owner = format!("controllers.xhci[{}] id='{}'", index, controller.id);
            placements.push((owner, bus, addr));
        }
    }

    for (index, display) in config.devices.displays.iter().enumerate() {
        if let Some((bus, addr)) = placement_pair(display.bus.as_deref(), display.addr.as_deref()) {
            let owner = format!("devices.displays[{}] type='{}'", index, display.r#type);
            placements.push((owner, bus, addr));
        }
    }

    for (index, audio) in config.devices.audio.iter().enumerate() {
        if let Some((bus, addr)) = placement_pair(audio.bus.as_deref(), audio.addr.as_deref()) {
            let owner = format!("devices.audio[{}] id='{}'", index, audio.id);
            placements.push((owner, bus, addr));
        }
    }

    if let Some(agent) = config.options.guest_agent.as_ref()
        && agent.enabled
        && let Some((bus, addr)) = placement_pair(agent.bus.as_deref(), agent.addr.as_deref())
    {
        placements.push(("options.guest_agent".to_string(), bus, addr));
    }

    if let Some(ballooning) = config.system.memory.ballooning.as_ref()
        && ballooning.enabled
        && let Some((bus, addr)) =
            placement_pair(ballooning.bus.as_deref(), ballooning.addr.as_deref())
    {
        placements.push(("system.memory.ballooning".to_string(), bus, addr));
    }

    if let Some(ivshmem) = config.system.memory.ivshmem.as_ref()
        && ivshmem.enabled
        && let Some((bus, addr)) = placement_pair(ivshmem.bus.as_deref(), ivshmem.addr.as_deref())
    {
        placements.push(("system.memory.ivshmem".to_string(), bus, addr));
    }

    placements
}

fn placement_pair(bus: Option<&str>, addr: Option<&str>) -> Option<(String, String)> {
    Some((bus?.to_string(), addr?.to_string()))
}

fn normalize_placement_part(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{PlacementSource, resolve_placement_precedence, validate_placement_conflicts};
    use crate::config::VmConfig;

    #[test]
    fn resolves_placement_precedence_explicit_then_profile_then_normalized() {
        let explicit = resolve_placement_precedence(Some("pci.0"), Some("pci.2"), Some("pcie.0"));
        assert_eq!(
            explicit,
            Some(("pci.0".to_string(), PlacementSource::Explicit))
        );

        let profile_default = resolve_placement_precedence(None, Some("pci.2"), Some("pcie.0"));
        assert_eq!(
            profile_default,
            Some(("pci.2".to_string(), PlacementSource::ProfileDefault))
        );

        let normalized = resolve_placement_precedence(None, None, Some("pcie.0"));
        assert_eq!(
            normalized,
            Some(("pcie.0".to_string(), PlacementSource::Normalized))
        );
    }

    #[test]
    fn placement_validator_accepts_distinct_slots() {
        let config = VmConfig::from_str(
            r#"
name: slot-ok
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory: { size: 4096 }
  cpu: { model: host, vcpus: 2 }
devices:
  networks:
        - { id: net0, model: virtio-net-pci, backend: { type: user }, bus: pci.0, addr: "0x12" }
host:
  pci:
    - device: 0000:03:00.0
      id: hostpci0
      bus: ich9-pcie-port-1
      addr: "0x0.0"
"#,
        )
        .expect("config should parse");

        let result = validate_placement_conflicts(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn placement_validator_rejects_pci_slot_conflicts() {
        let config = VmConfig::from_str(
            r#"
name: slot-conflict
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory: { size: 4096 }
  cpu: { model: host, vcpus: 2 }
host:
  pci:
    - device: 0000:03:00.0
      id: hostpci0
      bus: ich9-pcie-port-1
      addr: "0x0.0"
    - device: 0000:04:00.0
      id: hostpci1
      bus: ich9-pcie-port-1
      addr: "0x0.0"
"#,
        )
        .expect("config should parse");

        let err = validate_placement_conflicts(&config).expect_err("must fail");
        assert!(err.contains("placement conflict validation failed"));
        assert!(err.contains("host.pci[0]"));
        assert!(err.contains("host.pci[1]"));
    }

    #[test]
    fn placement_validator_rejects_drive_unit_conflicts() {
        let config = VmConfig::from_str(
            r#"
name: drive-conflict
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory: { size: 4096 }
  cpu: { model: host, vcpus: 2 }
devices:
  drives:
    - id: ide0
      path: /tmp/a.img
      interface: ide
      type: disk
      format: raw
      bus: ide.1
      unit: 0
    - id: ide1
      path: /tmp/b.img
      interface: ide
      type: disk
      format: raw
      bus: ide.1
      unit: 0
"#,
        )
        .expect("config should parse");

        let err = validate_placement_conflicts(&config).expect_err("must fail");
        assert!(err.contains("drive attachment"));
        assert!(err.contains("devices.drives[0]"));
        assert!(err.contains("devices.drives[1]"));
    }
}
