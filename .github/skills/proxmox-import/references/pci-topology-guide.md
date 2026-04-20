# PCI Topology Guide

PCI topology (where devices are placed on the bus) matters to guests. This guide explains why Proxmox-compatible placement is crucial and how to implement it.

## Why PCI Placement Matters

### Guest OS Perspective

Guests see PCI devices as hardware. When a device moves from one PCI slot to another:
- **Linux/KVM**: Usually tolerates slot changes; minimal impact
- **Windows**: Treats different PCI slots as different adapters; triggers driver reassignment
- **macOS**: May require driver reinstall or config update

### The Moving NIC Problem

In a previous session, after fixing network topology issues, the NIC was moved from its original PCI slot. Windows treated this as a new network adapter and registered it separately from the original. This broke:
- Network connectivity (new adapter wasn't configured with DHCP)
- Looking Glass keyboard (SPICE input routing depends on the serial controller that's attached to the NIC's original controller)

### Solution: Use Proxmox Default Placement

Proxmox assigns devices to standard PCI slots. These slots are part of the initial VM config and shouldn't change unless explicitly relocated by the administrator.

## Proxmox Default Placements

### Network Interface (NIC) Placement

**Proxmox default**: `bus=pci.0,addr=0x12` for the first NIC (`net0`)

```yaml
# In a Proxmox .conf file:
net0: virtio=AA:BB:CC:DD:EE:FF,bridge=vmbr0
# Converts to QEMU args:
-device virtio-net-pci,mac=AA:BB:CC:DD:EE:FF,bus=pci.0,addr=0x12,...
```

**Why it matters**:
- Windows expects the NIC at `0x12`; moving it to `0x10` registers as a new adapter
- Guest DHCP configuration is tied to MAC address, but Windows network discovery is tied to PCI slot
- Proxmox always uses `0x12` for net0; changing it breaks guest driver assumptions

**ezkvm implementation**:
1. **Profile-based placement** (`etc/profiles.d/proxmox-base.yaml`):
   ```yaml
   policies:
     networks:
       - match:
           backend_type: tap
         defaults:
           bus: pci.0
         placement:
           addr:
             scope: bus
             bus: pci.0
             start: "0x12"
             step: 1
   ```

2. **Mapper respects mapping** (doesn't override):
   - Mapper imports net0 → networks[0]
   - Profile policy applies placement based on backend_type
   - No explicit bus/addr set by mapper; profile supplies it

### Guest-Agent Serial Placement

**Proxmox default**: Explicit placement at `bus=pci.0,addr=0x8`

```
# Proxmox QEMU command snippet:
-device virtio-serial,id=qga0,bus=pci.0,addr=0x8
-chardev socket,path=/var/run/qemu-server/108.qga,...
-device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
```

**Why it matters**:
- Proxmox pins the guest-agent to a fixed PCI slot for reliable socket access
- When guest-agent is pinned, it uses the basic `virtio-serial` controller model
- When guest-agent is auto-placed, Proxmox uses `virtio-serial-pci` (PCI form) that gets a dynamic slot

**ezkvm implementation**:
1. **Mapper sets explicit placement** (`src/import/proxmox/mapper/system.rs`, lines 350-351):
   ```rust
   Some(GuestAgentConfig {
       enabled: true,
       socket_path,
       freeze_cpu: false,
       bus: Some("pci.0".to_string()),
       addr: Some("0x8".to_string()),  // Proxmox default
   })
   ```

2. **Conditional controller model** (`src/qemu/args/guest_agent.rs`, lines 22-29):
   ```rust
   let mut serial_spec = if bus.is_some() || addr.is_some() {
       "virtio-serial,id=qga0".to_string()  // Explicit placement
   } else {
       "virtio-serial-pci,id=virtio-serial0".to_string()  // Auto-placed
   };
   ```

### Other Common Device Placements

| Device Type | Proxmox Slot | Notes |
|---|---|---|
| Storage (virtio SCSI) | `bus=pci.0,addr=0x3` | Or auto-placed if not pinned |
| GPU (hostpci) | `bus=pci.0,addr=0x10` | For first GPU; slot depends on number of prior devices |
| USB controller | `bus=pci.0,addr=0x5` | Auto-placed; actual slot varies |
| Audio | `bus=pci.0,addr=0x4` | Auto-placed |

## Serial Controller Topology: Guest-Agent + SPICE/vdagent

This is the most critical topology for Looking Glass functionality.

### The Problem

Serial controllers (virtio-serial) multiplex multiple serial devices over a single controller. Both the guest-agent and SPICE vdagent can share a controller:

```
One virtio-serial controller can carry:
- org.qemu.guest_agent.0 (guest-agent)
- com.redhat.spice.0 (SPICE vdagent for keyboard/mouse input)
```

**But when guest-agent placement is explicit** (pinned to `bus=pci.0,addr=0x8`):
- Guest-agent gets a plain `virtio-serial` controller (not `-pci` form)
- This controller doesn't have a separate PCI slot; it's embedded in the guest-agent device
- VDagent can't attach to this controller; it needs its own `virtio-serial-pci` controller

### Proxmox Topology (When Guest-Agent is Pinned)

```
-device virtio-serial,id=qga0,bus=pci.0,addr=0x8
  └─ -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0 (ATTACHED)

-device virtio-serial-pci,id=virtio-serial0
  └─ -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 (ATTACHED)
```

Two separate controllers; vdagent is on its own PCI slot.

### ezkvm Implementation

**Device groups orchestration** (`src/qemu/command_builder/device_groups.rs`, lines 41-48):
- When guest-agent is pinned (bus/addr set), create a separate virtio-serial-pci controller for vdagent
- When guest-agent is auto-placed, reuse the auto-placed controller for vdagent

```rust
// Pseudo-code
if guest_agent.bus.is_some() || guest_agent.addr.is_some() {
    // Pinned: create separate vdagent controller
    add_device("virtio-serial-pci,id=virtio-serial0");
} else {
    // Auto-placed: reuse guest-agent controller
    // vdagent attaches to same controller
}
```

## Checking Topology in Dry-Run Output

### Command to Inspect Placement

```bash
ezkvm run 108.yaml --dry-run 2>&1 | grep -E "(device|addr=0x)"
```

Expected output for Proxmox-aligned VM:
```
-device virtio-net-pci,...,bus=pci.0,addr=0x12,...
-device virtio-serial,id=qga0,bus=pci.0,addr=0x8
-device virtio-serial-pci,id=virtio-serial0
-device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
-device virtserialport,chardev=vdagent,name=com.redhat.spice.0
```

### Using the Comparison Script

See `./scripts/compare-proxmox-qemu.sh` to diff your ezkvm output against a captured Proxmox reference.

## Test Fixtures and Regression Detection

All integration test fixtures include device topology. When topology changes:

1. **Update fixture snapshots** (`tests/fixtures/proxmox_import/*.args`)
2. **Re-run integration tests** to generate new snapshots
3. **Inspect the diff** to confirm only intended topology changed
4. **Compare against Proxmox reference** to verify parity

Example diff (expected):
```diff
  -device virtio-net-pci,...,mac=AA:BB:CC:DD:EE:FF,bus=pci.0,addr=0x12
- # NO CHANGE to NIC placement if it was already at 0x12
+ # OR CHANGE if we're fixing a regression
```

## Common Topology Mistakes

| Mistake | Symptom | Fix |
|---|---|---|
| NIC placed at non-standard slot (e.g., `addr=0x10`) | Windows treats as new adapter; DHCP fails | Use Proxmox default `bus=pci.0,addr=0x12` |
| Guest-agent placed at wrong slot | Guest-agent socket not found by Proxmox tools | Use `bus=pci.0,addr=0x8` |
| SPICE vdagent shares controller with pinned guest-agent | Looking Glass keyboard stops working | Create separate `virtio-serial-pci` for vdagent |
| Wrong serial controller model (virtio-serial-pci when pinned) | Device ID conflicts or routing errors | Use `virtio-serial` when explicit placement |

## Related Code

- **NIC placement policy**: `etc/profiles.d/proxmox-base.yaml`
- **Guest-agent mapping**: `src/import/proxmox/mapper/system.rs` lines 350-351
- **Guest-agent args**: `src/qemu/args/guest_agent.rs` lines 22-29
- **Device groups**: `src/qemu/command_builder/device_groups.rs`
- **Test fixtures**: `tests/fixtures/proxmox_import/*.args`
- **Integration tests**: `tests/integration/proxmox_import.rs`
