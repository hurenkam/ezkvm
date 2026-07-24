# Requirements: ezkvm

**Defined:** 2025-07-15
**Core Value:** Import Proxmox VM configurations, represent them in an in-memory Runtime model, save/load as ezkvm YAML, generate QEMU commandlines, and manage the full VM lifecycle (start, stop, reset) with UI client integration.

## v1 Requirements

### Runtime Model

- [x] **RUNT-01**: Runtime model supports EFI disk (`efidisk0`) including pre-enrolled keys and size attributes
- [x] **RUNT-02**: Runtime model supports TPM state (`tpmstate0`) at version 2.0 with storage volume reference
- [x] **RUNT-03**: Runtime model supports PCI passthrough (`hostpci0`–`hostpciN`) including `pcie`, `x-vga`, `rombar`, `romfile` sub-options
- [x] **RUNT-04**: Runtime model supports ivshmem shared memory devices with size and name attributes
- [x] **RUNT-05**: Runtime model supports audio devices (`audio0`) with device type (ich9-intel-hda) and driver (spice/pa/none)
- [x] **RUNT-06**: Runtime model supports SPICE display including gl, rendernode, port, and clipboard attributes
- [x] **RUNT-07**: Runtime model supports raw/opaque argument passthrough (`args`) preserved verbatim through all conversions
- [x] **RUNT-08**: Device trait exposes `device_kind()` method to eliminate unsafe `downcast_ref()` usage across device types
- [x] **RUNT-09**: All conversion errors use typed `thiserror` error enums (no `type Error = ()`)

### Proxmox Import

- [x] **PROX-01**: Parser correctly splits active config from named snapshot sections before processing any fields
- [x] **PROX-02**: Parser handles URL-encoded comments (e.g., `##args%3A`) without treating them as live config entries
- [x] **PROX-03**: Parser tokenizes sub-option values correctly for fields containing colons (MAC addresses, PCI BDFs, storage volume names)
- [x] **PROX-04**: `storage.cfg` is parsed as a first-class input alongside `.conf` to resolve storage pool references
- [x] **PROX-05**: Proxmox importer populates all RUNT-01 through RUNT-07 Runtime fields from `.conf` input
- [x] **PROX-06**: Multi-function PCI devices (e.g., `0000:03:00.0` and `0000:03:00.1`) are represented distinctly in the Runtime

### ezkvm YAML Round-Trip

- [x] **YAML-01**: Runtime serializes to ezkvm YAML format via saphyr
- [x] **YAML-02**: ezkvm YAML deserializes back to an identical Runtime (round-trip lossless for all RUNT-01–07 fields)
- [x] **YAML-03**: YAML schema covers all v1 Runtime device types

### QEMU Commandline Generation

- [x] **QEMU-01**: Runtime generates a valid QEMU commandline covering all RUNT-01–07 device types
- [x] **QEMU-02**: Emitter guarantees drive/netdev argument precedes its corresponding `-device` argument
- [x] **QEMU-03**: Raw `args` passthrough is appended verbatim at end of generated commandline
- [ ] **QEMU-04**: Generated commandline for felucia/108.conf produces a VM that starts in QEMU

### VM Lifecycle Management

- [ ] **VMGR-01**: ezkvm starts a VM by launching `qemu-system-x86_64` (and `swtpm` when TPM is configured) as child processes
- [ ] **VMGR-02**: After VM start, ezkvm launches the configured UI client (Looking Glass client or `remote-viewer`) to connect to the running VM
- [ ] **VMGR-03**: ezkvm gracefully shuts down a running VM via QEMU monitor `system_powerdown` command
- [ ] **VMGR-04**: ezkvm force-stops a running VM via QEMU monitor `quit` (or SIGTERM fallback)
- [ ] **VMGR-05**: ezkvm resets a running VM via QEMU monitor `system_reset` command

## v2 Requirements

### Extended Runtime Model

- **RUNT-V2-01**: NUMA topology (node counts, CPU and memory affinity per node)
- **RUNT-V2-02**: Serial port devices (`serial0`–`serial3`) with socket and device backends
- **RUNT-V2-03**: VGA device types (std, vmware, virtio, qxl, none)
- **RUNT-V2-04**: SMBIOS type-1 fields (uuid, manufacturer, product, version, serial)
- **RUNT-V2-05**: vmgenid (VM generation identifier for Windows licensing)
- **RUNT-V2-06**: Balloon memory device with target size
- **RUNT-V2-07**: USB tablet input device

### VM Hotplug

- **VMGR-V2-01**: Hot-plug a storage device into a running VM
- **VMGR-V2-02**: Hot-unplug a storage device from a running VM
- **VMGR-V2-03**: Hot-plug a USB device into a running VM
- **VMGR-V2-04**: Hot-unplug a USB device from a running VM
- **VMGR-V2-05**: Hot-plug a PCIe device into a running VM
- **VMGR-V2-06**: Hot-unplug a PCIe device from a running VM

### Proxmox Export

- **EXPX-01**: Runtime exports to syntactically valid Proxmox `.conf` format
- **EXPX-02**: Exported `.conf` round-trips back through importer to identical Runtime

### QEMU Commandline Import

- **QIMP-01**: Tokenize `.qemu.cmd` files using `shlex`-compatible splitting
- **QIMP-02**: Populate Runtime from tokenized QEMU arguments

## Out of Scope

| Feature | Reason |
|---------|--------|
| Proxmox API integration | File-based import only; API adds auth/network complexity |
| Full NUMA support (v1) | Deferred; adds significant Runtime complexity with low v1 priority |
| Windows guest agent config | Out of scope for commandline generation fidelity |
| Cloud-init parsing | Not present in target corpus configurations |
| Container (LXC) support | VM-only scope; LXC uses a different `.conf` schema |

## Traceability

*Populated: 2026-07-22 during roadmap creation.*

| Requirement | Phase | Status |
|-------------|-------|--------|
| RUNT-01 | Phase 2 | Complete |
| RUNT-02 | Phase 2 | Complete |
| RUNT-03 | Phase 2 | Complete |
| RUNT-04 | Phase 2 | Complete |
| RUNT-05 | Phase 2 | Complete |
| RUNT-06 | Phase 2 | Complete |
| RUNT-07 | Phase 2 | Complete |
| RUNT-08 | Phase 1 | Complete |
| RUNT-09 | Phase 1 | Complete |
| PROX-01 | Phase 3 | Complete |
| PROX-02 | Phase 3 | Complete |
| PROX-03 | Phase 3 | Complete |
| PROX-04 | Phase 3 | Complete |
| PROX-05 | Phase 4 | Complete |
| PROX-06 | Phase 4 | Complete |
| YAML-01 | Phase 6 | Complete |
| YAML-02 | Phase 6 | Complete |
| YAML-03 | Phase 5 | Complete |
| QEMU-01 | Phase 7 | Complete |
| QEMU-02 | Phase 7 | Complete |
| QEMU-03 | Phase 7 | Complete |
| QEMU-04 | Phase 9 | Pending |
| VMGR-01 | Phase 8 | Pending |
| VMGR-02 | Phase 8 | Pending |
| VMGR-03 | Phase 8 | Pending |
| VMGR-04 | Phase 8 | Pending |
| VMGR-05 | Phase 8 | Pending |

**Coverage:**

- v1 requirements: 27 total
- Mapped to phases: 27 ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2025-07-15*
*Last updated: 2025-07-15 after initial definition*
