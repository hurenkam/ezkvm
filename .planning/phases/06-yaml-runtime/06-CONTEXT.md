# Phase 6: YAML↔Runtime - Context

**Gathered:** 2026-07-23
**Status:** Ready for planning

## Phase Boundary

Wire the Runtime ↔ ezkvm YAML ↔ Runtime pipeline end-to-end with field-level fidelity for all v1 device types.

## Implementation Decisions

### Validation and error handling
- **D-01:** Missing resources and unsupported device configurations fail the entire load with typed, actionable errors.
- **D-02:** Report the first invalid reference deterministically.
- **D-03:** Use documented schema defaults for absent optional fields; error only for required fields.

### Round-trip fidelity
- **D-04:** Preserve every modeled field, device topology, and opaque raw arguments.
- **D-05:** Preserve semantic data only; YAML formatting, ordering, and omission of default-valued fields may normalize.

## Canonical References

- `.planning/ROADMAP.md` — Phase 6 goal and dependency boundary.
- `.planning/REQUIREMENTS.md` — YAML-01 and YAML-02 traceability.
- `.planning/phases/05-yaml-schema/05-SUMMARY.md` — Existing YAML schema surface.
- `.planning/phases/04-proxmox-runtime/04-SUMMARY.md` — Runtime input produced by the Proxmox importer.

## Existing Code Insights

- Runtime/schema conversion follows `TryFrom` patterns between layer types.
- The internal `crate::serde_yaml` implementation remains the serialization layer.
- `RawArgs` are opaque and must never be parsed or normalized.

## Specific Ideas

No additional requirements beyond the locked validation and fidelity decisions.

## Deferred Ideas

None — discussion stayed within phase scope.

---

*Phase: 06-yaml-runtime*
*Context gathered: 2026-07-23*
