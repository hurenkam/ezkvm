# ADR-0001: Base Selection for Incremental Convergence

**Date:** 2026-04-15  
**Status:** Accepted  
**Context:** ezkvm Incremental Convergence Initiative

## Question

Which codebase (current, v1, or from-scratch) should serve as the foundation for merging v1 strengths (Proxmox import, lifecycle hooks, trait extensibility) with current strengths (profile system, layered architecture, clap CLI)?

## Decision

**Start from the current codebase** (`/home/hurenkam/Workspace/ezkvm`).

## Rationale

### Current Codebase Advantages
1. **Already has target foundations** - Profile system, clean layered architecture, and clap-based CLI are in place and stable
2. **Lowest migration risk** - Selective import of v1 features into current avoids re-implementing major current capabilities
3. **Shortest path to value** - Can add v1 strengths without destabilizing active workflows
4. **Better architecture** - Current layered model (CLI → Config → QEMU → OS) is maintainable and testable
5. **Type safety** - Strong typing and fail-fast validation are proven in current version
6. **Minimal dependencies** - 7 core crates vs v1's 20+

### v1 Alternative Approach (Rejected)
- Would require modernizing v1 to reach current parity
- Duplicates work on profile system, validation, and architecture
- Higher risk of introducing regressions
- Slower delivery timeline

### From-Scratch Alternative (Rejected)
- Highest risk - likely feature regression
- Longest lead time
- Loses proven profile merging logic
- Duplicates all config schema work

## Architectural Implication

**Compatibility Boundary:** v1 features (particularly Proxmox import) must normalize to current's canonical schema. No reverse mapping allowed. This ensures current version remains the single source of truth for configuration structure.

## Consequences

**Positive:**
- Existing current-version users see no disruption
- Profile system and layering remain stable
- Can incrementally add v1 features without big-bang rewrites
- New features gated behind tests before enablement

**Negative:**
- v1 users cannot directly use v1 to upgrade (will need import tool)
- Some v1-specific patterns (lifecycle hooks, trait objects) require adaptation to current's layered model
- Initial work focused on feature ports rather than optimization

## Related ADRs
- [ADR-0002: Import Normalization Contract](ADR-0002-import-normalization-contract.md)
- [ADR-0003: Hooks Policy](ADR-0003-hooks-policy.md)
- [ADR-0004: Trait Seam Policy](ADR-0004-trait-seam-policy.md)

## References

- Comparison document: `EZKVM_COMPARISON.md`
- Convergence plan: `INCREMENTAL_CONVERGENCE.md`
- Backlog: `doc/planning/backlog/done/2026/A.md`
