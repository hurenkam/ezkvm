# ezkvm

## What This Is

ezkvm is a Rust library and CLI tool for converting and wrapping QEMU virtual machine configurations. It imports Proxmox `.conf` and `.qemu.cmd` files into an in-memory Runtime model, allows saving/loading via its own YAML format, and exports back to Proxmox config files and raw QEMU commandlines.

## Core Value

Full round-trip fidelity: import a Proxmox VM config → ezkvm YAML → re-export to byte-identical Proxmox `.conf` and working QEMU commandline.

## Requirements

### Validated

- ✓ Basic Runtime model (Memory, Q35 Chipset, basic CPU config) — existing
- ✓ Basic storage devices: SSD, HDD, CDROM via PvScsi/SATA/IDE buses — existing
- ✓ ezkvm schema layer (ConfigSchema, VirtualMachineSchema, HostSchema, DeviceSchema) — existing
- ✓ Custom YAML serialization using saphyr (serde_yaml replacement) — existing
- ✓ File I/O layer: parse/build ezkvm YAML files — existing
- ✓ Builder pattern for Runtime construction (RuntimeBuilder, Q35ChipsetBuilder) — existing

### Active

- [ ] Runtime model complete — represents all Proxmox VM features including: EFI/OVMF, TPM, PCI passthrough (hostpci), USB passthrough, audio devices, SPICE display, ivshmem shared memory, network interfaces (virtio/e1000), SMBIOS, vmgenid, NUMA, raw `args` passthrough, snapshots
- [ ] Import Proxmox `.conf` files → Runtime (parse `<host>/<vm>.conf` + `storage.conf`)
- [ ] Save Runtime → ezkvm YAML file; load ezkvm YAML → Runtime
- [ ] Export Runtime → Proxmox `.conf` file
- [ ] Convert Runtime → QEMU commandline
- [ ] Import QEMU `.qemu.cmd` files → Runtime
- [ ] VM lifecycle management — start VM (qemu + swtpm), launch UI client (Looking Glass / remote-viewer), graceful shutdown, force-stop, reset via QEMU monitor

### Out of Scope

- GUI or web interface — CLI and library API only
- Live migration or cluster management — Proxmox cluster features beyond config import
- Network/storage provisioning on the host — only VM config representation, not host setup
- VM hotplug (storage/USB/PCIe) — deferred to v2

## Context

- Input corpus: `input/<host>/<vm>.conf`, `input/<host>/storage.conf`, `input/<host>/<vm>.qemu.cmd` files from real Proxmox servers (coruscant, felucia, wakiza, zbp-server-mh2, ezkvm hosts)
- Canonical test case: `input/felucia/108.conf` — Windows 11 gaming VM with PCI passthrough, Looking Glass (ivshmem), SPICE, TPM, EFI, USB passthrough
- Runtime layer is largely incomplete; ezkvm schema is reasonably complete; Proxmox import/export and Runtime↔Schema conversions are scaffolding
- Architecture: Runtime ↔ ConfigSchema ↔ YAML, with trait-based polymorphism (`Arc<dyn Trait>`) and `TryFrom` conversions between layers
- Custom saphyr-based YAML avoids JSON intermediates and preserves field ordering

## Constraints

- **Tech Stack**: Rust (edition 2024) — all implementation in Rust; no runtime deps outside Cargo
- **Compatibility**: Must correctly represent the full range of configs in the `input/` corpus, with `felucia/108.conf` as the minimum bar
- **Round-trip fidelity**: Export output must match original Proxmox `.conf` and `.qemu.cmd` format closely enough to be valid inputs

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Custom saphyr-based serde_yaml | Avoids JSON intermediate format; preserves YAML ordering and style | — Pending |
| Arc-wrapped trait objects for devices | Shared ownership across multiple buses (e.g. storage on SATA + SCSI) | — Pending |
| Schema-first serialization (ConfigSchema is YAML source of truth) | Clean separation between in-memory Runtime and serializable schema | — Pending |
| Proxmox `.conf` snapshot sections → Runtime | Snapshots are named sections in `.conf`; Runtime needs to represent or skip them | — Pending |

---
*Last updated: 2026-07-22 after initial project onboarding*
