---
description: "Use when editing Proxmox VM import code, device topology, runtime paths, or QEMU args generation. Preserve Proxmox runtime parity; validate against captured QEMU commands. See /proxmox-import skill for full workflow."
applyTo: "src/import/proxmox/**/*.rs, src/qemu/args/**/*.rs, src/qemu/command_builder/**/*.rs, etc/profiles.d/proxmox-*.yaml, tests/fixtures/proxmox_import/**, tests/integration/proxmox_import.rs"
---

# Proxmox Import Code Guidelines

## Quick Checklist Before Committing

- [ ] **Runtime paths preserved**: PID files at `/var/run/qemu-server/<vmid>.pid`, sockets at `/var/run/qemu-server/<vmid>.qga`
- [ ] **TAP naming**: Uses `tap<vmid>i<index>` pattern, not generic names
- [ ] **PCI placement**: NIC at `bus=pci.0,addr=0x12`, guest-agent at `bus=pci.0,addr=0x8`
- [ ] **Q35 hierarchy policy**: PCIe devices on PCIe ports; legacy PCI devices behind `pcie-pci-bridge`/`pci-bridge`
- [ ] **Bridge budget reviewed**: IO and bus-number impact justified when adding bridges/switch depth
- [ ] **Slot identity stability**: imported sensitive devices keep guest-visible `bus/addr` unless migration note exists
- [ ] **Serial topology**: Separate `virtio-serial-pci` for vdagent when guest-agent is pinned
- [ ] **Test fixtures updated**: `tests/fixtures/proxmox_import/*.args` reflect new device topology
- [ ] **Dry-run parity validated**: No regression diffs vs captured Proxmox reference

## Common Mistakes to Avoid

| Mistake | Impact | Fix |
|---------|--------|-----|
| PID path not set from VMID | Network no access (firewall bridge fails) | Mapper must set from inferred VMID |
| NIC moved to different PCI slot | Windows treats as new adapter; DHCP fails | Use profile placement policy: `addr: "0x12"` |
| Guest-agent socket path undefined | Proxmox tools can't find guest-agent | Mapper must set: `/var/run/qemu-server/<vmid>.qga` |
| SPICE vdagent shares pinned agent controller | Looking Glass keyboard stops working | Create separate `virtio-serial-pci` for vdagent |
| Fixtures not updated | Test suite out of sync; false positives on future changes | Re-run integration tests; accept new snapshots |

## Runtime Path Integration

Proxmox runtime integration depends on specific paths:

```rust
// In mapper: derive paths from inferred VMID
let inferred_vmid = extract_vmid_from_config_path(&config_file);
if let Some(vmid) = inferred_vmid {
    system_config.pid_file = Some(format!("/var/run/qemu-server/{}.pid", vmid));
    guest_agent.socket_path = Some(format!("/var/run/qemu-server/{}.qga", vmid));
}
```

## PCI Topology Rules

- **NIC placement** (profile policy): `bus: pci.0`, addr: `0x12` for net0, `0x13` for net1, etc.
- **Guest-agent placement** (mapper): `bus: pci.0`, addr: `0x8`
- **Q35 separation**: keep PCIe devices in PCIe hierarchy; keep legacy PCI devices in legacy PCI islands
- **Flat-by-default**: avoid adding PCIe switch depth unless bus-count constraints require it
- **Serial controller model**: Use `virtio-serial` (not `-pci`) when explicit bus/addr set
- **SPICE vdagent**: Separate controller `virtio-serial-pci` when guest-agent is pinned

## Topology Change Fail Conditions

Treat these as blockers unless explicitly approved in the change notes:

- Imported NIC/GPU/guest-agent-related devices move to different guest-visible slots without migration rationale.
- PCIe and legacy PCI hierarchies are mixed in a way that breaks the Q35 placement policy.
- Bridge/switch additions are made without IO and bus-number budget review.

## Device Topology Code Pattern

```rust
// src/qemu/args/guest_agent.rs
let mut serial_spec = if bus.is_some() || addr.is_some() {
    "virtio-serial,id=qga0".to_string()  // Explicit placement: use base model
} else {
    "virtio-serial-pci,id=virtio-serial0".to_string()  // Auto-placed: use PCI form
};
```

## Test Fixture Updates

When device topology changes:

```bash
# 1. Generate new snapshots
cargo test --test integration_tests proxmox_import

# 2. Review diffs for expected topology changes only
diff -u tests/fixtures/proxmox_import/01-scsi-bridge-tpm.args.expected \
        tests/fixtures/proxmox_import/01-scsi-bridge-tpm.args

# 3. Accept snapshots
cp tests/fixtures/proxmox_import/*.args.expected tests/fixtures/proxmox_import/*.args

# 4. Validate parity against Proxmox
ezkvm run 108.yaml --dry-run > ezkvm.cmd
./.github/skills/proxmox-import/scripts/compare-proxmox-qemu.sh proxmox_ref.cmd ezkvm.cmd
```

## Related Documentation

- **Full workflow**: See `/proxmox-import` skill for complete procedures
- **Runtime parity invariants**: `doc/dev/ARCHITECTURE_GUIDELINES.md` → "Proxmox Runtime Parity Invariants" section
- **Import compactness rules**: `doc/dev/CODING_GUIDELINES.md` → "Import output compactness rule" section
- **Mapper implementation**: `src/import/proxmox/mapper/system.rs`
- **Device groups orchestration**: `src/qemu/command_builder/device_groups.rs`
- **Profile bases**: `etc/profiles.d/proxmox-*.yaml`

## When to Load the Full Skill

Use `/proxmox-import` skill for:
- Debugging network or keyboard failures after import
- Comparing QEMU commands against Proxmox reference
- Understanding PCI topology alignment
- Learning the complete import workflow
