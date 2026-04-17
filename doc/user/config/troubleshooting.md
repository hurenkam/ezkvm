# Troubleshooting

Common issues and quick checks for VM configuration and startup.

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

### Checks

1. For SPICE flow, use `display.type: remote-viewer` and a valid `spice` section

2. For headless operation, use `display.type: no_display` and connect via serial/VNC intentionally

3. If using VNC TCP mode, verify expected display/port mapping:
   - TCP `port: 5900` maps to VNC display `:0`
   - TCP `port: 5901` maps to VNC display `:1`, etc.

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
