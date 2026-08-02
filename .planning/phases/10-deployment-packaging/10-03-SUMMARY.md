---
phase: 10-deployment-packaging
plan: 03
subsystem: infra
tags: [debian, ubuntu, packaging, docs, qemu, musl]
requires:
  - phase: 10-deployment-packaging
    provides: packaging metadata, Debian maintainer scripts, CI package verification workflow
provides:
  - packaging decision reference for the Debian/Ubuntu `.deb`
  - manual-only real-hardware verification checklist for felucia/108 (`wakiza`)
affects: [deployment-packaging, docs, manual-verification]
tech-stack:
  added: []
  patterns: [documentation-only packaging guidance, explicit manual-vs-automated verification split]
key-files:
  created:
    - doc/dev/PACKAGING.md
    - doc/dev/MANUAL-VERIFICATION.md
    - .planning/phases/10-deployment-packaging/10-03-SUMMARY.md
  modified: []
key-decisions:
  - "Cross-referenced debian/host.yaml.example for shipped operator notes instead of duplicating its wording."
  - "Documented felucia/108 boot as manual-only because its GPU and USB passthrough cannot exist in CI."
patterns-established:
  - "Packaging docs should separate archive/package-name facts from operator setup notes."
  - "Real-hardware passthrough checks must be documented as manual when CI cannot supply the devices."
requirements-completed: [DEPLOY-02, DEPLOY-03]
coverage:
  - id: D1
    description: "Developer packaging reference documenting musl-static single-package rationale, FHS layout, ezkvm group model, dependency split, unconfined stance, revision suffixes, and distro quirks placeholder."
    requirement: "DEPLOY-03"
    verification:
      - kind: other
        ref: "test -f doc/dev/PACKAGING.md && grep -c \"unconfined\" doc/dev/PACKAGING.md | grep -qv '^0$' && grep -c \"qemu-system-x86\" doc/dev/PACKAGING.md | grep -qv '^0$' && grep -c \"Recommends\" doc/dev/PACKAGING.md | grep -qv '^0$'"
        status: pass
    human_judgment: false
  - id: D2
    description: "Manual-only felucia/108 real-hardware verification checklist documenting GPU/USB passthrough expectations and non-CI status."
    requirement: "DEPLOY-02"
    verification:
      - kind: other
        ref: "test -f doc/dev/MANUAL-VERIFICATION.md && grep -ci \"manual\" doc/dev/MANUAL-VERIFICATION.md | grep -qv '^0$' && grep -c \"wakiza\\|felucia\" doc/dev/MANUAL-VERIFICATION.md | grep -qv '^0$' && ! grep -rl \"MANUAL-VERIFICATION\" .github/workflows/ 2>/dev/null"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-07-30
status: complete
---

# Phase 10: Deployment Packaging Summary

**Packaging reference docs now explain the single musl-static `.deb` design and separate felucia/108's real-hardware boot check from automated CI.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-07-30T13:15:39+02:00
- **Completed:** 2026-07-30T13:40:00+02:00
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added `doc/dev/PACKAGING.md` covering the Debian/Ubuntu package model, dependency rationale, access-control model, and v1 security stance.
- Added `doc/dev/MANUAL-VERIFICATION.md` labeling felucia/108 boot validation as manual-only real-hardware work.
- Verified both docs with the plan's required grep/test checks, including confirming no workflow references the manual-verification doc.

## Task Commits

None — per execution instructions, this work was left uncommitted and no `git add`/`git commit` was used.

## Files Created/Modified
- `doc/dev/PACKAGING.md` - packaging architecture and distro-difference reference
- `doc/dev/MANUAL-VERIFICATION.md` - manual-only felucia/108 real-hardware checklist
- `.planning/phases/10-deployment-packaging/10-03-SUMMARY.md` - plan completion summary

## Decisions Made
- Cross-referenced `debian/host.yaml.example` for the shipped comment text instead of copying it verbatim.
- Kept the distro-quirks section honest with a single placeholder row because no real per-target quirks were observed in scope.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- The plan file itself exceeded a single `view` call and had to be read in sections before execution.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Packaging docs are in place for reviewers and operators.
- Manual real-hardware validation remains available as a non-CI follow-up on an appropriately configured host.

---
*Phase: 10-deployment-packaging*
*Completed: 2026-07-30*
