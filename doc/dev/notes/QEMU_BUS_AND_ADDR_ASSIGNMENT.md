# QEMU Bus and Address Assignment

Date: 2026-05-23
Scope: Upstream QEMU / Proxmox Q35 placement behavior
Purpose: Record how QEMU places devices when `bus` and/or `addr` are omitted on the command line.

## Question

How does QEMU handle devices in a Q35 machine when the user does not explicitly assign a bus or address on the command line?

## Findings

### 1. Omitted `bus=` uses the device class default bus type

QEMU first resolves the bus by looking at the device class `bus_type`.
If no explicit `bus=` is supplied, `qdev_device_add_from_qdict()` calls the default bus lookup path and searches recursively from the system bus for the first matching bus of that type.

For PCI devices, the class bus type is `TYPE_PCI_BUS`, so Q35 devices normally land on the root PCIe bus rather than on a legacy bus.

Relevant code paths:

- `system/qdev-monitor.c`: `qdev_device_add_from_qdict()`, `qdev_find_default_bus()`
- `hw/pci/pci.c`: PCI device class bus type registration (`TYPE_PCI_BUS`)

### 2. Q35's root PCI bus is `pcie.0`

The Q35 host bridge creates the root bus as `pcie.0` with type `TYPE_PCIE_BUS`.
The machine setup in `pc_q35.c` retrieves that child bus explicitly, so the default PCI placement target on Q35 is the `pcie.0` hierarchy unless a different bus is named.

Relevant code paths:

- `hw/pci-host/q35.c`: `q35_host_realize()` creates `pci_root_bus_new(..., "pcie.0", ..., TYPE_PCIE_BUS)`
- `hw/i386/pc_q35.c`: machine wiring uses `qdev_get_child_bus(..., "pcie.0")`

### 3. Omitted `addr=` means auto-assignment

The PCI `addr` property is defined with a default value of `-1`.
That sentinel means "auto assign" during device registration.

The property parser accepts either:

- `addr=<slot>` which means slot `<slot>`, function `0`
- `addr=<slot>.<fn>` which means the exact slot/function pair
- integer values in the range `-1..255`, where `-1` preserves the auto-assignment behavior

Relevant code paths:

- `hw/pci/pci.c`: `DEFINE_PROP_PCI_DEVFN("addr", PCIDevice, devfn, -1)`
- `hw/core/qdev-properties-system.c`: `set_pci_devfn()` and `print_pci_devfn()`

### 4. Auto-assignment scans for the first free function-0 slot

When `devfn < 0`, `do_pci_register_device()` walks the bus from `devfn_min` upward in steps of `PCI_FUNC_MAX`.
That means it searches slot-by-slot, always choosing function `0` for the first free and non-reserved slot.

The result is deterministic for a given bus state: QEMU picks the earliest available slot, not a random or policy-driven placement.

Relevant code paths:

- `hw/pci/pci.c`: `do_pci_register_device()`

## Practical Q35 Behavior

For a PCI device on Q35:

- no `bus=` and no `addr=` -> attach to `pcie.0`, auto-pick the first free slot, function `0`
- no `bus=` and `addr=06` -> attach to `pcie.0`, use slot `6`, function `0`
- no `bus=` and `addr=06.1` -> attach to `pcie.0`, use slot `6`, function `1`
- explicit `bus=` overrides the default bus selection

## Implications for ezkvm

This behavior matters when mirroring Proxmox or QEMU command lines:

- implicit PCI placement is not just "missing addr"; it is a two-step resolution of bus and devfn
- a Q35 importer must preserve the distinction between bus selection and slot allocation
- if a guest-visible command line omits bus/address, ezkvm should expect QEMU to place the device on `pcie.0` and auto-assign the first available function-0 slot
- compact or replay-safe exports should keep that implicit behavior stable across import cycles

## Source Files Reviewed

- `/home/hurenkam/Workspace/proxmox-qemu/qemu/system/qdev-monitor.c`
- `/home/hurenkam/Workspace/proxmox-qemu/qemu/hw/core/qdev-properties-system.c`
- `/home/hurenkam/Workspace/proxmox-qemu/qemu/hw/pci/pci.c`
- `/home/hurenkam/Workspace/proxmox-qemu/qemu/hw/pci-host/q35.c`
- `/home/hurenkam/Workspace/proxmox-qemu/qemu/hw/i386/pc_q35.c`
