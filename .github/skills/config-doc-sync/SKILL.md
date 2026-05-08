---
name: config-doc-sync
description: "Use when configuration schema/defaults/semantics or importer mapping changes in ezkvm and user-facing docs must be updated or explicitly deferred in backlog docs."
argument-hint: "What config or importer change should be reflected in docs?"
user-invocable: true
---

# Config Doc Sync

## What This Skill Produces
- A documentation impact assessment for configuration-related changes
- Concrete updates to user-facing docs that match implementation behavior
- Explicit backlog deferral entries when documentation cannot be completed immediately

## When to Use
- `src/config/**` changes
- `src/import/**` changes that affect user-visible mapping/behavior
- `src/cli/**` changes that affect user-visible command surfaces
- Changes to YAML under `etc/*.yaml`, `etc/profiles.d/*.yaml`, or `etc/vms.d/*.yaml`
- Any change that alters user-facing fields, defaults, semantics, or generated QEMU behavior

## Workflow
1. Identify changed config surfaces and map affected docs:
   - `doc/user/config/` for detailed user config behavior
   - `README.md` when onboarding or CLI-facing behavior changed
2. Update field descriptions, defaults, constraints, and examples in relevant docs.
3. Update command usage docs when CLI flags or semantics changed.
4. If docs must be deferred, create/update backlog tracking in `doc/backlog/BACKLOG.md` and keep `doc/backlog/TRACKING_BOARD.md` synchronized.
5. Report documentation impact as one of: `updated`, `not needed`, or `deferred`.

## Drift Checklist
- Does each new field/flag/behavior have user-facing documentation?
- Are renamed/removed fields reflected in docs?
- Do documented defaults match code defaults?
- Do examples still parse and match current schema?
- Are importer-driven user-visible behaviors documented?

## Output Requirements
- Include file references for updated docs.
- Call out residual doc risks if any sections remain partially documented.
