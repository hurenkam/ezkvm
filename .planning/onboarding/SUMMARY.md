# Onboarding Summary: ezkvm

**Completed:** 2025-07-15
**Workflow:** /gsd-onboard → /gsd-map-codebase → /gsd-new-project

---

## What Was Done

| Step | Output | Commit |
|------|--------|--------|
| Map codebase (4 agents) | 7 docs in `.planning/codebase/` | `6849d06` |
| Initialize project | `.planning/PROJECT.md` | `f63389b` |
| Configure workflow | `.planning/config.json` (YOLO/Standard/Parallel) | `8d25c62` |
| Research (4 agents) | 5 docs in `.planning/research/` | `c728e08` |
| Define requirements | `.planning/REQUIREMENTS.md` (22 v1 reqs) | `c51ebf2` |
| Create roadmap | `.planning/ROADMAP.md` (8 phases) | `4d87fef` |
| Initialize state | `.planning/STATE.md` | `4d87fef` |

---

## Project in One Paragraph

ezkvm is a brownfield Rust CLI tool (edition 2024, saphyr YAML, no external Proxmox parser) that converts Proxmox `.conf` + `storage.cfg` files into a typed in-memory Runtime model, serializes it to ezkvm YAML, and generates QEMU commandlines. The existing Runtime is skeletal; the Proxmox and QEMU layers are scaffolding. V1 success = import `felucia/108.conf` (Windows 11 gaming VM with GPU passthrough, ivshmem, SPICE, TPM, EFI) → ezkvm YAML → working QEMU commandline.

---

## V1 Roadmap (8 Phases)

| # | Phase | Key Deliverable |
|---|-------|----------------|
| 1 | Foundation | thiserror enums + device_kind() — unblocks everything |
| 2 | Runtime Model | EfiDisk, TpmState, HostPci, Ivshmem, Audio, SPICE, RawArgs |
| 3 | Proxmox Parser | Snapshot-aware .conf + storage.cfg parser |
| 4 | Proxmox→Runtime | ProxmoxImporter TryFrom wiring |
| 5 | YAML Schema | Schema extended for all 7 v1 device types |
| 6 | YAML↔Runtime | Lossless Runtime ↔ YAML round-trip |
| 7 | QEMU Cmdline | Segment-based emitter, correct arg ordering |
| 8 | Round-Trip | felucia/108.conf → bootable VM integration test |

Phases 3, 5, 7 can start in parallel after phases 1, 2, 2 respectively.

---

## Critical Pitfalls to Remember

1. **Snapshot contamination** — split `[snapshot]` sections BEFORE any key parsing
2. **Colon in sub-options** — MACs, PCI BDFs, storage volumes defeat generic splitting
3. **args is opaque** — store and emit verbatim as `RawArgs(String)`, never parse
4. **drive-before-device** — QEMU will refuse to start if ordering is wrong
5. **thiserror first** — `type Error = ()` must be gone before any new TryFrom code

---

## Next Action

```
/gsd-plan-phase 1
```
