# Code Review Update: import/proxmox Module Against Guidelines

**Original review date:** 2026-04-16  
**Updated:** 2026-04-17  
**Scope:** Current architecture, coding, and module ownership compliance review of `src/import/proxmox/`  
**References:**
- `doc/dev/ARCHITECTURE_GUIDELINES.md`
- `doc/dev/CODING_GUIDELINES.md`
- `doc/dev/MODULE_OWNERSHIP.md`

---

## Executive Summary

The original cleanup recommendations are still directionally correct, but the baseline has changed.

- The importer gained profile-aware compaction (`profile_compact.rs`), improving separation of concerns.
- The main mapper is now larger, not smaller (`mapper.rs` 2844 lines).
- Public API exposure remains broader than intended.
- Oversized-file debt increased and should be treated as active refactor debt.

| Category | Current Status | Severity |
|----------|----------------|----------|
| Dependency direction | Compliant | - |
| Error handling | Compliant | - |
| Module wiring | Compliant | - |
| Test placement | Compliant | - |
| File size | Violated | Critical |
| Function cohesion | Violated | High |
| Public surface area | Over-exposed | Medium |
| Struct sizes | Compliant | - |

---

## Current Evidence Snapshot

Measured from current workspace:

| File | Lines | Guideline Target | Variance | Status |
|------|-------|------------------|----------|--------|
| `mapper.rs` | **2844** | < 250 | **+2594** (+1038%) | **Critical** |
| `profile_compact.rs` | **732** | < 250 | **+482** (+193%) | **High** |
| `parser.rs` | 372 | < 250 | +122 (+49%) | High |
| `io.rs` | 305 | < 250 | +55 (+22%) | Medium |
| `storage_parser.rs` | 191 | < 250 | - | OK |
| `yaml_compact.rs` | 162 | < 250 | - | OK |
| `model.rs` | 56 | < 250 | - | OK |
| `error.rs` | 17 | < 250 | - | OK |
| `mod.rs` | 14 | < 250 | - | OK |

Additional complexity indicators:

- `mapper.rs` contains many function definitions (including tests/helpers), indicating concentration of behavior.
- `profile_compact.rs` is a meaningful extraction win, but is itself now well beyond the file-size guideline.

---

## What Changed Since the Original Review

### Improved

1. Profile-aware compaction was extracted into `profile_compact.rs` and integrated into import flow.
2. `mod.rs` still remains wiring-focused.
3. Dependency direction and error handling remain healthy.

### Regressed / Still Open

1. `mapper.rs` grew from 2082 to 2844 lines.
2. Total count of oversized files increased (`profile_compact.rs` added as a large module).
3. Public exports still leak low-level parser/mapper functions to external callers.

---

## Public Surface Review

Current `mod.rs` exports:

- Public modules: `error`, `io`, `mapper`, `model`, `parser`, `storage_parser`
- Public re-exports include:
    - `run_import_from_files` and `ImportRunOptions` (desired high-level API)
    - `map_proxmox_to_canonical_yaml*`, `parse_proxmox_config`, `parse_proxmox_storage_config` (low-level API leakage)

Observed usage in repository:

- CLI and integration tests use the high-level API (`run_import_from_files`) as intended.
- Low-level exports are mostly used by importer-internal tests.

Assessment:

- Restricting low-level public re-exports is still recommended and now lower risk than before.

---

## Recommendation Revalidation

Status of previous recommendations:

| Recommendation (2026-04-16) | Still Valid? | Updated Priority | Notes |
|-----------------------------|--------------|------------------|-------|
| Refactor `mapper.rs` into focused modules | Yes | **P0** | More urgent because file grew substantially |
| Restrict public exports to high-level API | Yes | **P1** | Repository usage suggests manageable migration |
| Add explicit justification for oversized files | Yes | **P2** | Must now include `profile_compact.rs` too |

New/additional recommendation:

| Recommendation (new) | Priority | Notes |
|----------------------|----------|-------|
| Split `profile_compact.rs` by profile domain (system/devices/network/storage) | **P1** | Prevents new monolith while preserving profile compaction behavior |

---

## Updated Mitigation Plan

### P0: Decompose `mapper.rs`

Target shape:

```
src/import/proxmox/
    mapper.rs              # entry/orchestration only
    mapper/
        system.rs
        storage.rs
        network.rs
        display.rs
        devices.rs
        helpers.rs
```

Acceptance:

- `mapper.rs` limited to orchestration and public mapping entry points.
- New mapper submodules each have one primary responsibility.
- No behavioral change in generated YAML/output fixtures.

### P1: Reduce API leakage and split profile compaction domains

1. Keep only high-level import API public from `proxmox` module surface:
     - `ImportRunOptions`
     - `run_import_from_files`
     - `ImportError`
2. Move low-level parser/mapper exports to internal module access.
3. Split `profile_compact.rs` by domain (or by merge target path families) to keep each file below guideline thresholds when practical.

Acceptance:

- No external module in repository depends on low-level parser/mapper public exports.
- Profile compaction logic remains profile-aware and test-covered.

### P2: Document justified overages while refactor is in progress

- Add module-level comments to oversized files that are intentionally temporary.
- Link rationale to backlog tasks so size debt is tracked and not normalized.

---

## Risks and Mitigations

### Risk: Behavioral drift during decomposition

Mitigation:

- Preserve existing fixture/snapshot integration tests.
- Validate canonical YAML and profile lists remain unchanged for known fixtures.

### Risk: Breaking callers by tightening exports

Mitigation:

- Update importer-internal tests to use internal module paths.
- Keep `run_import_from_files` contract stable.

### Risk: Refactor churn without measurable improvement

Mitigation:

- Track file-size and public-surface deltas per phase.
- Require each phase to retire at least one oversized hotspot.

---

## Conclusion

The original cleanup direction remains correct and should continue, with adjusted urgency.

- `mapper.rs` refactor is now the top priority.
- Export tightening remains recommended and appears practical.
- `profile_compact.rs` should be proactively split to avoid replacing one monolith with another.

This document supersedes the 2026-04-16 metrics and recommendation priorities.
