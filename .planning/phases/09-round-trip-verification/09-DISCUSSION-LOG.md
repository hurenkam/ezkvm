# Phase 9: Round-Trip Verification - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-28
**Phase:** 9-Round-Trip-Verification
**Areas discussed:** Real-QEMU execution scope, Corpus selection, Verification method

---

## Real-QEMU execution scope

| Option | Description | Selected |
|--------|-------------|----------|
| Stay string/structure-level only | Defer all real-QEMU execution to Phase 10; keeps Phase 9 host-agnostic like Phase 8 | ✓ |
| Spawn real qemu-system-x86_64 | Verify monitor-prompt reachability here too, in addition to Phase 10 | |
| Something else | Freeform | |

**User's choice:** Stay string/structure-level only in Phase 9; defer all real-QEMU execution to Phase 10.
**Notes:** ROADMAP success criterion #3 ("reaches the QEMU monitor prompt") is reinterpreted as structural well-formedness, not actual process execution.

---

## Corpus selection for additional configs

| Option | Description | Selected |
|--------|-------------|----------|
| coruscant + zbp-server-mh2 | Matches ROADMAP's own example (SCSI variant + hostpci/lspci-documented config) | ✓ |
| Different/more corpus configs | User picks alternates | |
| Something else | Freeform | |

**User's choice:** coruscant + zbp-server-mh2.

Follow-up — which specific files:

| Option | Description | Selected |
|--------|-------------|----------|
| coruscant/501 + zbp-server-mh2/301 | Both have lspci (and 301 also lsusb) output for cross-check | ✓ |
| coruscant/501 + zbp-server-mh2/103 | 103 has companion swtpm.cmd, exercises TPM path | |
| Pick specific files myself | Freeform | |

**User's choice:** coruscant/501 + zbp-server-mh2/301.
**Notes:** Chosen for real hardware cross-check data (lspci/lsusb) to validate hostpci/USB device mapping.

---

## Verification method

| Option | Description | Selected |
|--------|-------------|----------|
| No-errors + structural spot-checks only | Assert device counts/types/key flags present; no line-by-line diff | ✓ |
| Hand-write fresh ezkvm references | Write new .ezkvm.qemu.cmd files for coruscant/501 and zbp-server-mh2/301, exact-diff like felucia | |
| Something else | Freeform | |

**User's choice:** "Ignore the 108.ezkvm.qemu.cmd file, it was generated for an older version of ezkvm and may not be correct for the current version. For the rest, follow item 1."
**Notes:** This supersedes ROADMAP's original success criterion #2 (exact-diff against 108.ezkvm.qemu.cmd) — all three corpus configs (including felucia/108) now use structural spot-checks uniformly. The stale reference file is treated as historical/informational only, not regenerated or fixed as part of this phase.

Follow-up confirmation:

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, note it and structural-only for all three | Update ROADMAP criterion accordingly | ✓ |
| No, still want reference regenerated | As a documentation byproduct, not for assertions | |

**User's choice:** Yes, note it and structural-only for all three.

---

## the agent's Discretion

- Exact assertion granularity within "structural spot-checks" (device-kind presence vs. exact counts) — follow the rigor level already established in Phase 7's `qemu.rs` unit tests.
- Whether corpus `lspci`/`lsusb` files are parsed programmatically for assertions or used as human-reference documentation while writing tests.

## Deferred Ideas

- Real-QEMU boot verification (spawning `qemu-system-x86_64`, confirming monitor-prompt reachability) — belongs to Phase 10.
- Regenerating a fresh, current `108.ezkvm.qemu.cmd` reference file for human documentation purposes — not requested, noted only as a future possibility.
