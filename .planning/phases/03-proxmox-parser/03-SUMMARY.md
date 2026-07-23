---
phase: 03-proxmox-parser
plan: 01
subsystem: config-parser
tags: [proxmox, parser, storage, rust]
requires:
  - phase: 01-foundation
    provides: Typed conversion errors and stable device dispatch
provides:
  - Typed Proxmox VM and storage configuration parsers
  - Snapshot-isolated active configuration parsing
  - Colon-safe sub-option tokenization
affects: [proxmox-runtime, yaml-schema, qemu-cmdline]
tech-stack:
  added: []
  patterns:
    - Separate state machines for VM configuration and storage configuration grammars
    - Split sub-options on commas and first equals sign only
key-files:
  created:
    - src/config/proxmox/error.rs
    - src/config/proxmox/parser.rs
    - src/config/proxmox/conf.rs
    - src/config/proxmox/storage.rs
  modified:
    - src/config/proxmox.rs
    - src/config/proxmox/conf.rs
key-decisions:
  - "Snapshot sections are isolated before active VM fields are dispatched."
  - "Colons remain opaque inside MAC addresses, BDFs, and storage-volume references."
  - "Recognized malformed fields return ProxmoxParseError instead of being silently discarded."
patterns-established:
  - "Use BTreeMap for deterministic indexed Proxmox device fields."
requirements-completed: [PROX-01, PROX-02, PROX-03, PROX-04]
coverage:
  - id: D1
    description: Snapshot-isolated Proxmox VM parser with comment filtering
    requirement: PROX-01
    verification:
      - kind: unit
        ref: "src/config/proxmox/parser.rs#test_split_sections_active_has_cpu_host"
        status: pass
      - kind: unit
        ref: "src/config/proxmox/parser.rs#test_split_sections_skips_hash_comments"
        status: pass
    human_judgment: false
  - id: D2
    description: Colon-safe device sub-option parser
    requirement: PROX-03
    verification:
      - kind: unit
        ref: "cargo test -p ezkvm test_parse_"
        status: pass
    human_judgment: false
  - id: D3
    description: Typed Proxmox VM configuration parsing
    requirement: PROX-02
    verification:
      - kind: unit
        ref: "src/config/proxmox/conf.rs#test_proxmox_vm_conf_from_108_conf"
        status: pass
    human_judgment: false
  - id: D4
    description: Typed storage.cfg parsing
    requirement: PROX-04
    verification:
      - kind: unit
        ref: "cargo test -p ezkvm test_parse_storage"
        status: pass
    human_judgment: false
duration: 18min
completed: 2026-07-23
status: complete
---

# Phase 03: Proxmox Parser Summary

**Typed Proxmox VM and storage configuration parsers with snapshot isolation and colon-safe device parsing**

## Performance

- **Duration:** 18 min
- **Started:** 2026-07-23T20:47:54Z
- **Completed:** 2026-07-23T21:00:00Z
- **Tasks:** 4
- **Files modified:** 6

## Accomplishments

- Verified active VM fields are isolated from snapshot sections and raw hash-prefixed comments.
- Parsed typed device, scalar, and storage configuration fields while preserving colon-bearing values.
- Preserved errors for malformed recognized fields and added missing VM parser coverage.

## Task Commits

1. **Phase parser implementation** - `25e66e4` (existing implementation)
2. **Parser error propagation and coverage** - `2760cce` (fix)
3. **Parser dispatch cleanup** - `1406700` (fix)

## Files Created/Modified

- `src/config/proxmox/parser.rs` - Snapshot state machine and colon-safe device tokenizers.
- `src/config/proxmox/conf.rs` - Typed VM config dispatch and malformed-field error propagation.
- `src/config/proxmox/storage.rs` - Typed storage configuration parser.
- `src/config/proxmox/error.rs` - Parser error definitions.

## Decisions Made

- Keep unknown top-level keys forward-compatible, but reject malformed values for recognized keys.
- Treat the fixture corpus as a required test input for parser verification.

## Deviations from Plan

### Auto-fixed Issues

**1. [Correctness] Propagated malformed recognized-field errors**
- **Issue:** Existing VM parsing discarded malformed scalar and device fields.
- **Fix:** Return `ProxmoxParseError::InvalidSubOption` for malformed recognized values and indices.
- **Verification:** `cargo test -p ezkvm`
- **Committed in:** `2760cce`

## Issues Encountered

- The expected `input/felucia` fixture corpus was absent until supplied during execution.

## Next Phase Readiness

- Phase 4 can consume typed `ProxmoxVmConf` and `ProxmoxStorageConf`.
- `StorageResolver` directory-content path handling is documented in `03-REVIEW.md` for Phase 4 follow-up.

---
*Phase: 03-proxmox-parser*
*Completed: 2026-07-23*
