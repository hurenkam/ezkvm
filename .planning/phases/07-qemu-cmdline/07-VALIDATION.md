---
phase: 07
slug: qemu-cmdline
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-07-24
---

# Phase 07 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` via `cargo test` (edition 2024) |
| **Config file** | none — no external test config; tests co-located with source per project convention |
| **Quick run command** | `cargo test qemu` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test qemu`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01 | 07 | 1 | QEMU-01, QEMU-03 | — / N/A | Segmented `QemuCommandLine` struct + `Display` emit segments in fixed order | unit | `cargo test qemu` | ❌ W0 | ⬜ pending |
| 07-02 | 07 | 2 | QEMU-01, QEMU-02 | T-07-03, T-07-04 | pcie_bus dispatch (PvScsi/HostPci/Ivshmem/VirtioNet) + shared scsi/storage emission | unit + fixture | `cargo test qemu` | ❌ W0 | ⬜ pending |
| 07-03 | 07 | 3 | QEMU-01, QEMU-02 | T-07-05, T-07-06 | pci_bus/sata_bus/ide_bus/usb_bus dispatch + ProxmoxImporter net/usb wiring | unit | `cargo test qemu` | ❌ W0 | ⬜ pending |
| 07-04 | 07 | 4 | QEMU-01 | T-07-07 | Boot-order device-id reconstruction + bootindex lookup wired into scsi/sata/ide/net call sites | unit | `cargo test qemu` | ❌ W0 | ⬜ pending |
| 07-05 | 07 | 5 | QEMU-01, QEMU-02, QEMU-03 | T-07-08 | drive/netdev references always precede their `-device`; RawArgs verbatim at end; felucia bootindex end-to-end proof | integration, string-position assertion | `cargo test --test qemu_cmdline` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] New test module (`src/config/qemu.rs`'s own `#[cfg(test)]` block, or a new `tests/qemu_cmdline.rs` integration test if the felucia-fixture test needs the full `ProxmoxImporter` pipeline) — covers QEMU-01/02/03; created by Plan 07-05 Task 1
- [ ] Ordering-assertion helper (`fn assert_precedes(haystack: &str, needle_before: &str, needle_after: &str)`) — reusable across all drive/netdev ordering assertions, not present anywhere in the codebase today; created by Plan 07-05 Task 1

---

## Manual-Only Verifications

*None — all phase behaviors have automated verification (structural token/ordering assertions per D-05; no UI, no external I/O).*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — confirmed across all 5 plans (07-01–07-05), every task's `<verify>` includes an `<automated>` command
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references — `assert_precedes` + `tests/qemu_cmdline.rs` (Plan 07-05 Task 1)
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved (planning-time Nyquist compliance confirmed; per-task Status/File Exists in the verification map above remain ⬜ pending until Plan execution)
