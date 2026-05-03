# Troubleshooting

Common issues and quick checks for VM configuration and startup.

## Runtime preflight fails before startup

### Symptoms

- `ezkvm start` or `ezkvm start --dry-run` exits before QEMU launch.
- Error starts with `preflight failed:`.

### Likely Causes

- Required binary is missing (`qemu-system-*`, `swtpm`, or configured bridge helper path).
- UEFI/OVMF firmware is requested but no compatible firmware file is available.
- Runtime/socket parent directory is not writable by the current user.

### Checks

1. Verify required binaries:
   ```bash
   which qemu-system-x86_64
   which swtpm
   ls -la /usr/lib/qemu/qemu-bridge-helper
   ```

2. Verify OVMF files if using UEFI/OVMF:
   ```bash
   ls -la /usr/share/ovmf
   ```

3. Validate runtime path permissions:
   ```bash
   ls -ld /var/run/ezkvm
   ```

4. Use runtime overrides to point at host-specific paths:
   ```bash
   ezkvm start vm.yaml --run-dir /tmp/ezkvm --swtpm-binary /usr/bin/swtpm --ovmf-dir /usr/share/OVMF
   ```

### Notes

- Preflight checks run in deterministic order in both `start` and `start --dry-run`.
- Optional integrations (remote-viewer, Looking Glass) emit warnings and degrade gracefully instead of failing startup.

## VM fails with memory backend or hugepages errors

### Symptoms

- QEMU reports memory backend creation errors
- Startup fails when `hugepages` and `numa_nodes` are enabled

### Likely Causes

- `system.memory.mem_path` does not exist on host
- Hugepages are enabled but host hugepage mount is not available
- `numa_nodes[].mem` values do not align with intended node sizing

### Checks

1. Confirm configured path exists and is mounted for hugepages:
   ```bash
   ls -la /run/hugepages/
   ```

2. Temporarily disable `hugepages` to validate basic NUMA shape

3. Start with one NUMA node, then expand

## No remote display appears

### Symptoms

- No viewer window appears
- Viewer opens but cannot connect

### Likely Causes

- `display.type` is `no_display`
- Missing or conflicting `spice`/`vnc` endpoint config
- Local environment cannot launch remote-viewer binary
- Central config points at the wrong viewer path for this host

### Checks

1. For SPICE flow, use `display.type: remote-viewer` and a valid `spice` section

2. For VNC flow, enable `vnc.enabled: true` with a TCP display endpoint (for example `127.0.0.1:0`)

3. For headless operation, use `display.type: no_display` and connect via serial/VNC intentionally

4. If using VNC TCP mode, verify expected display/port mapping:
   - TCP `port: 5900` maps to VNC display `:0`
   - TCP `port: 5901` maps to VNC display `:1`, etc.
   - When launching `remote-viewer` from a VNC endpoint, ezkvm translates `vnc.display` display indexes to TCP ports (`127.0.0.1:0` -> `vnc://127.0.0.1:5900`)

5. Verify the configured viewer path in central config:
   - Preferred key: `host_capabilities.integrations.remote_viewer.program`
   - Compatibility key: `tools.remote_viewer`

### Display isolation strategy (black screen after early boot)

When display handoff is ambiguous, isolate display variables before changing storage or CPU topology:

1. Switch to VNC + VGA (`vnc.enabled: true`, `devices.displays: [{type: vga}]`)
2. Temporarily remove SPICE/QXL-related profile layers
3. Keep storage/controller baseline fixed while testing display
4. Reintroduce SPICE/QXL only after baseline VGA boot is stable

## Guest gets DHCP lease but has no internet

### Symptoms

- Guest receives an IP on bridge subnet (for example via `dnsmasq.leases`)
- Guest can ping bridge gateway (host `br0` address)
- Guest cannot reach public IPs or DNS names

### Likely Causes

- Host IPv4 forwarding is disabled (`net.ipv4.ip_forward = 0`)
- NAT/forwarding policy is missing between bridge subnet and uplink interface

### Checks

1. Verify host forwarding state:
   ```bash
   sysctl -n net.ipv4.ip_forward
   ```

2. Confirm guest lease exists:
   ```bash
   sudo tail -n 20 /var/lib/misc/dnsmasq.leases
   ```

3. Confirm host default route uplink:
   ```bash
   ip route | awk '/default/ {print $5; exit}'
   ```

### Fix

1. Enable forwarding persistently:
   ```bash
   echo 'net.ipv4.ip_forward = 1' | sudo tee /etc/sysctl.d/99-ezkvm-forwarding.conf
   sudo sysctl --system
   ```

2. Add NAT and FORWARD rules for bridge subnet to uplink (replace uplink as needed):
   ```bash
   sudo iptables -t nat -A POSTROUTING -s 192.168.191.0/24 -o wlp5s0 -j MASQUERADE
   sudo iptables -A FORWARD -i br0 -o wlp5s0 -j ACCEPT
   sudo iptables -A FORWARD -i wlp5s0 -o br0 -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
   ```

   Use one firewall backend consistently (`nftables` or `iptables`) on the host.

## VM appears hung on shutdown with high CPU, then eventually exits

### Symptoms

- Display path goes black (VNC/SPICE)
- RDP/ICMP to guest stops responding
- QEMU process remains running and vCPU threads can be near 100% each
- After several minutes, process exits cleanly and `ezkvm status` reports `Not running`

### Notes

- This pattern can still be a delayed guest shutdown path (service/update/finalization), not an ezkvm host-runtime leak.
- Verify whether QEMU was started without `-no-shutdown` before assuming host-side process handling fault.
- Confirm final state using `ezkvm status`; if it transitions to `Not running`, shutdown completed.

### Checks

```bash
sudo ./target/debug/ezkvm status <vm-config.yaml>
ps -p <qemu-pid> -o pid,stat,etime,%cpu,cmd --no-headers
top -b -H -n 1 -p <qemu-pid> | sed -n '1,40p'
```

## Imported Proxmox VM boots from wrong disk

### Symptoms

- Guest enters firmware menu
- Wrong drive selected as first boot target

### Likely Causes

- Proxmox `boot: order=...` did not map to expected `boot_index` values
- Controller translation changed device naming

### Checks

1. Verify `storage[].drives[].boot_index` ordering

2. Ensure target root disk has the lowest valid boot index

3. See [import-proxmox.md#post-import-validation-checklist](import-proxmox.md#post-import-validation-checklist)

## PCI passthrough does not attach

### Symptoms

- Device missing inside guest
- QEMU rejects VFIO device arguments

### Likely Causes

- Host device id is wrong or missing function suffix
- `host.pci[].port` / `vm_id` combination conflicts
- IOMMU or VFIO not enabled on host

### Checks

1. Validate `host.pci[].host_id` format and function suffix:
   - Format: `BB:DD.F` (bus:device.function)
   - Example: `01:00.0` for single-function, `01:00` for multi-function groups

2. Start with one passthrough device, then add multi-function siblings

3. Verify host IOMMU and VFIO support:
   ```bash
   dmesg | grep -i iommu
   lsmod | grep vfio
   ```

## USB passthrough ignored

### Symptoms

- USB device not visible in guest

### Likely Causes

- Wrong selector format for the chosen mode
- Device re-enumerates on host and bus/port changed

### Checks

1. **Bus/port mode** requires three fields:
   - `vm_port`: USB port on guest controller (e.g., `"1"`)
   - `host_bus`: host bus number (e.g., `"1"`)
   - `host_port`: host port path (e.g., `"7.6"`)

2. **Vendor/product mode** requires two fields:
   - `vendor_id`: 4-digit hex text (e.g., `"0451"`)
   - `product_id`: 4-digit hex text (e.g., `"16a0"`)

3. To discover your USB device:
   ```bash
   lsusb
   # Output format: Bus 001 Device 002: ID 0451:16a0 Texas Instruments ...
   # Use: vendor_id: "0451", product_id: "16a0"
   ```

## See Also

- [Import Proxmox](import-proxmox.md)
- [Devices and Controllers](devices.md)
- [System and Boot](system-and-boot.md)
