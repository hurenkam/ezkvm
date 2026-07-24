---
phase: 07-qemu-cmdline
plan: 05
subsystem: testing
tags: [qemu, cmdline-emitter, integration-test, ordering-proof, felucia-fixture, bootindex]

requires:
  - phase: 07-qemu-cmdline
    provides: "Segmented QemuCommandLine emitter (Plan 07-01), scsi/sata/ide/hostpci/ivshmem/net emission (Plans 07-02/07-03), bootindex reconstruction + wiring (Plan 07-04)"
provides:
  - "tests/qemu_cmdline.rs: reusable assert_precedes helper + felucia golden-fixture drive-before-device ordering proof + RawArgs-verbatim-at-end regression proof + synthetic netdev-before-device ordering proof (closes RESEARCH.md Pitfall 7)"
affects: []

tech-stack:
  added: []
  patterns:
    - "assert_precedes(haystack, needle_before, needle_after) — position-based ordering assertion, reused across every drive/device and netdev/device check in this file (not reimplemented per-test)"
    - "Route around a pre-existing importer gap by re-registering all-but-one root device kind into a fresh Runtime via Runtime::new() + register_root_device(), rather than filtering/rebuilding via RuntimeBuilder's typed with_X chain (07-04 already established a narrower version of this pattern for Chipset+boot_order only; this generalizes it to arbitrary root device subsets)"

key-files:
  created:
    - tests/qemu_cmdline.rs
  modified: []

key-decisions:
  - "felucia_runtime_for_cmdline() test helper re-registers every imported root device EXCEPT EfiDisk into a fresh Runtime (dropping boot_order in the process, which this plan's assertions don't depend on) — this routes around a pre-existing, already-documented gap (ProxmoxImporter never resolves EfiDisk.block_device_size_bytes; emit_root_device's EfiDisk arm .expect()-panics without it), rather than fixing that unrelated importer gap, which is out of this plan's scope."
  - "The RawArgs-at-end assertion resolves the 'last device token' from QemuCommandLine's own devices() segment getter (via cmdline.devices().last()), not from a generic output.rfind(\"-device\") over the full rendered string — the felucia fixture's own RawArgs blob contains multiple embedded '-device ...' substrings (virtio-serial-pci, virtio-mouse, virtio-keyboard, etc.), so a naive whole-output rfind(\"-device\") would false-positive-match inside the verbatim blob itself rather than the real qemu-emitted devices segment."

patterns-established:
  - "must_haves.truths (PLAN.md frontmatter) take priority over a task's illustrative action-text pseudocode when the two conflict and the literal pseudocode is unachievable/misleading against real data — same principle Plan 07-04 established, reapplied here for the RawArgs 'last -device' lookup."

requirements-completed: [QEMU-01, QEMU-02, QEMU-03]

coverage:
  - id: D1
    description: "assert_precedes helper: passes when order correct, panics naming both needles and their byte positions when violated or when either needle is absent"
    requirement: "QEMU-02"
    verification:
      - kind: unit
        ref: "tests/qemu_cmdline.rs#test_assert_precedes_passes_when_order_correct"
        status: pass
      - kind: unit
        ref: "tests/qemu_cmdline.rs#test_assert_precedes_panics_naming_both_needles_and_positions"
        status: pass
    human_judgment: false
  - id: D2
    description: "felucia-108 golden fixture: id=drive-scsi0 precedes drive=drive-scsi0, and id=drive-ide2 precedes drive=drive-ide2, proven via real ProxmoxImporter::into_runtime() output, not byte-for-byte comparison against 108.qemu.cmd"
    requirement: "QEMU-02"
    verification:
      - kind: integration
        ref: "tests/qemu_cmdline.rs#test_qemu_cmdline_felucia_108_drive_before_device_ordering"
        status: pass
    human_judgment: false
  - id: D3
    description: "felucia-108 golden fixture: RawArgs verbatim blob appears exactly once, as a contiguous substring, positioned after the devices segment's last real emitted token — re-verified end-to-end now that Plans 07-02/07-03/07-04 populate devices/netdevs/objects/tpm segments Plan 07-01 originally left empty"
    requirement: "QEMU-03"
    verification:
      - kind: integration
        ref: "tests/qemu_cmdline.rs#test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end"
        status: pass
    human_judgment: false
  - id: D4
    description: "Synthetic Runtime with a hand-constructed VirtioNetPcie at PcieAddress(20,0): id=net20 (netdev) precedes netdev=net20 (device) — closes RESEARCH.md Pitfall 7 (felucia's single NIC alone is insufficient to prove netdev-ordering robustness); also confirms empty boot_order emits zero bootindex= tokens at the test-file level"
    requirement: "QEMU-01"
    verification:
      - kind: integration
        ref: "tests/qemu_cmdline.rs#test_qemu_cmdline_synthetic_netdev_before_device_ordering"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-07-24
status: complete
---

# Phase 7 Plan 5: QEMU Cmdline Golden-Fixture Ordering Test Summary

**Wrote `tests/qemu_cmdline.rs` — the phase-closing integration test proving drive-before-device and netdev-before-device ordering against both the real felucia-108 fixture and a hand-built synthetic multi-NIC Runtime, plus a RawArgs-verbatim-at-end regression check.**

## Performance

- **Duration:** 45 min
- **Started:** 2026-07-24T14:22:00Z (approx.)
- **Completed:** 2026-07-24T15:07:43Z
- **Tasks:** 2/2 completed
- **Files modified:** 1 (created)

## Accomplishments

- `assert_precedes(haystack, needle_before, needle_after)` helper implemented and unit-tested (both the pass case and the panic-with-both-positions case)
- Real felucia-108 fixture, imported via `ProxmoxImporter::new(...).into_runtime()`, proven to emit `-drive id=drive-scsi0` before `-device ...,drive=drive-scsi0` and `-drive id=drive-ide2` before `-device ...,drive=drive-ide2` — string-position assertions only, no byte-for-byte comparison against `108.qemu.cmd` (D-05)
- Real felucia-108 fixture's `RawArgs` verbatim blob confirmed to appear exactly once, as one contiguous substring, positioned strictly after the devices segment's last real emitted device token — re-verifying `QEMU-03` now that every segment Plans 07-02/07-03/07-04 populate (devices/netdevs/objects/tpm) is non-empty, not just the mostly-empty segments Plan 07-01 tested it against
- Synthetic `Runtime` with a single hand-constructed `VirtioNetPcie` at `PcieAddress(20, 0)` proves `id=net20` precedes `netdev=net20` independently of the felucia fixture's single-NIC limitation — closes `RESEARCH.md` Pitfall 7 exactly as the plan's `must_haves.truths` and `prohibitions` required
- Same synthetic `Runtime`'s output confirmed to contain zero `bootindex=` tokens for an empty `boot_order`, re-confirming Plan 07-04's Task 3 behavior at the test-file level (not just `handlers/root.rs`'s own unit tests)

## Task Commits

**Not committed — per project's standing no-auto-commit policy; all changes left as unstaged/untracked working-tree edits for manual review and commit.**

Task-by-task status (all passed verification):

1. **Task 1: assert_precedes helper + felucia golden-fixture ordering test** — PASS (`cargo test --test qemu_cmdline` — all felucia-related tests green after the EfiDisk-panic deviation fix below)
2. **Task 2: synthetic netdev-before-device ordering test (closes RESEARCH.md Pitfall 7)** — PASS

## Files Created/Modified

- `tests/qemu_cmdline.rs` (created) — `assert_precedes` helper (+2 unit tests for it), `felucia_runtime_for_cmdline()` test helper, `test_qemu_cmdline_felucia_108_drive_before_device_ordering`, `test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end`, `test_qemu_cmdline_synthetic_netdev_before_device_ordering` — 5 tests total, all passing

## Decisions Made

- **Route around the pre-existing `EfiDisk.block_device_size_bytes` panic via a generalized helper, not a fix:** `ProxmoxImporter::into_runtime()`'s felucia output panics in `emit_root_device`'s `EfiDisk` arm (`.expect("EfiDisk.block_device_size_bytes required for pflash size= (D-06)")`) because the importer never resolves that field — a pre-existing, already-documented gap from Plan 07-01, explicitly called out in Plan 07-04's "Known Stubs" section and confirmed intentional by `src/config/proxmox/importer.rs`'s own passing unit test `test_04_02_efidisk_logical_size_from_options` (which asserts `block_device_size_bytes().is_none()`). Plan 07-04 routed around the identical panic in `root.rs`'s own test module via a narrow `felucia_chipset_and_boot_order()` helper (extracting only `Chipset` + `boot_order`). This plan's tests additionally need `RawArgs`/`AudioDevice`/etc. root devices, so `felucia_runtime_for_cmdline()` generalizes that pattern: it re-registers every imported root device except `EfiDisk` into a fresh `Runtime` via the public `Runtime::new()` + `register_root_device()` API, dropping `boot_order` (unused by this plan's ordering assertions) since there is no public setter for it outside `RuntimeBuilder`'s per-type `with_X` chain.
- **RawArgs "last -device" position resolved via `QemuCommandLine::devices()` getter, not a whole-output `rfind("-device")`:** the plan's literal Task 1 action text implies checking `output.rfind("-device")`, but felucia's real `RawArgs` blob (`108.conf`'s `args:` line) itself contains five embedded `-device ...` substrings (`-device virtio-serial-pci`, `-device virtserialport,...`, `-device virtio-mouse`, `-device virtio-keyboard`, `-device ivshmem-plain,...`). A generic `rfind("-device")` over the full output would match one of these embedded occurrences inside the verbatim blob itself, not the real qemu-emitted devices segment's last token — producing a false pass/fail unrelated to the actual ordering guarantee being tested. Resolved by taking `cmdline.devices().last()` (the actual last entry of the `devices` segment, from the builder's own typed accessor) and locating it via `output[..rawargs_pos].rfind(...)`, restricted to the region strictly before the RawArgs blob starts. This is the same "must_haves.truths over illustrative pseudocode" resolution principle Plan 07-04 documented for its `net_label` ordinal fix.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `ProxmoxImporter::into_runtime()`'s felucia Runtime panics in `emit_root_device`'s `EfiDisk` arm when run through the full `QemuCommandLine::try_from` pipeline**

- **Found during:** Task 1, first run of `cargo test --test qemu_cmdline` — both felucia-based tests panicked with `EfiDisk.block_device_size_bytes required for pflash size= (D-06)` before reaching any of this plan's own assertions.
- **Issue:** The plan's Task 1 action text specifies building the Runtime via `ProxmoxImporter::new(...).into_runtime()` directly and passing it straight to `QemuCommandLine::try_from`. `ProxmoxImporter` never populates `EfiDisk.block_device_size_bytes` (confirmed intentional/pre-existing by `importer.rs`'s own `test_04_02_efidisk_logical_size_from_options`, and explicitly flagged as a "Known Stub" carried over from Plan 07-01 in Plan 07-04's SUMMARY.md), so any full felucia-imported `Runtime` panics when `emit_root_device` reaches its `EfiDisk` root device. This is a pre-existing, out-of-scope gap unrelated to this plan's own changes — not something to fix here (fixing it would require deciding a real byte-size derivation formula for the importer, an architectural addition out of this plan's scope, and would break the existing `test_04_02_efidisk_logical_size_from_options`'s explicit `is_none()` assertion).
- **Fix:** Added `felucia_runtime_for_cmdline()`, which imports felucia exactly as the plan specifies, then re-registers every root device except `EfiDisk` into a fresh `Runtime` (via `Runtime::new()` + `register_root_device()`), and uses that filtered `Runtime` for both felucia-based tests' `QemuCommandLine::try_from` call. This generalizes the routing pattern Plan 07-04 already established (`felucia_chipset_and_boot_order()`) for the identical panic, rather than reinventing a different workaround.
- **Files modified:** `tests/qemu_cmdline.rs` only.
- **Verification:** `test_qemu_cmdline_felucia_108_drive_before_device_ordering` and `test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end` both pass; full workspace suite (`cargo test`) remains green (104/104).
- **Committed in:** Not committed — per project's standing no-auto-commit policy.

**2. [Rule 1 - Bug in test-design, caught before landing] RawArgs "last -device" lookup via whole-output `rfind` would false-positive-match inside the verbatim blob itself**

- **Found during:** Task 1, while designing `test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end`'s assertion (before running it — caught by inspecting felucia's actual `args:` string content, which itself contains 5 `-device ...` substrings).
- **Issue:** The plan's Task 1 behavior/action text says to assert `output.rfind(&rawargs_substring).unwrap() > output.rfind("-device").unwrap()`. Since the RawArgs blob is rendered in the `misc` segment (last), and the blob's own text contains multiple `-device ...` substrings, a whole-output `rfind("-device")` finds the last such substring *inside the blob itself* — not the real qemu-emitted devices segment's last token. Depending on which specific embedded `-device` phrase is last within the blob relative to where the chosen `rawargs_substring` starts, this could make the assertion pass or fail for a reason unrelated to the actual ordering guarantee (RawArgs positioned after every other segment's last token, per the plan's own `must_haves.truths`).
- **Fix:** Resolved the "last real device token" via `QemuCommandLine`'s own `devices()` segment getter (`cmdline.devices().last()`) instead of a generic substring search, and restricted the position search to `output[..rawargs_pos]` (the region strictly before the RawArgs blob starts) to guarantee no coincidental match inside the blob could produce a false pass.
- **Files modified:** `tests/qemu_cmdline.rs` only.
- **Verification:** `test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end` passes and correctly reflects the `must_haves.truths` requirement ("RawArgs's verbatim blob ... positioned after every other segment's last token").
- **Committed in:** Not committed — per project's standing no-auto-commit policy.

### Additional scope note

The plan's frontmatter lists `files_modified: [tests/qemu_cmdline.rs]` — this plan touched only that one file, exactly as scoped. No `src/` changes were made or needed.

## Issues Encountered

None beyond the two deviations documented above, both resolved without needing a checkpoint.

## Known Stubs

None introduced by this plan. Reiterating the pre-existing, out-of-scope stub from Plan 07-01 (documented again here for visibility since this plan's tests route around it): `ProxmoxImporter`-derived `EfiDisk.block_device_size_bytes` is always `None`, which panics `emit_root_device`'s `EfiDisk` arm if a full felucia-imported `Runtime` (including its `EfiDisk` root device) is ever passed to `QemuCommandLine::try_from` unfiltered. Not blocking this plan's own truths — routed around via `felucia_runtime_for_cmdline()`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 7's four success criteria (`QEMU-01`/`QEMU-02`/`QEMU-03` coverage + ordering proofs) are now fully closed: drive-before-device and netdev-before-device ordering are proven via string-position assertions (not substring presence) against both a real-world fixture and a synthetic multi-device case; `RawArgs` verbatim-at-end placement is re-confirmed end-to-end after all of Plans 07-02/07-03/07-04's additions; the netdev-ordering guarantee is proven independently of felucia's single-NIC limitation (RESEARCH.md Pitfall 7 closed).
- This is the last plan in Phase 7 — ready for phase-level verification/audit.

---
*Phase: 07-qemu-cmdline*
*Completed: 2026-07-24*

## Self-Check: PASSED

- `tests/qemu_cmdline.rs` — FOUND
- `cargo test --test qemu_cmdline` — 5/5 PASS
- `cargo test` (full workspace suite: 84 lib + 7 proxmox_import + 5 qemu_cmdline + 5 runtime_phase2 + 3 yaml_round_trip) — 104/104 PASS, 0 failed
- `cargo build` — succeeds with zero errors (pre-existing warnings only, none introduced by this plan)
- No commits exist for this plan (by design — no-auto-commit policy); nothing to verify via `git log`. `git status --short` confirms `tests/qemu_cmdline.rs` is the only new/modified file from this plan's execution.
