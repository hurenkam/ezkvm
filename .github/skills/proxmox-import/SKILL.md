---
name: proxmox-import
description: "Workflow for importing Proxmox VMs, ensuring runtime parity, debugging network/topology issues, testing fixtures, and validating dry-run commands. Use when working on Proxmox `.conf` import, device topology, runtime integration paths, or comparing generated QEMU commands against Proxmox reference captures."
argument-hint: "Describe the import task: topology alignment, fixture update, runtime path validation, dry-run comparison, etc."
---

# Proxmox VM Import Workflow

## When to Use

- **Importing Proxmox VMs** into ezkvm and ensuring guest functionality (network, keyboard, display)
- **Debugging network or input failures** after import (tap naming, PCI placement, serial topology)
- **Updating fixture snapshots** when device topology or QEMU command arguments change
- **Validating dry-run parity** against captured Proxmox command lines to catch regressions
- **Preserving Proxmox runtime integration anchors** (PID files, socket paths, tap naming)
- **Aligning PCI and serial topology** to match Proxmox defaults and guest OS expectations
- **Resolving Looking Glass keyboard or SPICE input failures** caused by serial controller mismatches

## Key Principles

1. **Runtime parity matters**: Proxmox runtime paths (`/var/run/qemu-server/<vmid>.pid`, socket paths, tap naming) are infrastructure anchors. Changing them can break host/guest integration.

2. **PCI topology affects guests**: Windows and other guests treat different PCI slots as different adapters. Moving a NIC or guest-agent from one slot to another can trigger driver reassignment and network loss.

3. **Serial topology controls input routing**: Guest-agent and SPICE vdagent channels use serial controllers. Sharing a controller when the guest-agent is pinned can break Looking Glass keyboard input.

4. **Dry-run validation is essential**: Always compare generated QEMU arguments against a captured Proxmox reference to detect regressions before testing in VMs.

5. **Q35 hierarchy is policy, not preference**: Keep PCIe devices in PCIe paths and legacy PCI devices in bridge-backed legacy PCI paths unless an explicit exception is documented.

## Q35 Topology Decision Tree

Use this decision flow before changing bus placement:

1. Is the target device PCIe-capable and expected to use PCIe features?
   - Yes: place behind `pcie-root-port` (or a downstream PCIe switch port if already required).
   - No: continue to step 2.
2. Is the target device legacy PCI only?
   - Yes: place behind `pcie-pci-bridge` and optionally `pci-bridge` when scaling device count.
   - No: continue to step 3.
3. Is hotplug required?
   - PCIe hotplug: use dedicated root ports/downstream ports and keep ports available.
   - Legacy PCI hotplug: ensure bridge path supports ACPI/SHPC behavior.
4. Is imported guest identity stability required?
   - Yes: preserve existing `bus/addr` mapping unless migration guidance explicitly allows change.
5. Are IO and bus-number budgets still safe?
   - If unclear, stop and evaluate budget impact before merging topology changes.

## Workflow

### Phase 1: Environment Setup

1. **Capture a Proxmox reference command**
   ```bash
   # On the Proxmox host, get the VMID (e.g., 108)
   qm config 108
   # Extract the live QEMU command
   ps aux | grep qemu-system-x86_64 | grep -v grep
   # Save to a file for comparison
   cat > proxmox_reference.cmd << 'EOF'
   # Paste the full qemu-system-x86_64 command here
   EOF
   ```

2. **Export the Proxmox .conf file**
   ```bash
   # From host running ezkvm
   scp proxmox-host:/etc/pve/qemu-server/108.conf .
   ```

3. **Set up ezkvm dry-run environment**
   - Ensure you have `--dry-run` or similar mode to emit QEMU args without starting VM
   - Prepare output redirection to capture `-device` and `-netdev` arguments

### Phase 2: Import and Initial Validation

1. **Run the import**
   ```bash
   ezkvm import-proxmox --source 108.conf --output 108.yaml
   ezkvm run 108.yaml --dry-run > ezkvm.cmd
   ```

2. **Compare network arguments**
   - See [debugging-workflow.md](./references/debugging-workflow.md) for diagnostic commands
   - Check tap naming: should be `tap<vmid>i<index>` (e.g., `tap108i0`)
   - Check NIC bus/addr: should match Proxmox placement (typically `bus=pci.0,addr=0x12` for first NIC)
   - Verify bridge script path: `/usr/libexec/qemu-server/pve-bridge`

3. **Log any mismatches**
   - Record differences in tap naming, PCI slots, or missing runtime paths
   - These become the basis for mapper and profile updates

### Phase 3: Runtime Path and Device Topology Alignment

See [runtime-parity-checklist.md](./references/runtime-parity-checklist.md) for complete invariants.

**Step A: PID and socket paths**
- Mapper must derive from VMID (if inferred from config paths)
- Format: `/var/run/qemu-server/<vmid>.pid`
- Socket: `/var/run/qemu-server/<vmid>.qga`

**Step B: NIC PCI placement**
- See [pci-topology-guide.md](./references/pci-topology-guide.md)
- Proxmox default: `bus=pci.0,addr=0x12` for first NIC (net0)
- Add placement policy to `proxmox-base.yaml` profile to ensure consistency across imports

**Step C: Guest-agent serial topology**
- Proxmox pins guest-agent to `bus=pci.0,addr=0x8`
- Mapper must set explicit bus/addr (not auto-place)
- When explicit placement is set, use `virtio-serial` controller model (not `virtio-serial-pci`)
- Create separate `virtio-serial-pci` controller for SPICE vdagent to avoid input routing conflicts

### Phase 4: Fixture Update and Regression Testing

1. **Update integration test fixtures**
   - New device topology requires new `.args` snapshots
   - Located in `tests/fixtures/proxmox_import/*.args`
   - Format: one QEMU argument per line (or grouped device definitions)
   - Use [asset/test-fixture-template.args](./assets/test-fixture-template.args) for reference

2. **Run integration tests**
   ```bash
   cargo test --test integration_tests --features proxmox proxmox_import -- --nocapture
   ```

3. **Validate dry-run parity**
   - Re-generate ezkvm dry-run output with updated code
   - Use [compare-proxmox-qemu.sh](./scripts/compare-proxmox-qemu.sh) script
   - Expected output: minimal diffs, no topology regressions

4. **Run topology parity checks**
   - Confirm PCIe/legacy PCI separation is still respected.
   - Confirm bridge count and bus-number growth are intentional and justified.
   - Confirm imported sensitive devices keep stable guest-visible slot identity.

### Phase 5: Guest Functionality Testing

1. **Boot the VM with updated config**
   ```bash
   ezkvm run 108.yaml
   ```

2. **Verify connectivity**
   - Network: check DHCP, ping, RDP (Windows) or SSH (Linux)
   - Display: check SPICE/Looking Glass rendering
   - Input: test keyboard and mouse in Looking Glass
   - TPM: if enabled, verify PCR extensions

3. **Log issues and map to topology**
   - Network failures often indicate PID path, tap naming, or bridge script issues
   - Input failures usually indicate serial controller or PCI placement problems
   - See [debugging-workflow.md](./references/debugging-workflow.md) for diagnosis

## Code Changes Checklist

Before committing import-related changes:

- [ ] Mapper updates preserve Proxmox runtime paths when VMID is known
- [ ] NIC PCI placement policy added or maintained in `proxmox-base.yaml` profile
- [ ] Guest-agent serial controller model conditional on explicit vs auto-placement
- [ ] SPICE vdagent uses separate serial controller when guest-agent is pinned
- [ ] All integration test fixtures updated with new device topology
- [ ] Dry-run parity validation performed (no regression diffs)
- [ ] ARCHITECTURE_GUIDELINES.md updated if new parity invariants discovered
- [ ] CODING_GUIDELINES.md extended if new import compactness rules apply

## Debugging References

- **Network not working?** → See [runtime-parity-checklist.md](./references/runtime-parity-checklist.md) PID path and tap naming
- **Keyboard not working in Looking Glass?** → See [pci-topology-guide.md](./references/pci-topology-guide.md) serial controller section
- **How to compare QEMU commands?** → See [debugging-workflow.md](./references/debugging-workflow.md) and use [compare-proxmox-qemu.sh](./scripts/compare-proxmox-qemu.sh)
- **Which test fixtures to update?** → See [asset/test-fixture-template.args](./assets/test-fixture-template.args)

## Links to Related Documentation

- **Architecture Guidelines** (`doc/dev/architecture/architecture-guidelines.md`): Proxmox Runtime Parity Invariants section
- **Coding Guidelines** (`doc/dev/workflow/coding-guidelines.md`): Import output compactness rules
- **Mapper implementation** (`src/import/proxmox/mapper/system.rs`): Where runtime paths are derived
- **QEMU args generation** (`src/qemu/args/`): Device topology and serial controller logic
- **Integration tests** (`tests/integration/proxmox_import.rs`): Import validation workflow
