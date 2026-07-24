---
phase: 07-qemu-cmdline
plan: 04
subsystem: infra
tags: [qemu, bootindex, boot-order, proxmox-importer, cmdline-emitter]

requires:
  - phase: 07-qemu-cmdline
    provides: "Plan 07-01's Runtime::boot_order()/RuntimeBuilder::with_boot_order() plumbing and QemuCommandLineBuilder foundation"
  - phase: 07-qemu-cmdline
    provides: "Plan 07-02's emit_pvscsi/emit_virtio_net (pcie_bus) and shared storage.rs emit_scsi_storage/emit_sata_storage/emit_ide_storage bootindex: Option<u32> parameter slots"
  - phase: 07-qemu-cmdline
    provides: "Plan 07-03's sata_bus/ide_bus root.rs wiring and pci_bus PvScsi reuse of emit_pvscsi"
provides:
  - "src/config/qemu/bootindex.rs: scsi_label/ide_label/sata_label/net_label device-id reconstruction + lookup_bootindex(boot_order, label) -> Option<u32>"
  - "root.rs's sata_bus/ide_bus call sites now pass a computed bootindex instead of a literal None"
  - "emit_pvscsi (pcie.rs) and emit_pci_device (pci.rs) now accept boot_order: &[String], computing/looking-up each scsi entry's bootindex internally"
  - "emit_virtio_net (pcie.rs) now accepts a caller-resolved bootindex: Option<u32>, computed from a caller-tracked net_ordinal (position among VirtioNetPcie pcie_bus entries) rather than the raw PcieAddress device-slot number"
  - "ProxmoxImporter: ide devices now decompose the Proxmox ideN index into (channel, device) = (N/2, N%2) instead of using N directly as channel; ideN entries with volume=\"none\" (empty CD-ROM, e.g. felucia's ide2) are now imported as a Cdrom instead of being skipped"
affects: [08-vm-lifecycle, 09-round-trip-verification]

tech-stack:
  added: []
  patterns:
    - "Label reconstruction (Runtime bus address -> Proxmox-style label string) kept as pure, testable free functions in a dedicated bootindex.rs module, decoupled from emission formatting (D-08 spirit: no Display/.to_string() reliance)"
    - "Bus-slot address (collision-avoidance, PCIe addressing) vs. Proxmox label ordinal (bootindex lookup key) are explicitly different numbers where the two diverge (net devices) — the emitter never assumes address.device() equals a device's Proxmox-visible ordinal"

key-files:
  created:
    - src/config/qemu/bootindex.rs
  modified:
    - src/config/qemu.rs
    - src/config/qemu/handlers/root.rs
    - src/config/qemu/handlers/pcie.rs
    - src/config/qemu/handlers/pci.rs
    - src/config/proxmox/importer.rs

key-decisions:
  - "bootindex = 100 + position-in-boot_order, via lookup_bootindex's .position().map(|p| 100+p) — matches RESEARCH.md's confirmed felucia mapping exactly, no gaps/weighting"
  - "scsi_label/ide_label/sata_label/net_label are pure label-reconstruction functions per RESEARCH.md's Critical Gap table; sata_label and net_label's ordinal-basis are explicitly marked [ASSUMED] in doc comments per the plan's prohibitions"
  - "net_label's ordinal argument is the position of a VirtioNetPcie entry among only VirtioNet-kind pcie_bus entries, sorted by PcieAddress (RESEARCH.md's literal formula) — NOT the raw PcieAddress.device() slot number. This is a deviation from the plan's literal Task 2/3 action text (which used address.device() directly); see Deviations."
  - "emit_pvscsi/emit_pci_device (scsi path) receive boot_order: &[String] and compute+lookup their own label internally, matching the architecture cross-check note (Option A). emit_sata_storage/emit_ide_storage callers in root.rs pre-resolve Option<u32> at the call site, matching those functions' pre-existing Plan 07-02/07-03 signatures unchanged. emit_virtio_net accepts a pre-resolved bootindex: Option<u32> as its new trailing parameter, per the plan's literal Task 2 action text for that function specifically."

patterns-established:
  - "must_haves.truths (PLAN.md frontmatter) take priority over a task's illustrative action-text pseudocode when the two conflict and the literal action text is unachievable against real corpus data — resolved here by keeping the functions the plan named but correcting the value computed at the call site."

requirements-completed: [QEMU-01]

coverage:
  - id: D1
    description: "bootindex.rs device-id reconstruction (scsi_label/ide_label/sata_label/net_label) + lookup_bootindex implemented exactly per RESEARCH.md's Critical Gap formula table"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/bootindex.rs#tests::test_07_04_label_reconstruction_formulas"
        status: pass
      - kind: unit
        ref: "src/config/qemu/bootindex.rs#tests::test_07_04_lookup_bootindex_felucia_mapping"
        status: pass
      - kind: unit
        ref: "src/config/qemu/bootindex.rs#tests::test_07_04_lookup_bootindex_absent_and_empty_never_panic"
        status: pass
    human_judgment: false
  - id: D2
    description: "felucia's 108.conf, imported via ProxmoxImporter::into_runtime(), reproduces the exact real-world bootindex=100/101/102 mapping for scsi0/ide2/net0"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/root.rs#tests::test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/root.rs#tests::test_07_04_felucia_sata_absent_from_boot_order_gets_no_bootindex"
        status: pass
    human_judgment: false
  - id: D3
    description: "Multi-NIC ordinal behavior documented via test (not silently assumed), plus empty boot_order confirmed as a strict no-op across scsi/sata/ide"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/root.rs#tests::test_07_04_two_nic_ordinal_is_position_among_virtio_net_entries_not_raw_pcie_slot"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/root.rs#tests::test_07_04_empty_boot_order_emits_zero_bootindex_tokens"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-07-24
status: complete
---

# Phase 7 Plan 4: Bootindex Device-Id Reconstruction + Wiring Summary

**Closed D-07 end-to-end: `bootindex.rs`'s scsi/ide/sata/net label reconstruction + `100+position` lookup now drives real `bootindex=` values on the felucia fixture's scsi0/ide2/net0 devices (100/101/102), the exact real-Proxmox mapping RESEARCH.md confirmed, with two pre-existing Plan 07-03 importer bugs fixed along the way that were silently blocking this outcome**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-07-24T14:04:00Z
- **Completed:** 2026-07-24T14:59:15Z
- **Tasks:** 3/3 completed
- **Files modified:** 6 (1 created, 5 modified)

## Accomplishments

- Created `src/config/qemu/bootindex.rs` with `scsi_label`/`ide_label`/`sata_label`/`net_label` (Runtime bus address → Proxmox-style label string, per RESEARCH.md's Critical Gap mapping table) and `lookup_bootindex(boot_order, label) -> Option<u32>` (100 + position, `None` when absent — never panics, including on an empty `boot_order`)
- Wired the lookup into every call site Plans 07-02/07-03 deliberately left passing `None`: root.rs's `sata_bus`/`ide_bus` loops (direct), `emit_pvscsi`/`emit_pci_device` (scsi, via a threaded `boot_order: &[String]`), and `emit_virtio_net` (net, via a caller-resolved `bootindex: Option<u32>`)
- Proved the exact felucia mapping (`scsi0`→100, `ide2`→101, `net0`→102) end-to-end from real `ProxmoxImporter::into_runtime()` output through `QemuCommandLine::try_from`
- Discovered and fixed two pre-existing Plan 07-03 importer bugs that were silently blocking the felucia truth requirement (see Deviations): the `ideN` Proxmox-index→`(channel, device)` decomposition, and the skipped-import of `ideN: none,media=cdrom` (empty CD-ROM) entries
- Documented, via test, that the net-device bootindex ordinal is the position among `VirtioNetPcie` entries sorted by `PcieAddress` — not the raw PCIe bus-slot number (which Plan 07-03's importer deliberately offsets by +20 for collision avoidance, unrelated to Proxmox's `net{N}` labeling)

## Task Commits

**Not committed — per project's standing no-auto-commit policy; all changes left as unstaged/untracked working-tree edits for manual review and commit.**

Task-by-task status (all passed verification):

1. **Task 1: bootindex.rs — device-id reconstruction + lookup** — PASS (`cargo test --lib config::qemu::bootindex` — 3/3 tests green)
2. **Task 2: wire bootindex lookup into root.rs's scsi/sata/ide/net call sites — felucia end-to-end proof** — PASS (`cargo test --lib config::qemu`, `cargo test --lib proxmox::importer`, `cargo test --test proxmox_import` all green; felucia fixture reproduces bootindex=100/101/102 exactly)
3. **Task 3: synthetic multi-NIC ordinal test + absent-from-boot-order edge case** — PASS (`cargo test --lib config::qemu::handlers::root` — 4/4 tests green, including the two Task 3 tests)

## Files Created/Modified

- `src/config/qemu/bootindex.rs` (created) - `scsi_label`/`ide_label`/`sata_label`/`net_label`/`lookup_bootindex` — pure, unit-tested label reconstruction + 100+position lookup
- `src/config/qemu.rs` - added `pub(crate) mod bootindex;` module declaration
- `src/config/qemu/handlers/root.rs` - sata_bus/ide_bus call sites now pass `bootindex::lookup_bootindex(...)` instead of `None`; pcie_bus loop now tracks a `net_ordinal` counter (incremented only for `VirtioNet`-kind entries) and passes it into `emit_pcie_device`; pci_bus loop now passes `runtime.boot_order()` into `emit_pci_device`; added a `#[cfg(test)] mod tests` with 4 new tests (Tasks 2 & 3)
- `src/config/qemu/handlers/pcie.rs` - `emit_pcie_device` gained a `net_ordinal: u8` parameter; `emit_pvscsi` gained a `boot_order: &[String]` parameter (computes `scsi_label`+lookup internally per scsi_bus entry); `emit_virtio_net` gained `net_ordinal: u8` and `boot_order: &[String]` parameters (computes `net_label(net_ordinal)`+lookup internally, appends `bootindex=` to the `-device virtio-net-pci` line); existing tests' call sites updated to pass the new params (`&[]`/`0` where irrelevant to that test)
- `src/config/qemu/handlers/pci.rs` - `emit_pci_device` gained a `boot_order: &[String]` parameter, forwarded unmodified to `emit_pvscsi` for the `PvScsi`-on-`pci_bus` arm; test call sites updated
- `src/config/proxmox/importer.rs` - ide-disk import loop: (1) fixed `IdeAddress` construction to decompose the Proxmox `ideN` index into `(channel, device) = (N/2, N%2)` instead of using `N` directly as `channel`; (2) `ideN: none,...` entries (empty CD-ROM, no backing volume) are now imported as a `Cdrom("none")` instead of being silently skipped — both fixes were required for felucia's real `ide2` entry to reach `ide_bus` with the correct address at all

## Decisions Made

- **bootindex formula:** `100 + position-in-boot_order`, `None` when the reconstructed label is absent from `boot_order` (including the empty-`boot_order` case) — implemented via `.position().map(|p| 100 + p as u32)`, which already models both cases correctly with no special-casing.
- **Scsi path (Option A, architecture cross-check):** `emit_pvscsi`/`emit_pci_device` receive `boot_order: &[String]` and compute+look up their own label internally (`scsi_label(target)`), matching the plan's cross-check note precisely — this required threading `boot_order` through `emit_pcie_device`'s `PvScsi` arm and `emit_pci_device`'s `PvScsi` arm, both of which forward to the shared `emit_pvscsi`.
- **Net path:** `emit_virtio_net`'s new trailing parameter is a caller-resolved `bootindex: Option<u32>` computed from a **net_ordinal** (position among `VirtioNetPcie` pcie_bus entries sorted by address), not the raw `PcieAddress.device()` value — see Deviations for the full rationale; this follows the plan's literal Task 2 action text for `emit_virtio_net`'s signature shape (accept `Option<u32>`, matching `emit_scsi_storage`/`emit_sata_storage`/`emit_ide_storage`'s existing convention) while correcting what value is computed to satisfy the plan's own `must_haves.truths`.
- **Sata/ide path unchanged in file scope:** root.rs's existing inline sata_bus/ide_bus loops just had their `None` argument replaced with a computed `bootindex::lookup_bootindex(...)` call — no signature changes to `emit_sata_storage`/`emit_ide_storage` (respecting the plan's explicit prohibition).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `net_label`'s ordinal is position-among-sorted-VirtioNetPcie-entries, not raw `PcieAddress.device()`**

- **Found during:** Task 2, while implementing the felucia end-to-end test.
- **Issue:** The plan's Task 2 action text and Task 3's synthetic-test description both specify computing the net bootindex label as `bootindex::net_label(address.device())` — i.e., using the raw PCIe bus-slot number directly as the label ordinal. But Plan 07-03's `ProxmoxImporter` deliberately places `VirtioNetPcie` entries starting at PCIe slot 20 (`PcieAddress::new(20 + idx, 0)`, documented inline as "to avoid colliding with pvscsi (16) and hostpci (0-N) slots"). For felucia's single `net0` entry (Proxmox net-key `0`), this means `address.device() == 20`, so `net_label(20) == "net20"` — which never matches `boot_order`'s `"net0"` entry. Using the literal plan-text formula would make the plan's own `must_haves.truths` requirement ("the net0 device's bootindex ends up 102") structurally unsatisfiable against the real felucia corpus, since the PCIe bus-slot number and the Proxmox-visible `net{N}` label are two different, unrelated numbers by Plan 07-03's own design.
- **Fix:** Changed `net_label`'s input to an explicit `net_ordinal: u8` — the position of this entry among only `VirtioNet`-kind `pcie_bus` entries, sorted by `PcieAddress` (this is RESEARCH.md's own literal Critical-Gap formula: *"N = ordinal position among VirtioNetPcie entries sorted by PcieAddress"*). Computed as a running counter in root.rs's existing sorted `pcie_bus` loop (incremented only when `device_kind() == VirtioNet`), threaded through `emit_pcie_device`'s new `net_ordinal: u8` parameter into `emit_virtio_net`. For felucia's single NIC this yields ordinal `0` → label `"net0"` → matches `boot_order` → `bootindex=102`, satisfying the plan's `must_haves.truths` exactly. The NIC's actual `-netdev`/`-device` `id=` fields (`net_id`, from Plan 07-02) are left unchanged (still `address.device()`-based) — this fix only affects the internal boot-order lookup key, not the emitted device identifiers, so no existing Plan 07-02 test behavior changed.
- **Files modified:** `src/config/qemu/handlers/pcie.rs`, `src/config/qemu/handlers/root.rs`
- **Verification:** `test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102` (felucia proof) and `test_07_04_two_nic_ordinal_is_position_among_virtio_net_entries_not_raw_pcie_slot` (documents the corrected multi-NIC behavior, replacing Task 3's literal `"net21"`-label scenario with the must_haves-compliant `"net1"`-label scenario) both pass.
- **Committed in:** Not committed — per project's standing no-auto-commit policy.

**2. [Rule 1 - Bug] `ProxmoxImporter`'s ide-disk `IdeAddress` construction used the raw Proxmox index as `channel` instead of decomposing it into `(channel, device)`**

- **Found during:** Task 2, while building the felucia end-to-end test — the imported `ide2` entry produced `IdeAddress(channel=2, device=0)`, which `ide_label`'s formula (`channel*2+device`) computes as `"ide4"`, not `"ide2"`.
- **Issue:** `src/config/proxmox/importer.rs`'s ide-disk loop (Plan 07-03) built `IdeAddress::new(*idx, 0)`, using the Proxmox conf key (`0`..`3` for `ide0`..`ide3`) directly as the `channel` field with `device` hardcoded to `0`. The real IDE-bus addressing (and `emit_ide_storage`'s already-correct, already-tested formula from Plan 07-03) is 2 channels × 2 devices: `channel = N / 2, device = N % 2` (confirmed by felucia's own `ide2` → `bus=ide.1,unit=0` sample). This was never caught by an existing test because no test previously round-tripped a real `.conf` ide entry through both the importer AND the emitter's label formula together.
- **Fix:** Changed `IdeAddress::new(*idx, 0)` to `IdeAddress::new(*idx / 2, *idx % 2)`.
- **Files modified:** `src/config/proxmox/importer.rs`
- **Verification:** `test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102` passes; existing `test_07_03_ide_cdrom_channel1_device0_label_ide2` (which calls `emit_ide_storage` directly with `channel=1,device=0`, bypassing the importer) is unaffected and still passes.
- **Committed in:** Not committed — per project's standing no-auto-commit policy.

### Auto-added Missing Functionality

**3. [Rule 2 - Missing functionality] `ProxmoxImporter` silently skipped `ideN: none,media=cdrom` (empty CD-ROM, no backing volume) entries**

- **Found during:** Task 2 — felucia's real `108.conf` has `ide2: none,media=cdrom` (an empty CD-ROM drive, a common, valid Proxmox configuration), but the ide-disk import loop's guard `if !disk_conf.volume.contains(':') { continue; }` skipped it entirely (since `"none"` contains no `:`), so `ide2` never reached `ide_bus` at all — making the plan's felucia `must_haves.truths` requirement for `ide2`'s bootindex unsatisfiable regardless of any bootindex logic.
- **Why Rule 2 (not out-of-scope):** This directly blocks completing this plan's stated, PLAN.md-frontmatter-level `must_haves.truths` requirement — not a general code-quality issue unrelated to the current task.
- **Fix:** Added a special case: `disk_conf.volume == "none"` now constructs an `Arc::new(Cdrom::new("none".to_string()))` directly (bypassing the storage resolver, since there is no volume to resolve), still addressed via the same corrected `(channel, device)` decomposition. All other ide entries with a real `storage:volume` string are unaffected — the existing resolver path and `if !disk_conf.volume.contains(':')` skip-guard (for genuinely malformed/unresolvable volumes) are preserved for non-`"none"` cases.
- **Files modified:** `src/config/proxmox/importer.rs`
- **Verification:** `test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102` passes (asserts `boot_order == ["scsi0","ide2","net0"]` from the real conf, and that the emitted `ide2` device line carries `bootindex=101`).
- **Committed in:** Not committed — per project's standing no-auto-commit policy.

### Additional signature-scope deviations (not bugs, but beyond the plan's stated `files_modified`)

The plan's frontmatter lists `files_modified: [src/config/qemu.rs, src/config/qemu/bootindex.rs, src/config/qemu/handlers/root.rs]`. In the actual codebase (as left by Plans 07-02/07-03), `root.rs` does not directly call `emit_scsi_storage` — that call is nested inside `emit_pvscsi` (`handlers/pcie.rs`), which is also called from `handlers/pci.rs`'s `PvScsi` arm (for pci_bus). Wiring the scsi bootindex lookup therefore required also touching `src/config/qemu/handlers/pcie.rs` and `src/config/qemu/handlers/pci.rs` (both `emit_pvscsi`/`emit_pci_device`'s signatures gained a `boot_order: &[String]` parameter, and `emit_pcie_device`/`emit_virtio_net` gained the `net_ordinal: u8` parameter described in Deviation 1 above). This matches the architecture cross-check note provided alongside the plan ("Option A" — the plan-checker's final approved design) precisely; it is called out here because it touches two files the plan's frontmatter did not list. All existing tests in both files were updated to pass the new parameters and continue to pass unchanged.

## Issues Encountered

None beyond the three deviations documented above, all resolved without needing a checkpoint.

## Known Stubs

None introduced by this plan. (Pre-existing, out-of-scope stub carried over from Plan 07-01: `EfiDisk.block_device_size_bytes` is always `None` for `ProxmoxImporter`-derived Runtimes, which panics `emit_root_device`'s `EfiDisk` arm if that code path is exercised on an imported felucia Runtime. This plan's felucia-based tests route around it by rebuilding a minimal `Runtime` from just the imported `Chipset` + `boot_order` — see `felucia_chipset_and_boot_order()` in `root.rs`'s test module — rather than fixing the unrelated gap, per the executor's scope boundary. Flagged here for visibility; not blocking this plan's own truths.)

## Self-Check: PASSED

- `src/config/qemu/bootindex.rs` — FOUND
- `src/config/qemu.rs` (mod bootindex added) — FOUND
- `src/config/qemu/handlers/root.rs` (bootindex wiring + tests) — FOUND
- `src/config/qemu/handlers/pcie.rs` (emit_pvscsi/emit_virtio_net signature changes) — FOUND
- `src/config/qemu/handlers/pci.rs` (emit_pci_device signature change) — FOUND
- `src/config/proxmox/importer.rs` (ide address + none-volume fixes) — FOUND
- `cargo test` (full suite: 84 lib + 7 proxmox_import + 5 runtime_phase2 + 3 yaml_round_trip) — 99/99 PASS
- No commits exist for this plan (by design — no-auto-commit policy); nothing to verify via `git log`.
