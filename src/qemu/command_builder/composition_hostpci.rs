use crate::qemu::{QemuManager, types::QemuArgs};
use std::collections::HashMap;

const MAX_Q35_HOSTPCI_ROOT_PORTS: usize = 8;

struct HostPciSlotPlacement {
    bus: Option<String>,
    assign_function_addrs: bool,
}

pub(super) fn add_hostpci_args(manager: &QemuManager, args: &mut QemuArgs) {
    let has_q35_bridge_readconfig = manager
        .config
        .system
        .readconfig
        .iter()
        .any(|p| p.contains("pve-q35") || p.contains("ezkvm-q35"));
    let slot_placements = if has_q35_bridge_readconfig {
        plan_q35_hostpci_slot_placement(manager.config.host_pci())
    } else {
        HashMap::new()
    };

    for hostpci in manager.config.host_pci() {
        let slot_placement =
            hostpci_slot_key(&hostpci.device).and_then(|slot| slot_placements.get(slot));
        let bus = manager
            .normalize_legacy_root_bus(hostpci.bus.as_deref())
            .or_else(|| {
                slot_placement.and_then(|placement| {
                    manager.normalize_legacy_root_bus(placement.bus.as_deref())
                })
            });
        let default_addr = slot_placement.and_then(|placement| {
            if placement.assign_function_addrs && placement.bus.is_some() {
                default_q35_hostpci_function_addr(hostpci)
            } else {
                None
            }
        });
        let addr = hostpci.addr.as_deref().or(default_addr.as_deref());
        args.add_vfio_pci(
            &hostpci.device,
            &hostpci.id,
            hostpci.pcie,
            hostpci.x_vga,
            bus.as_deref(),
            addr,
            hostpci.multifunction,
            hostpci.romfile.as_deref(),
        );
    }
}

fn plan_q35_hostpci_slot_placement(
    hostpci_devices: &[crate::config::HostPciConfig],
) -> HashMap<String, HostPciSlotPlacement> {
    #[derive(Clone)]
    struct SlotState {
        first_seen: usize,
        min_hostpci_index: usize,
        explicit_bus: Option<String>,
        device_count: usize,
        should_assign_root_port: bool,
        has_multifunction_hint: bool,
    }

    let mut states = HashMap::<String, SlotState>::new();

    for (index, device) in hostpci_devices.iter().enumerate() {
        let Some(slot) = hostpci_slot_key(&device.device) else {
            continue;
        };

        let state = states.entry(slot.to_string()).or_insert_with(|| SlotState {
            first_seen: index,
            min_hostpci_index: hostpci_id_index(&device.id).unwrap_or(index),
            explicit_bus: None,
            device_count: 0,
            should_assign_root_port: false,
            has_multifunction_hint: false,
        });

        state.min_hostpci_index = state
            .min_hostpci_index
            .min(hostpci_id_index(&device.id).unwrap_or(index));
        state.device_count += 1;
        if state.explicit_bus.is_none() {
            state.explicit_bus = device.bus.clone();
        }
        state.should_assign_root_port |= device.pcie || device.multifunction || device.x_vga;
        state.has_multifunction_hint |= device.multifunction || device.x_vga;
    }

    let mut ordered_slots = states
        .iter()
        .filter_map(|(slot, state)| {
            if state.explicit_bus.is_none() && state.should_assign_root_port {
                Some((state.min_hostpci_index, state.first_seen, slot.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ordered_slots.sort();

    let default_buses = ordered_slots
        .into_iter()
        .enumerate()
        .map(|(position, (_, _, slot))| {
            let bus = if position < MAX_Q35_HOSTPCI_ROOT_PORTS {
                format!("ich9-pcie-port-{}", position + 1)
            } else {
                "pcie.0".to_string()
            };
            (slot, bus)
        })
        .collect::<HashMap<_, _>>();

    states
        .into_iter()
        .map(|(slot, state)| {
            let bus = state
                .explicit_bus
                .or_else(|| default_buses.get(&slot).cloned());
            (
                slot,
                HostPciSlotPlacement {
                    bus,
                    assign_function_addrs: state.has_multifunction_hint || state.device_count > 1,
                },
            )
        })
        .collect()
}

fn default_q35_hostpci_function_addr(hostpci: &crate::config::HostPciConfig) -> Option<String> {
    Some(format!("0x0.{}", hostpci_function(&hostpci.device)?))
}

fn hostpci_id_index(id: &str) -> Option<usize> {
    id.strip_prefix("hostpci")?.split('.').next()?.parse().ok()
}

fn hostpci_slot_key(device: &str) -> Option<&str> {
    device.rsplit_once('.').map(|(slot, _)| slot)
}

fn hostpci_function(device: &str) -> Option<u8> {
    let (_, slot_function) = device.rsplit_once(':')?;
    let (_, function) = slot_function.split_once('.')?;
    function.parse().ok()
}
