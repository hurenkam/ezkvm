# Test Fixture Template for Proxmox Import

## Purpose

Integration test fixtures in `tests/fixtures/proxmox_import/*.args` capture expected QEMU command arguments for regression detection.

When Proxmox import changes affect device topology or QEMU args generation, fixtures must be updated to reflect the new expected output.

## Format

Each fixture file contains the full `-device` and argument lines for a specific test scenario:

```
-device virtio-serial,id=qga0,bus=pci.0,addr=0x8
-chardev socket,path=/var/run/qemu-server/108.qga,server=on,wait=off,id=qga0
-device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
-device virtio-serial-pci,id=virtio-serial0
-chardev spicevmc,id=vdagent,name=vdagent
-device virtserialport,chardev=vdagent,name=com.redhat.spice.0
-device virtio-net-pci,mac=AA:BB:CC:DD:EE:FF,netdev=net0,bus=pci.0,addr=0x12,...
-netdev tap,id=net0,ifname=tap108i0,script=/usr/libexec/qemu-server/pve-bridge,downscript=/usr/libexec/qemu-server/pve-bridgedown
```

## Key Invariants to Preserve

When updating fixtures:

1. **Runtime paths** must use normalized placeholders:
   - PID files: `/var/run/qemu-server/<VMID>.pid` → `<RUNTIME_DIR>/108.pid` (or similar)
   - Socket paths: `/var/run/qemu-server/108.qga` → standardized format
   - Bridge scripts: must be `/usr/libexec/qemu-server/pve-bridge*`

2. **PCI topology** must match Proxmox defaults:
   - NIC: `bus=pci.0,addr=0x12`
   - Guest-agent: `bus=pci.0,addr=0x8`
   - GPU (if passed): typically `bus=pci.0,addr=0x10` or similar

3. **Serial controllers** must have correct topology:
   - When guest-agent is pinned (has bus/addr): use `virtio-serial` (not `-pci`)
   - SPICE vdagent: use separate `virtio-serial-pci` if agent is pinned

4. **TAP naming** must follow pattern:
   - `tap<vmid>i<index>` (e.g., `tap108i0` for VM 108, net0)

## Updating Fixtures

### Step 1: Generate New Output

```bash
cd ezkvm/
cargo test --test integration_tests proxmox_import -- --nocapture 2>&1 | tee test_output.log
```

Fixtures will fail showing expected vs actual diffs.

### Step 2: Review Diffs

```bash
# Show diffs for a specific fixture
diff -u tests/fixtures/proxmox_import/01-scsi-bridge-tpm.args.expected tests/fixtures/proxmox_import/01-scsi-bridge-tpm.args
```

Expected types of changes:
- New PCI placement addresses
- Serial controller model changes (virtio-serial vs virtio-serial-pci)
- Device ID updates

Unexpected types of changes:
- Bridge script path changes (should stay `/usr/libexec/qemu-server/pve-bridge*`)
- TAP name gone or changed
- Device removed entirely

### Step 3: Accept Expected Diffs

If diffs are intentional (because of intended topology changes):

```bash
# Update fixture files
cp tests/fixtures/proxmox_import/*.args.expected tests/fixtures/proxmox_import/*.args

# Or let your test framework auto-update them, if available
cargo test --test integration_tests proxmox_import -- --accept-diffs
```

### Step 4: Re-run Tests

```bash
cargo test --test integration_tests proxmox_import
```

All tests should pass with new fixtures.

### Step 5: Validate Parity

```bash
# Compare new ezkvm output against Proxmox reference
ezkvm run 108.yaml --dry-run > ezkvm_new.cmd
./.github/skills/proxmox-import/scripts/compare-proxmox-qemu.sh proxmox_reference.cmd ezkvm_new.cmd
```

## Common Fixture Updates

### NIC Placement Changed

Before:
```
-device virtio-net-pci,mac=...,netdev=net0,bus=pci.0,addr=0x10,...
```

After (if moved to standard Proxmox slot):
```
-device virtio-net-pci,mac=...,netdev=net0,bus=pci.0,addr=0x12,...
```

Update fixtures and verify no other devices moved.

### Guest-Agent Serial Topology Changed

Before (auto-placed):
```
-device virtio-serial-pci,id=virtio-serial0
-chardev socket,...,id=qga0
-device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
```

After (pinned):
```
-device virtio-serial,id=qga0,bus=pci.0,addr=0x8
-chardev socket,...,id=qga0
-device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
-device virtio-serial-pci,id=virtio-serial0  # NEW: separate for vdagent
```

Update fixtures to reflect new separation.

## Related Code

- **Fixture runner**: `tests/integration/proxmox_import.rs`
- **Fixture location**: `tests/fixtures/proxmox_import/*.args`
- **Mapper**: `src/import/proxmox/mapper/system.rs`
- **Args generation**: `src/qemu/args/`, `src/qemu/command_builder/`
