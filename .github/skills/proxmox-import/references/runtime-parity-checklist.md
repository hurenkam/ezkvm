# Runtime Parity Checklist

This checklist ensures that imported Proxmox VMs maintain runtime integration with Proxmox host infrastructure. These are not cosmetic differences—changing them can break network access, guest-agent functionality, or storage integration.

## Checklist: Runtime Path Integration

### PID File Path
- [ ] **Check**: Is PID file path set to `/var/run/qemu-server/<vmid>.pid`?
- [ ] **Why**: Proxmox firewall bridge validation script reads this PID to match iptables rules
- [ ] **Impact if wrong**: VM has no network access; host routing/firewall rules don't bind to VM process
- [ ] **Code location**: `src/import/proxmox/mapper/system.rs`, line ~256
- [ ] **Derivation**: VMID inferred from config file paths; format: `/var/run/qemu-server/{inferred_vmid}.pid`

### Guest-Agent Socket Path
- [ ] **Check**: Is guest-agent socket path set to `/var/run/qemu-server/<vmid>.qga`?
- [ ] **Why**: Proxmox cluster management looks for guest-agent sockets at this standard location
- [ ] **Impact if wrong**: Proxmox can't communicate with guest (live migration, snapshots affected)
- [ ] **Code location**: `src/import/proxmox/mapper/system.rs`, line ~350-355
- [ ] **Example**: VM 108 should have socket `/var/run/qemu-server/108.qga`

### TAP Interface Naming
- [ ] **Check**: Is TAP interface named `tap<vmid>i<net_index>`?
- [ ] **Why**: Proxmox bridge scripts (`/usr/libexec/qemu-server/pve-bridge*`) use this naming to identify which VM owns which tap
- [ ] **Impact if wrong**: Bridge scripts don't recognize tap; firewall rules don't apply; network isolation fails
- [ ] **Format**: `tap108i0` for VM 108, first network (net0); `tap108i1` for second network, etc.
- [ ] **Code location**: `src/qemu/args/network.rs` or equivalent `-netdev` argument builder
- [ ] **Derivation**: VMID + network index from config

### Swiftpm (TPM) Socket Path (if applicable)
- [ ] **Check**: If TPM enabled, is socket path `/var/run/qemu-server/<vmid>.swtpm`?
- [ ] **Why**: Proxmox TPM integration relies on standard socket location
- [ ] **Impact if wrong**: TPM attestation and PCR measurements may not persist across reboots
- [ ] **Code location**: `src/qemu/args/` TPM device builder
- [ ] **Example**: VM 108 with TPM should use `/var/run/qemu-server/108.swtpm`

## Verification Commands

Test runtime path integration on a running ezkvm-launched VM:

```bash
# Check PID file was created
ls -la /var/run/qemu-server/108.pid

# Check guest-agent socket exists
ls -la /var/run/qemu-server/108.qga

# Check tap interface enslaved to bridge
ip link show tap108i0
# Should show: "master fwbr108i0 state forwarding"

# Check firewall bridge exists (created by pve-bridge script)
ip link show fwbr108i0

# Validate iptables rules bound to PID
iptables-save | grep -A5 "108"
```

## Related Code Locations

- **Mapper derivation**: `src/import/proxmox/mapper/system.rs`
- **Mapper tests**: `src/import/proxmox/mapper.rs` test module
- **Network args**: `src/qemu/args/network.rs`
- **Guest-agent args**: `src/qemu/args/guest_agent.rs`
- **Dry-run snapshots**: `tests/fixtures/proxmox_import/*.args`

## Common Mistakes

| Mistake | Symptom | Fix |
|---------|---------|-----|
| PID path not set | Network no access from bridge | Add PID derivation in mapper |
| TAP name is generic (not `tap<vmid>i*`) | Bridge scripts don't run; no firewall rules | Use VMID in TAP name |
| Guest-agent socket path incorrect | Agent socket at `/var/run/qga-*` instead of Proxmox standard | Set explicit socket path in mapper |
| Runtime paths use XDG_RUNTIME_DIR variable | Works locally, fails when XDG undefined on different host | Hardcode `/var/run/qemu-server/` prefix |

## Testing Runtime Path Integration

### Unit Test Example
```rust
// In mapper tests, verify PID path is set when VMID inferred
#[test]
fn test_mapper_sets_pid_path_from_vmid() {
    let inferred_vmid = Some(108);
    let result = map_proxmox_to_config(conf, inferred_vmid);
    assert_eq!(result.system.pid_file, Some("/var/run/qemu-server/108.pid".to_string()));
}
```

### Integration Test Validation
1. Run import: `ezkvm import-proxmox 108.conf`
2. Start VM: `ezkvm run 108.yaml`
3. Verify paths exist in `/var/run/qemu-server/`
4. Test network connectivity from host and guest
