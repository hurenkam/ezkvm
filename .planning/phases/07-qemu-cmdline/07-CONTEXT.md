# Phase 7: QEMU Cmdline - Context

**Gathered:** 2026-07-24
**Status:** Ready for planning

<domain>
## Phase Boundary

The QEMU commandline emitter converts a fully populated `Runtime` (plus a `QemuContext` carrying vm identity and resolved paths) into a correctly ordered, valid `qemu-system-x86_64` argument list. It covers every root device Runtime can hold today — not just the seven v1 types added in Phase 2 — with drives and netdevs always emitted before the `-device` arguments that reference them, and `RawArgs` appended verbatim at the end.

</domain>

<decisions>
## Implementation Decisions

### Device coverage scope
- **D-01:** `QemuCommandLine::try_from((Runtime, QemuContext))` dispatches over ALL root devices via `device_kind()`, not just the seven v1 types (EfiDisk, TpmState, HostPci, Ivshmem, AudioDevice, SpiceDisplay, RawArgs). Pre-existing types (`Memory`, `Q35Chipset`, storage, network, USB) get their own emit handlers in this phase since `src/config/qemu.rs` is currently only a stub (`NoHandler` error, no working handlers). — **Reversibility:** costly — the segment/dispatch structure and per-device handler map is the foundation later phases (8, 9) build on; changing device coverage after Phase 9's integration test locks in expected output would require touching every handler.

### Base/machine-level args scope
- **D-02:** Args needed to make the emitted cmdline actually runnable for testing are in scope now: `-machine` (from `Q35Chipset`), `-m` (from `Memory`), `-smp`, `-cpu`. These derive from Runtime device data via the same dispatch mechanism as D-01.
- **D-03:** Host/process-management args are explicitly deferred to Phase 8 (VM Lifecycle), which owns process launching: `-id`, `-name`, `-smbios type=1,uuid=...`, `-pidfile`, `-daemonize`, `-readconfig`. Phase 7 does not emit these.

### Testing strategy
- **D-04:** Use the existing `input/felucia/108.conf` corpus (via the Phase 3/4 Proxmox parser + importer) as a golden-fixture Runtime source for an ordering/coverage test, per the existing plan bullet 07-04. Combine with synthetic per-device unit tests for individual handler correctness.
- **D-05:** The felucia-based test asserts **structural correctness only** — presence of expected drive/netdev/device tokens and correct relative ordering (drive-before-device, netdev-before-device) — NOT byte-for-byte match against `input/felucia/108.qemu.cmd`. ezkvm's generated cmdline is expected to differ from Proxmox's `qm`-generated one (different flags, different paths). Exact-match / boot verification is Phase 9's job (QEMU-04).

### Error handling
- **D-06:** `QemuContext` is built internally by ezkvm (not parsed from untrusted input). Missing context data the emitter needs (e.g., an unset socket or storage path) is a programmer error — panic/`.expect()` with a clear message, not a typed `QemuConversionError` variant. Typed errors remain reserved for genuine Runtime-data conversion problems (matching the existing thiserror convention from Phase 1).

### Boot order / bootindex data model (found during research — RESEARCH.md Critical Gap)
- **D-07:** Research confirmed no boot-order data exists anywhere in the model: `ProxmoxVmConf.boot: Option<String>` is parsed but never read by `ProxmoxImporter::into_runtime()`, and `Runtime` has no boot-order field. Decision: implement full plumbing now, in this phase — add `boot_order: Vec<String>` to `Runtime`, update `ProxmoxImporter` to parse Proxmox's `order=a;b;c` string into that list, and implement device-id reconstruction (mapping Runtime bus addresses like `ScsiAddress{target,lun}` back to label strings like `"scsi0"`) plus `bootindex` assignment (starting at 100, incrementing by 1 per boot-order position; devices absent from boot order omit `bootindex`). — **Reversibility:** costly — touches Phase 4's importer output and adds a new Runtime field; deferring this later would mean revisiting completed-phase code under time pressure instead of now while the gap is freshly understood.

### Architectural boundary: Runtime `Display` impls are not a config-emission API
- **D-08:** Several `Runtime` domain types (`Memory`, `Chipset`, `Q35Chipset`, `PvScsi`, and the bus address types `PcieAddress`/`ScsiAddress`/`IdeAddress`/`SataAddress`/`PciAddress`) implement `std::fmt::Display` for their own purposes (debug/logging/internal rendering), and these impls are subject to change independently of QEMU output requirements. No type or method anywhere in the `src/config` tree (this phase's `QemuCommandLineBuilder`/handlers included) may rely on a `Runtime` type's `Display`/`.to_string()` output to construct or parse QEMU argument strings. All data must be read from `Runtime` types via their typed getters (e.g. `.size()`, `.device()`, `.port()`, `.target()`, `.lun()`, `.channel()`, `.mode()`, `.sockets()`, `.cores()`, `.cpu_type()`) and formatted explicitly by the emitter itself. The only `Display` impl the `src/config/qemu` tree may define and depend on is `QemuCommandLine`'s own (a new type introduced by this phase, not a `Runtime` type). Where a plan references `Q35Chipset::fmt`'s "sort-then-render" idiom (Plan 07-02), this is citing an *algorithmic pattern* to replicate (sort bus-map entries by address key before iterating, since `HashMap` order isn't stable) — not a code dependency on `Q35Chipset`'s `Display` trait implementation itself; the emitter must re-implement its own sort and its own string formatting independently. — **Reversibility:** cheap now (verified: no plan in this phase currently violates this) — but codified here so future phases/reviewers keep the `Runtime` (domain) and `src/config` (serialization/emission) layers decoupled; a `Display` impl on a `Runtime` type may change for unrelated (e.g. debug-printing) reasons at any time without notice to `src/config`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap and requirements
- `.planning/ROADMAP.md` §"Phase 7: QEMU Cmdline" — goal, success criteria, pitfalls (drive/netdev ordering, bootindex), and existing plan bullets 07-01 through 07-04
- `.planning/REQUIREMENTS.md` — QEMU-01, QEMU-02, QEMU-03 traceability (QEMU-04 belongs to Phase 9)

### Prior phase outputs
- `.planning/phases/06-yaml-runtime/06-SUMMARY.md` — Runtime construction/round-trip surface this phase consumes
- `.planning/phases/02-runtime-model/` — SUMMARY for the seven v1 device types and their trait implementations
- `.planning/phases/01-foundation/` — SUMMARY for `device_kind()` dispatch pattern and thiserror error-enum convention this phase must follow

### Existing code
- `src/config/qemu.rs` — current stub (`QemuSchemaBuilder`, `QemuConversionError`, `NoHandler`); this phase replaces/extends it with the segmented `QemuCommandLine` struct
- `src/runtime/q35.rs`, `src/runtime/memory.rs`, `src/runtime/chipset.rs` — pre-existing root device types needing QEMU emit handlers
- `input/felucia/108.conf`, `input/felucia/108.qemu.cmd`, `input/felucia/108.qemu.cmd.split` — golden-fixture corpus for the structural ordering test

No additional external specs or ADRs — requirements fully captured in decisions above.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/config/qemu.rs` — existing `QemuConversionError` (thiserror-based) and `TypeId`-keyed handler map pattern can be extended rather than rebuilt from scratch, though the handler signature will need to change to support ordered segment collection instead of single-pass emission.
- Phase 1's `device_kind()` trait method — use for dispatch instead of the current `TypeId`/`downcast`-adjacent approach in the stub.

### Established Patterns
- `TryFrom` conversion pattern used consistently across proxmox.rs, qemu.rs, and Runtime↔YAML layers — `QemuCommandLine: TryFrom<(Runtime, QemuContext)>` follows this.
- thiserror-based typed error enums (Phase 1 convention) — reserved for Runtime-data conversion failures per D-06, not internal QemuContext invariants.

### Integration Points
- Input: `Runtime` (from Phases 2–6) + new `QemuContext` (vm_name, socket_paths, storage_paths) constructed by the caller (eventually Phase 8).
- Output: `QemuCommandLine` implementing `Display`, emitting segments (`machine`, `firmware`, `drives`, `netdevs`, `chardevs`, `tpm`, `objects`, `devices`, `misc`) in fixed order, consumed by Phase 8 (process launch) and validated end-to-end by Phase 9.

</code_context>

<specifics>
## Specific Ideas

No additional requirements beyond the locked decisions above — the ROADMAP's existing plan bullets (07-01 through 07-04) already specify the segmented struct shape, TryFrom emission order, RawArgs verbatim append, and bootindex derivation (starting at 100). This discussion resolved the gaps the roadmap left open: full device coverage, base-arg scope boundary with Phase 8, testing approach, and error-handling strictness.

</specifics>

<deferred>
## Deferred Ideas

- Host/process-management args (`-id`, `-name`, `-smbios`, `-pidfile`, `-daemonize`, `-readconfig`) — belongs in Phase 8 (VM Lifecycle).
- Byte-for-byte / boot-verified cmdline correctness against real Proxmox output — belongs in Phase 9 (Round-Trip Verification, QEMU-04).

None else — discussion stayed within phase scope.

</deferred>

---

*Phase: 07-qemu-cmdline*
*Context gathered: 2026-07-24*
