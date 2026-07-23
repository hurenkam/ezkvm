# Project Research Summary

**Project:** ezkvm — QEMU/Proxmox VM configuration conversion tool
**Domain:** Rust library + CLI — VM config import/export with full round-trip fidelity
**Researched:** 2026-07-22
**Confidence:** HIGH

## Executive Summary

ezkvm is a config-conversion library: it reads Proxmox `.conf` and `.qemu.cmd` files, builds a typed in-memory Runtime model, and exports back to Proxmox config or valid QEMU commandlines. The research confirms this is a well-scoped, tractable problem. The hard part is not algorithm design — it is schema completeness. The entire 60+ VM corpus from five real Proxmox hosts has been analyzed, and roughly half the Runtime types needed to represent those VMs are missing from the current codebase. The ezkvm YAML schema is reasonably complete; Runtime and both conversion pipelines (Proxmox↔Runtime, Runtime→QEMU) are largely scaffolding.

The recommended implementation order flows from strict dependency: Runtime types must exist before any conversion code can be written, Proxmox schema structs must exist before the file parser, and QEMU emission only needs Runtime (not Proxmox). The canonical test case is `input/felucia/108.conf` — a Windows 11 gaming VM with GPU passthrough, Looking Glass (ivshmem), SPICE, TPM v2.0, OVMF, and USB passthrough. This single config exercises essentially every hard feature. Passing it end-to-end is the correct first milestone.

The key risk is the `args` field in Proxmox `.conf` — it carries arbitrary QEMU flags that Proxmox doesn't model natively (SPICE config, ivshmem devices in the corpus). Research strongly recommends treating `args` as an **opaque passthrough** rather than parsing it: preserve it verbatim through Runtime and re-emit at the end of the QEMU commandline. Attempting to parse `args` into structured types introduces fragility with no reliable test oracle. All other risks are ordering/dependency problems that the phased build plan directly addresses.

---

## Key Findings

### Recommended Stack

The current Cargo.toml is already well-chosen. Two additions are recommended: `thiserror = "2.0.19"` (promote from transitive dep; needed for typed library error enums) and `shlex = "2.0.1"` (POSIX shell tokenization for `.qemu.cmd` parsing — handles quoted strings that `split_whitespace` breaks on). `winnow` should be added only if sub-option value parsing proves to require more than 2–3 irregular forms; start hand-rolled with `split_once` and migrate if needed.

**Core technologies:**
- `saphyr 0.0.11`: YAML I/O — `serde_yaml` is officially deprecated; saphyr is its successor with proper YAML 1.2 compliance and no JSON intermediary
- `hashlink 0.12.1`: Ordered HashMap — required for round-trip conf fidelity (field order preserved)
- `thiserror 2.0.19`: Library error types — already a transitive dep; promote to explicit for `ProxmoxParseError`, `QemuParseError`, `ConversionError`
- `shlex 2.0.1`: QEMU cmdline tokenization — handles POSIX quoted strings in `.qemu.cmd` files
- `winnow 1.0.4` *(conditional)*: Sub-option parser combinator — only if `split_once`-based parsing proves insufficient

**Do NOT add:** `serde_yaml` (deprecated), `anyhow` (wrong for library errors), `nom` (superseded by winnow), `regex` (overkill for delimiter-based parsing), `clap` (defer until a real CLI interface is needed).

### Expected Features

**Must have (table stakes — required for felucia/108.conf):**
- Proxmox `.conf` parser: key=value + option bag + indexed keys (`scsiN`, `netN`, `hostpciN`)
- Snapshot section handling: `[snapshot_name]` splitting before any key parsing
- BIOS/OVMF + EFI disk: `bios: ovmf` → pflash firmware setup
- TPM v2.0: `tpmstate0` → swtpm chardev in QEMU output
- PCI passthrough (vfio-pci): `hostpciN` with multi-function device support
- USB passthrough (bus:port and VID:PID addressing)
- SPICE display: comes via `args` in Proxmox; must survive passthrough
- ivshmem (Looking Glass): comes via `args` in Proxmox; must survive passthrough
- `args` passthrough: opaque preservation of arbitrary QEMU flags
- VGA type (`vga: none` suppresses emulated GPU with passthrough)
- SCSI controller variants (pvscsi/virtio-scsi-pci)
- Storage: SCSI disks with `discard`, `ssd`, `iothread`, `cache` options
- QEMU commandline generator: segment-based builder to avoid ordering errors
- ezkvm YAML round-trip: complete `TryFrom` wiring for all Runtime types

**Should have (differentiators — present in corpus but not in 108.conf):**
- NUMA + hugepages (high complexity; critical for coruscant 501.conf)
- e1000e network variant
- Memory balloon, serial port, USB tablet
- SATA/virtio disk variants
- Multiple ivshmem instances (multi-VM Looking Glass host)
- EFI ms-cert variant, CPU hidden flag, CPU flags passthrough
- QEMU `.qemu.cmd` import (reverse direction; useful for auditing)

**Defer (v2+):**
- SeaBIOS VMs (zero examples in corpus)
- VM lifecycle management, GUI, network/storage provisioning
- OVMF binary distribution

**Critical design decision on `args`:** Use Option C (hybrid) in principle but treat `args` as fully opaque in practice. Parse `args` into structured SPICE/ivshmem types only if the Proxmox schema layer needs to manipulate them. For the QEMU emitter, re-append raw `args` verbatim at cmdline end regardless.

### Architecture Approach

The architecture is a hub-and-spoke pipeline: **Runtime is the canonical in-memory hub**, and all external formats (Proxmox `.conf`, ezkvm YAML, QEMU cmdline) are spokes that convert to/from it via `TryFrom` implementations. This pattern is already established by the ezkvm layer and must be extended consistently to the Proxmox and QEMU layers. Each spoke has three sub-layers: schema structs (typed data), file I/O (parse/emit), and runtime adapters (TryFrom). The dependency rule is strict: Runtime has zero upward dependencies; schema layers depend on Runtime only through conversion impls; file I/O layers depend only on their own schema.

**Major components:**
1. **Runtime** (`src/runtime/`) — In-memory VM topology; trait-based polymorphism (`Arc<dyn Trait>`); canonical representation for all devices
2. **ezkvm Schema + File I/O** (`src/config/ezkvm/`) — YAML-serializable config structs; already functional; needs `TryFrom` wiring for advanced types
3. **Proxmox Schema** (`src/config/proxmox/schema/`) — Typed representation of `.conf` key=value format; `ProxmoxVmConf` with `BTreeMap<u8, T>` for indexed devices
4. **Proxmox File I/O** (`src/config/proxmox/file/`) — `FromStr`/`Display` for `.conf` and `storage.cfg`; section-splitting before key parsing is critical
5. **Proxmox Runtime Adapters** (`src/config/proxmox/runtime/`) — `TryFrom` in both directions
6. **QEMU Schema + Emitter** (`src/config/qemu/`) — `QemuCommandLine` as a segmented arg builder; emit segments in fixed category order to satisfy QEMU's `-netdev before -device` requirement
7. **ID Allocator** (within QEMU schema) — Assigns stable cross-reference IDs (`drive=drive-scsi0`, `netdev=net0`, `chardev=tpmchar`)

### Critical Pitfalls

*Note: No separate PITFALLS.md was produced. These are derived from ARCHITECTURE.md anti-patterns and FEATURES.md analysis.*

1. **Parsing `.conf` without section-splitting first** — `[snapshot_name]` sections contain identical key names to the active config. Naive line-by-line parsing merges active config and snapshot data into garbage. **Fix:** Always split on `[...]` headers first; parse active section and snapshot sections independently.

2. **Inline QEMU arg emission (single-pass, ad-hoc order)** — QEMU requires strict ordering: `-netdev` before `-device virtio-net-pci`, `-drive` before `-device scsi-hd`. Single-pass emission produces incorrect ordering without fragile look-ahead logic. **Fix:** Use `QemuCommandLine` segment struct (collect by category, emit in fixed category order: machine → firmware → drives → netdevs → chardevs → tpm → objects → devices → misc).

3. **Building Runtime types in isolation before Proxmox parsing** — Building all missing Runtime types (EFI, TPM, hostpci, etc.) without the Proxmox parser context risks API mismatches. The parser reveals what the conversion code actually needs. **Fix:** Interleave — build enough Proxmox schema to produce typed `ProxmoxVmConf`, then drive Runtime completeness from conversion code requirements.

4. **Treating `args` as structured data to parse** — The Proxmox `args` field is arbitrary QEMU commandline fragments; parsing it is fragile with no authoritative spec. SPICE and ivshmem devices in the corpus come through `args`. **Fix:** Store `args` as an opaque `String` in both `ProxmoxVmConf` and Runtime; re-append verbatim at the end of QEMU cmdline emission. Do not attempt to lift `args` content into structured types.

5. **Expanding `ProxmoxVmSchema` inline** — The existing `src/config/proxmox.rs` stub mixes schema, builder, and conversion concerns. Expanding it inline produces an unmaintainable monolith. **Fix:** Expand into `src/config/proxmox/` subdirectory matching the ezkvm layer's `schema/`, `file/`, `runtime/` layout.

6. **Sub-option parsing with `split(',').split('=')` naively** — Proxmox values have a positional-arg form (`vm1-pool:vm-108-boot,opt=val`) where the first token is not a key=value pair. Naive split breaks on it. **Fix:** Use `splitn(2, ',')` to separate positional token from option chain; check whether first token contains `=` to distinguish positional from pure k=v.

---

## Implications for Roadmap

### Phase 1: Runtime Completeness
**Rationale:** All conversion code (Proxmox→Runtime, Runtime→QEMU) targets Runtime types. Nothing else can be built until the missing types exist. This is the highest-leverage unblocking phase.
**Delivers:** Complete in-memory representation of all device types in the felucia/108.conf canonical test case — EfiDisk, TpmState, HostPci (vfio-pci), Ivshmem, Audio (ich9-intel-hda), SpiceDisplay, SmbiosUuid, VmGenId, RawArgs, VgaType, Balloon, Tablet
**Addresses:** All table-stakes features from FEATURES.md that are currently missing from Runtime
**Avoids:** Pitfall 3 (isolation mismatch) by letting parser requirements drive the API

### Phase 2: Proxmox Schema Structs
**Rationale:** The file parser needs typed structs to populate. Schema definition before parser implementation ensures the parser doesn't dictate struct shape in ad-hoc ways.
**Delivers:** `ProxmoxVmConf` + all sub-structs (`ProxmoxDiskConf`, `ProxmoxNetConf`, `ProxmoxHostPciConf`, `ProxmoxUsbConf`, `ProxmoxAudioConf`, `ProxmoxTpmConf`) + `ProxmoxStorageConf`
**Uses:** `BTreeMap<u8, T>` for indexed devices; `Option<String>` for `args` passthrough
**Avoids:** Pitfall 5 (monolith expansion) via `src/config/proxmox/` subdirectory layout

### Phase 3: Proxmox File Parser
**Rationale:** Parses real `.conf` files into `ProxmoxVmConf`; validates schema design against real corpus. Will drive schema adjustments before conversion code is written.
**Delivers:** `FromStr for ProxmoxVmConf` (handles section-splitting, indexed key dispatch, sub-option tokenization) + `FromStr for ProxmoxStorageConf`
**Uses:** `shlex` for QEMU cmdline tokenization if `.qemu.cmd` parsing is included here; hand-rolled `split_once` for Proxmox sub-options
**Avoids:** Pitfall 1 (section-split missing), Pitfall 6 (naive sub-option split)

### Phase 4: Proxmox → Runtime Conversion
**Rationale:** First end-to-end import pipeline. Connects Phase 2 schema to Phase 1 Runtime types. Validates that both layers are correctly designed.
**Delivers:** `TryFrom<(ProxmoxVmConf, ProxmoxStorageConf)> for Runtime` — full import of felucia/108.conf into Runtime model
**Addresses:** Proxmox import requirement from PROJECT.md
**Avoids:** Pitfall 4 (`args` treated as opaque String carried through Runtime as-is)

### Phase 5: Runtime → QEMU Commandline
**Rationale:** Can proceed after Phase 1 (Runtime only); does not need Proxmox pipeline. Delivers the primary export value — a working QEMU launch commandline.
**Delivers:** `QemuCommandLine` segmented struct + `TryFrom<(Runtime, QemuContext)> for QemuCommandLine` + `Display` emitter. Output should match `108.ezkvm.qemu.cmd` (canonical reference).
**Uses:** Segment-based builder (machine/firmware/drives/netdevs/chardevs/tpm/objects/devices/misc), stable ID allocator for cross-references
**Avoids:** Pitfall 2 (inline arg ordering failures)

### Phase 6: Runtime → Proxmox Export + ezkvm YAML Round-Trip
**Rationale:** Completes the full round-trip. Proxmox export re-uses the Phase 2 schema structs. ezkvm YAML wiring completes `TryFrom` impls for advanced Runtime types.
**Delivers:** `TryFrom<Runtime> for ProxmoxVmConf` + `Display for ProxmoxVmConf` + complete ezkvm YAML → Runtime → QEMU pipeline with all Phase 1 types wired
**Addresses:** Round-trip fidelity requirement from PROJECT.md

### Phase 7: NUMA, Hugepages, and Full Corpus Coverage
**Rationale:** Extends beyond felucia/108.conf to the wider corpus. High complexity (NUMA changes the QEMU memory model significantly). Deferred until Phase 4–6 are validated.
**Delivers:** NUMA topology, hugepages memory backends, e1000e network, serial port, virtio disk, SATA disk, multiple ivshmem, extended CPU flags
**Addresses:** Differentiator features from FEATURES.md

### Phase 8: QEMU `.qemu.cmd` Import (Optional)
**Rationale:** Reverse-parse `.qemu.cmd` files into Runtime. Useful for auditing Proxmox's actual behavior vs. its `.conf` representation. Lowest priority; all conversion goals are met without it.
**Delivers:** `FromStr for QemuCommandLine` + `TryFrom<QemuCommandLine> for Runtime`
**Uses:** `shlex` for tokenization (already added in Phase 3)

### Phase Ordering Rationale

- **Runtime first** (Phase 1): Every other phase writes code against Runtime types. Building without them causes rework.
- **Schema before parser** (Phase 2 before 3): Typed structs give the parser a clear target; the parser won't dictate ad-hoc struct shapes.
- **Parser before conversion** (Phase 3 before 4): Real corpus parsing validates that schema design is correct. Schema adjustments before conversion code prevent double rework.
- **QEMU emission independent** (Phase 5 can start after Phase 1): QEMU emission only needs Runtime; it doesn't depend on Proxmox pipeline completion. These can be parallelized.
- **Export and YAML wiring deferred** (Phase 6): Building after Phase 4 validates Runtime API correctness via the import path first.
- **NUMA/hugepages last** (Phase 7): Significant architectural complexity (NUMA changes memory model); deferred until core pipeline is validated.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 5 (QEMU cmdline):** QEMU argument ordering, pflash firmware setup, blockdev JSON format for modern QEMU — compare `108.qemu.cmd` vs `108.ezkvm.qemu.cmd` diff carefully before planning
- **Phase 7 (NUMA/hugepages):** NUMA memory backend file format, hugepage paths, `-numa` flag interactions — significant QEMU model change with sparse documentation

Phases with standard patterns (skip research-phase):
- **Phase 1 (Runtime types):** Pattern is established — follow existing `SSD`/`HDD`/`PvScsi` as templates; add missing types by analogy
- **Phase 2 (Proxmox schema structs):** Pattern is fully specified in ARCHITECTURE.md with code examples; straightforward implementation
- **Phase 3 (Proxmox parser):** Algorithm fully specified in STACK.md and ARCHITECTURE.md with code samples; no ambiguity
- **Phase 6 (ezkvm YAML round-trip):** ezkvm schema is already mostly complete; wiring is mechanical `TryFrom` impl work

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All libraries verified against crates.io; serde_yaml deprecation confirmed; real corpus validates format requirements |
| Features | HIGH | Derived from direct analysis of 60+ real Proxmox VM configs; canonical test case (108.conf) fully analyzed |
| Architecture | HIGH | Entire codebase directly read; ezkvm layer provides a proven template; all gaps identified with code-level specificity |
| Pitfalls | HIGH | All pitfalls derived from direct code/corpus analysis, not speculation; each has a concrete prevention strategy |

**Overall confidence:** HIGH

### Gaps to Address

- **`args` parsing scope**: Research recommends full opaque passthrough, but the existing `PcieDeviceTypeSchema` has structured `IvshmemPlain` types suggesting a prior intent to lift `args` content. Resolve this intent early in Phase 1 before writing `TryFrom` impls.
- **`storage.cfg` format**: The storage configuration file format has been identified as needed (for resolving `pool:volume` references to host paths) but not fully analyzed. Plan Phase 3 with a spike on `storage.cfg` parsing before committing to the schema.
- **QEMU context object**: `QemuContext` (carries vm_name, socket paths, storage paths) is named in ARCHITECTURE.md but not yet defined. Its exact contents will be determined by what the QEMU emitter needs — let Phase 5 planning define it.
- **`winnow` decision point**: Whether sub-option parsing requires `winnow` or can stay hand-rolled will be determined during Phase 3 implementation. No action needed before then.

---

## Sources

### Primary (HIGH confidence)
- Direct codebase read: `src/runtime/`, `src/config/ezkvm/`, `src/config/proxmox.rs`, `src/config/qemu.rs` — architecture and current-state findings
- Corpus analysis: `input/felucia/108.conf`, `108.qemu.cmd`, `108.ezkvm.qemu.cmd`, `storage.cfg` — canonical test case
- Extended corpus: `input/coruscant/501.conf`, `100.conf`, `530.conf`, `zbp-server-mh2/201.conf`, `301.conf` — differentiator feature coverage
- crates.io API (verified 2026-07-22): winnow 1.0.4, shlex 2.0.1, thiserror 2.0.19, saphyr 0.0.11

### Secondary (MEDIUM confidence)
- `serde_yaml` deprecation: confirmed via crates.io version string `0.9.34+deprecated`; saphyr identified as community successor
- `winnow` vs `nom`: winnow 1.0.0 released 2026-03-17; same author (Epage); documented as nom successor for new code

### Tertiary (for validation during planning)
- QEMU `-blockdev` JSON format: inferred from `.qemu.cmd` corpus; needs validation against QEMU docs during Phase 5 planning
- `storage.cfg` format: inferred from `input/felucia/storage.cfg`; needs full format analysis during Phase 3

---
*Research completed: 2026-07-22*
*Ready for roadmap: yes*
