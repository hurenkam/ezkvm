# Debugging Workflow for Proxmox Import Issues

This guide provides step-by-step diagnostic procedures for common import-related failures.

## Issue: Network Not Working After Import

### Symptoms

- VM boots but no network connectivity
- Can't ping gateway or DHCP server
- RDP or SSH connection fails

### Diagnosis Steps

1. **Check PID file exists**
   ```bash
   ls -la /var/run/qemu-server/108.pid
   # Should exist and contain running process ID
   ps -p $(cat /var/run/qemu-server/108.pid)
   ```

2. **Verify TAP interface created**
   ```bash
   ip link show tap108i0
   # Should show: "tap108i0: ... master fwbr108i0 state forwarding"
   ```

3. **Check firewall bridge state**
   ```bash
   ip link show fwbr108i0
   # Should show UP state
   ```

4. **Inspect Proxmox bridge scripts execution**
   ```bash
   # Check if pve-bridge script was called (logs bridge creation)
   journalctl -u qemu-server | grep -i bridge
   # Or manually verify interfaces
   ip route
   ```

5. **Compare QEMU netdev arguments**
   ```bash
   # Generate dry-run output
   ezkvm run 108.yaml --dry-run 2>&1 | grep -E "netdev|device.*virtio-net"
   
   # Expected format:
   # -netdev tap,id=net0,ifname=tap108i0,script=/usr/libexec/qemu-server/pve-bridge,downscript=/usr/libexec/qemu-server/pve-bridgedown
   # -device virtio-net-pci,...,netdev=net0,bus=pci.0,addr=0x12,...
   ```

6. **Check host firewall rules**
   ```bash
   iptables-save | grep -A10 "108"
   # Should see rules binding to PID from /var/run/qemu-server/108.pid
   ```

### Root Causes & Fixes

| Root Cause | Check | Fix |
|---|---|---|
| PID file not set | `/var/run/qemu-server/108.pid` missing | Mapper must set `pid_file` from inferred VMID |
| TAP name doesn't match `tap<vmid>i*` pattern | `ip link show` doesn't find `tap108i0` | Generator must use `tap108i0`, not generic `tap0` |
| Bridge script path wrong | Proxy bridge doesn't exist at runtime | Use `/usr/libexec/qemu-server/pve-bridge*` |
| Missing explicit VMID in config | TAP/PID paths use fallback names | Import must infer or require VMID; mapper derives paths from it |

## Issue: Keyboard Not Working in Looking Glass

### Symptoms

- Mouse works; keyboard doesn't respond
- Can use RDP or on-screen keyboard
- SPICE vdagent process exists but no input

### Diagnosis Steps

1. **Verify SPICE server is running**
   ```bash
   # Check guest-agent socket
   ls -la /var/run/qemu-server/108.qga
   # Should exist and be accessible
   ```

2. **Check guest-agent process in guest**
   ```bash
   # Inside guest (via RDP or console)
   # Windows: Check Device Manager for PVE QEMU Guest Agent
   # Linux: ps aux | grep qemu-guest-agent
   ```

3. **Inspect serial controller topology**
   ```bash
   ezkvm run 108.yaml --dry-run 2>&1 | grep -E "virtio-serial|chardev.*vdagent"
   
   # Expected with pinned guest-agent:
   # -device virtio-serial,id=qga0,bus=pci.0,addr=0x8
   # -chardev socket,path=/var/run/qemu-server/108.qga,...,id=qga0
   # -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0
   # -device virtio-serial-pci,id=virtio-serial0
   # -chardev spicevmc,id=vdagent,name=vdagent
   # -device virtserialport,chardev=vdagent,name=com.redhat.spice.0
   ```

4. **Check PCI placement**
   ```bash
   ezkvm run 108.yaml --dry-run 2>&1 | grep "addr=0x8"
   # Should see guest-agent at 0x8
   ezkvm run 108.yaml --dry-run 2>&1 | grep "addr=0x12"
   # Should see NIC at 0x12
   ```

5. **Test with Looking Glass client**
   ```bash
   # Verify Looking Glass client can connect
   looking-glass-client -h <host> -p <port>
   # Try mouse first (should work)
   # Then try keyboard (if broken, see next step)
   ```

6. **Compare against Proxmox reference**
   ```bash
   # Use compare-proxmox-qemu.sh to diff topology
   ./scripts/compare-proxmox-qemu.sh proxmox_reference.cmd <(ezkvm run 108.yaml --dry-run 2>&1)
   ```

### Root Causes & Fixes

| Root Cause | Check | Symptom | Fix |
|---|---|---|---|
| Guest-agent not at `0x8` | `grep addr=0x8` | Socket path wrong or not created | Mapper must set `bus=pci.0,addr=0x8` |
| NIC moved to different PCI slot | `grep addr=0x1*` instead of `0x12` | SPICE channel routing broken | Use profile placement policy: `bus=pci.0,addr=0x12` |
| vdagent shares pinned agent controller | Only one `virtio-serial` entry | SPICE channel ID collision | Create separate `virtio-serial-pci` for vdagent |
| Wrong serial controller model | Model mixes `virtio-serial` and `virtio-serial-pci` | Device routing errors | Use `virtio-serial` only when explicit placement |

## Comparison Workflow: Proxmox vs ezkvm

### Step 1: Capture Proxmox Reference

On the Proxmox host running the original VM:

```bash
# Get VMID
vmid=108

# Extract full QEMU command
ps aux | grep "[q]emu-system.*$vmid" > proxmox_reference.txt

# Or use pgrep
ps -p $(pgrep -f "qemu-system.*$vmid") -o pid,args >> proxmox_reference.txt

# Clean up to show only arguments after qemu-system-x86_64
sed -n 's/.*qemu-system-x86_64 //p' proxmox_reference.txt > proxmox_reference.cmd
```

### Step 2: Generate ezkvm Dry-Run Output

```bash
# In ezkvm workspace
ezkvm run 108.yaml --dry-run > ezkvm_output.txt 2>&1

# Extract just the QEMU command (if prefixed with metadata)
grep "qemu-system-x86_64" ezkvm_output.txt | head -1 > ezkvm_reference.cmd
```

### Step 3: Use Comparison Script

```bash
./.github/skills/proxmox-import/scripts/compare-proxmox-qemu.sh proxmox_reference.cmd ezkvm_reference.cmd
```

Expected output:
- Minimal diffs (path substitutions, hostname-specific values)
- No device topology differences
- Same PCI placements, serial controllers, bridge scripts

### Step 4: Interpret Diffs

```bash
# Diffs like these are EXPECTED and safe:
< /var/run/qemu-server/108.pid
> /tmp/qemu-108.pid
# (Different PID file paths due to runtime env)

# Diffs like these are PROBLEMS:
< -device virtio-net-pci,...,bus=pci.0,addr=0x12,...
> -device virtio-net-pci,...,bus=pci.0,addr=0x10,...
# (NIC moved to wrong slot; will break Windows driver)

< -device virtio-serial,id=qga0,bus=pci.0,addr=0x8
> -device virtio-serial-pci,id=qga0,bus=pci.0,addr=0x8
# (Wrong serial controller model for pinned agent; breaks input routing)
```

## Regression Testing Workflow

After making changes to mapper, profile, or args generation:

1. **Update fixtures**
   ```bash
   # Run tests to show failures
   cargo test --test integration_tests proxmox_import
   # Fixtures will show diffs; review and accept if intentional
   cargo test --test integration_tests proxmox_import -- --accept-diffs
   ```

2. **Validate dry-run parity**
   ```bash
   # Generate new ezkvm output
   ezkvm run 108.yaml --dry-run > ezkvm_new.cmd
   # Compare to saved Proxmox reference
   ./scripts/compare-proxmox-qemu.sh proxmox_reference.cmd ezkvm_new.cmd
   ```

3. **Document changes**
   ```bash
   # If diffs are intentional (e.g., new placement policy),
   # update ARCHITECTURE_GUIDELINES.md or CODING_GUIDELINES.md
   # and capture the rationale
   ```

## Related Code & Docs

- **Runtime path setup**: `src/import/proxmox/mapper/system.rs`
- **Network args**: `src/qemu/args/network.rs`
- **Guest-agent args**: `src/qemu/args/guest_agent.rs`
- **Device groups**: `src/qemu/command_builder/device_groups.rs`
- **Architecture guidelines**: `doc/dev/ARCHITECTURE_GUIDELINES.md`
- **Coding guidelines**: `doc/dev/CODING_GUIDELINES.md`
