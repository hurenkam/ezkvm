# ezkvm Machine Model Policy for Q35 and i440fx

Status: Draft
Date: 2026-05-28
Scope: ezkvm implementation policy for canonical machine modeling and deterministic QEMU rendering
Purpose: Define project policy for chipset-sensitive topology/placement decisions while preserving FR-004 model separation and FR-005 determinism.

## 1. Policy Intent and Requirement Alignment

This policy constrains how ezkvm represents and renders Q35/i440fx VM topology.

- FR-004 alignment: keep import-host concerns, canonical machine model, and runtime-host concerns strictly separated.
- FR-005 alignment: ensure identical canonical model + runtime-host input always produces identical QEMU arguments.

This document is policy. Background architecture details remain in chipset-domain and QEMU-behavior references.

## 2. Deterministic Topology and Placement Policy

For Q35 and i440fx, canonical model generation and rendering MUST follow these rules:

1. Machine type and machine version are explicit canonical fields.
2. Device placement is explicit in the canonical model for all chipset-sensitive devices (`bus`, slot/function address, and bridge path).
3. QEMU auto-placement is not relied on for chipset-sensitive endpoints in final rendering.
4. Device rendering order is stable and deterministic:
   - chipset/root elements first
   - bridge/root-port hierarchy second
   - endpoints last
   - ties resolved by stable canonical sort key (device class, then canonical id)
5. Canonical topology identity is preserved across re-renders from equivalent inputs.

Policy rationale: deterministic explicit placement prevents realization-order drift from changing guest-visible slot/function outcomes.

## 3. Boundary: Canonical Model vs Runtime Host Resolution

### 3.1 Canonical Model (Guest-Visible Contract)

Canonical model owns all guest-visible machine topology decisions, including:

- machine family (`q35` or `pc-i440fx-*`) and pinned version
- bridge/root-port structure
- endpoint attachment path
- stable slot/function identity where applicable

These fields are treated as fixed layout contract after import/model-build.

### 3.2 Runtime Host Resolution (Host-Local Adaptation)

Runtime resolution may resolve host-local values only, for example:

- binary/path discovery
- host capability checks
- acceleration/runtime host toggles that do not alter guest-visible topology

Runtime resolution MUST NOT rewrite canonical chipset topology or silently re-place guest-visible devices.

## 4. Conflict Resolution and Precedence Rules

When source inputs are incomplete or conflicting, ezkvm applies the following precedence:

1. Explicit, internally consistent source placement and machine-version fields.
2. Import-adapter deterministic inference from source-native structure.
3. ezkvm machine-family policy defaults (Q35/i440fx templates).
4. Runtime host resolution for host-local values only.

Conflict handling rules:

- If explicit source fields conflict with chipset invariants, fail validation; do not silently remap.
- If source omits placement details, fill using deterministic machine-family defaults.
- If multiple source artifacts disagree, choose the most explicit canonicalizable representation and emit a warning with provenance.
- If runtime host limitations make canonical topology non-runnable, fail preflight with actionable diagnostics; do not mutate canonical model.

## 5. Must-Have Validation for Chipset-Sensitive Configurations

Before execution, validation MUST include at least:

1. Machine-family validity and version pinning checks (`q35`/`pc-q35-*` vs `pc-i440fx-*`).
2. Unique bus/address occupancy checks (no duplicate slot/function on same bus).
3. Topology-path validity checks (every endpoint references an existing realized bus path).
4. Q35-specific checks:
   - PCIe endpoints are attached through valid PCIe hierarchy.
   - Conventional PCI endpoints under Q35 use explicit bridge path (for example PCIe-to-PCI bridge) when required.
5. i440fx-specific checks:
   - placement remains valid for legacy PCI-centric topology assumptions.
   - incompatible PCIe-only attachment assumptions are rejected.
6. Bridge hierarchy sanity checks (depth/fanout and bus-number budget risk signaling).
7. Determinism check in dry-run path: equivalent canonical+runtime input produces byte-stable argument order.
8. Separation check: runtime-resolved fields do not overwrite canonical guest-visible topology fields.

## 6. Implementation Guidance

- Import adapters should preserve explicit source wiring whenever possible.
- Missing values should be populated by machine-family deterministic defaults, never by non-deterministic probe order.
- Error messages should identify whether failure originates in import normalization, canonical model validation, or runtime capability resolution.

## 7. Related References

- `doc/dev/requirements/product-requirements.md` (FR-004, FR-005)
- `doc/dev/domain-knowledge/qemu/qemu-chipset-behavior.md`
- `doc/dev/domain-knowledge/q35/q35-chipset-domain.md`
- `doc/dev/domain-knowledge/i440fx/i440fx-chipset-domain.md`
