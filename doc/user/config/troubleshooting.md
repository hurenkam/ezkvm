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
- A configured `system.readconfig` file is missing on this host (for example
   `/usr/share/ezkvm/ezkvm-q35.cfg`), which can lead to startup errors like
   `Bus 'pci.1' not found` when Q35 bridge buses are expected.

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

5. Verify every configured readconfig path exists:
   ```bash
   yq '.system.readconfig[]' vm.yaml
   ls -la /usr/share/ezkvm/ezkvm-q35.cfg
   ```

### Notes

- Preflight checks run in deterministic order in both `start` and `start --dry-run`.
- Optional integrations (remote-viewer, Looking Glass) emit warnings and degrade gracefully instead of failing startup.
- `ezkvm status <vm.yaml>` now includes a guest-agent section when QEMU guest agent is reachable, including interface/IP information from `guest-network-get-interfaces`.

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

- ezkvm automatically monitors QMP `SHUTDOWN` events and sends `quit` to QEMU when the
  guest initiates power-off. This prevents the indefinite spin that happens when GPU
  passthrough teardown hangs (common with AMD RDNA2/3 on Windows).
- When the guest has already reported shutdown but QEMU is still alive, `ezkvm status`
   reports `Stopping` and prints `Shutdown: guest shutdown observed; waiting for QEMU to exit`.
- If the VM is still alive after the screen goes dark, the auto-quit should fire within
  a few seconds. If it does not, the QMP socket may not have been created (check `ezkvm
  status` output and `/var/run/ezkvm/` for `<name>.qmp`).
- This pattern can also be a delayed guest shutdown path (service/update/finalization),
  not an ezkvm host-runtime leak.
- Confirm final state using `ezkvm status`; if it transitions to `Not running`, shutdown completed.

### Checks

```bash
sudo ./target/debug/ezkvm status <vm-config.yaml>
ps -p <qemu-pid> -o pid,stat,etime,%cpu,cmd --no-headers
top -b -H -n 1 -p <qemu-pid> | sed -n '1,40p'
```

### Manual stop behavior

- `ezkvm stop` now attempts an orderly shutdown via QMP first: it sends
   `system_powerdown`, then `quit` if the VM process stays alive.
- `SIGTERM` is now only used as a fallback when QMP is unavailable or the
   VM does not exit after QMP requests.

## GPU passthrough: VM works once but hangs on passthrough after stop/kill

### Symptoms

- First boot after host reboot works correctly with GPU passthrough
- After using `ezkvm stop` or `ezkvm kill` (or a forced termination), subsequent
  passthrough attempts produce a spinner hang or black screen
- `dmesg | grep -Ei 'vfio|reset'` shows clean reset entries with no errors

### Cause

AMD RDNA2/3 GPUs (and other AMD discrete GPUs in hybrid laptop configurations) can get
stuck in a bad register state after an unclean QEMU exit, even when the VFIO reset
sequence completes without kernel errors. The GPU driver inside the guest may have left
the device in a partially initialized state that the host VFIO reset does not fully
clear.

### Fix

Reboot the Ubuntu host before retrying GPU passthrough after any forced stop/kill:

```bash
sudo reboot
```

A host reboot performs a hardware-level GPU reset (power cycle via firmware) that fully
clears the bad state. The `vendor-reset` kernel module (`sudo modprobe vendor-reset`)
can sometimes clear it without a reboot for RDNA1 but has limited support for RDNA3.

### Prevention

- Use `ezkvm stop` rather than `ezkvm kill` when possible; the QMP auto-quit (above)
  now handles clean guest-initiated shutdown automatically.
- Never force-kill QEMU while the guest is mid-boot or mid-shutdown; wait for the
  screen to go dark before stopping.
- Disable Windows **Fast Startup** in the guest (Control Panel → Power Options →
  Choose what the power buttons do → Turn on fast startup: OFF). Fast Startup uses
  hybrid sleep (S4) instead of full shutdown (S5), which leaves the GPU in a dirty
  state and also prevents Windows from applying driver updates on boot.

## Imported VM108 (portable mode): spinner disappears and boot appears stuck

### Important non-fix

- Do not treat `x-vga` as a default remediation for this specific case.
- The captured Proxmox runtime command for VM108 does not include `x-vga=on`, so
  adding it is not required for parity with the known-good Proxmox runtime.
- Some host/QEMU combinations reject `x-vga` for `vfio-pci`; when that happens,
  forcing `x-vga` adds noise to diagnosis instead of narrowing root cause.

### What to check instead

1. Compare generated ezkvm dry-run args against the captured Proxmox command,
   prioritizing PCI bus/addr placement, USB controller placement, storage
   controller path, and guest-agent/serial topology.
2. Confirm no recent forced-stop path left the GPU in a stale state (host reboot
   remains the safest reset for this class of issue).

## Repeated EHCI warning lines during interactive start

### Symptoms

- `ezkvm start <vm.yaml>` prints repeated lines similar to:
   - `ehci: PERIODIC list base register set while periodic schedule is enabled and HC is enabled`
- VM otherwise appears to keep running.

### What it means

- This is typically QEMU guest-warning output from the emulated ICH9 EHCI controller.
- For Proxmox-style Q35 topologies (for example with `system.readconfig` set to
   `/usr/share/ezkvm/ezkvm-q35.cfg` or `pve-q35-*`), these warnings can appear even
   when topology and placement are correct.
- In most cases this is non-fatal noise, not a startup failure.

### Why it is visible

- Interactive mode inherits QEMU stderr directly in the terminal.
- In daemonized workflows, the same warnings are usually less visible because output
   is redirected to logs.

### Checks

1. Confirm VM process is actually running:
    ```bash
    sudo ./target/debug/ezkvm status <vm.yaml>
    ps -ef | grep '[q]emu-system'
    ```

2. Confirm the Q35 readconfig path exists on this host:
    ```bash
    yq '.system.readconfig[]' <vm.yaml>
    ls -la /usr/share/ezkvm/ezkvm-q35.cfg
    ```

3. If the guest is expected to boot with no local display (`type: none`), verify guest
    liveness through network, serial, or agent rather than terminal silence.

### Escalate when

- QEMU exits shortly after these warnings.
- `ezkvm status` shows the VM is not running.
- Additional hard errors appear (for example `Bus '...' not found`, failed device
   creation, or QMP startup failures).

## Pre-start hostpci slot conflict error

### Symptoms

- `ezkvm start` fails before QEMU launch with an error like:
   - `hostpci slot conflict before QEMU start: ... both use bus='...', addr='...'`

### What it means

- Two passthrough devices resolve to the same guest PCI location.
- Resolution includes both explicit config and runtime-filled defaults for missing
   hostpci placement on Q35 bridge-backed topologies.

### Fix

1. Ensure each `host.pci` device has a unique `(bus,addr)` pair.
2. For multi-function devices (for example `.0` and `.1`), place both functions on
    the same root port with different function addresses (`0x0.0`, `0x0.1`).
3. Validate portable-mode capability resolution and runtime assets with
   diagnostics before changing guest-visible passthrough flags.

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
