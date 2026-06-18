# Ezkvm/Proxmox Config-Format Alignment Recommendations

Date: 2026-06-18
Status: Draft design note for follow-up implementation work

## Scope

This document captures recommendations after a close comparison of the ezkvm and Proxmox import/export implementations in `src/config_format/**`.

## Current Strengths

### Ezkvm strengths

- Strong validation and diagnostic model with issue severity, remediation hints, and path-oriented reports.
- Clear importer flow: read input, parse schema, validate, convert to runtime model.
- End-to-end exporter implementation exists and writes YAML output.

### Proxmox strengths

- Solid typed parser for `.conf` text with deterministic rendering support.
- Practical runtime mapping for core fields (machine, CPU, memory) and baseline device/resource extraction.
- Parse/render roundtrip tests exist for schema-level behavior.

## Current Weaknesses

### Proxmox weaknesses

- Exporter is currently not implemented.
- Storage config argument is accepted but not actively used in import mapping.
- Import module still contains duplicate/unused parsing path, which increases maintenance risk.
- Parser module layout is inconsistent (placeholder module exists alongside implemented schema parser).
- Runtime mapping is implemented as an extension on `RuntimeModelBuilder` inside format adapter code, which blurs stage boundaries.

### Ezkvm weaknesses

- Export reconstruction partly infers data by inspecting generated QEMU args, which is fragile.
- TPM reconstruction uses display-string parsing, which is brittle.
- Some host/name plumbing appears present only for future use and currently adds noise.

## Recommendations To Address Weaknesses

1. Implement Proxmox exporter first:
   - Add runtime -> Proxmox schema mapping.
   - Render deterministic `.conf` text via schema renderer.
   - Add exporter tests for minimum supported fields and deterministic output.
2. Make storage config functional in both Proxmox directions:
   - Resolve Proxmox storage tokens using parsed storage config instead of static assumptions.
   - Use the same resolver for import and export.
3. Remove dead or duplicate parse paths:
   - Keep one parser entrypoint for Proxmox source text.
   - Delete or integrate unused parse helpers and placeholder parser module.
4. Improve ezkvm export robustness:
   - Render from typed runtime data directly, not from qemu-arg text inspection.
   - Replace TPM display-text parsing with typed TPM accessors in runtime model.

## Recommendations To Align Both Adapters

### 1) Align module approach and pipeline shape

Adopt the same internal structure in both format adapters:

- `schema.rs` for format schema types and text rendering/parsing boundaries.
- `parser.rs` for low-level source parsing helpers (if needed).
- `to_runtime.rs` for schema -> runtime mapping.
- `from_runtime.rs` for runtime -> schema mapping.
- `importer.rs` as a thin orchestrator.
- `exporter.rs` as a thin orchestrator.

### 2) Align naming conventions

Use symmetric naming patterns:

- `EzkvmSchema` and `ProxmoxSchema`
- `EzkvmImportArgs` and `ProxmoxImportArgs`
- `EzkvmExportArgs` and `ProxmoxExportArgs`
- `EzkvmToRuntimeMapper` and `ProxmoxToRuntimeMapper`
- `EzkvmFromRuntimeMapper` and `ProxmoxFromRuntimeMapper`

### 3) Keep importer/exporter thin and deterministic

- Importer responsibilities: read input, parse, validate, map to runtime.
- Exporter responsibilities: map from runtime, render deterministic output, write file.
- Keep format-specific mapping logic inside format adapter modules, not in shared runtime builder type extensions.

### 4) Align error and diagnostics model

- Use structured, path-oriented error reporting for both adapters.
- Keep parse errors and semantic mapping errors distinct.
- Preserve line-level parse context where available.

## Suggested Implementation Order

1. Implement Proxmox export path and baseline tests.
2. Introduce shared Proxmox storage resolver and wire it into import/export.
3. Remove duplicate/unused Proxmox importer parse code and placeholder parser module.
4. Refactor naming and file layout toward symmetric adapter structure.
5. Refactor ezkvm renderer to avoid QEMU arg and display-string introspection.
6. Add cross-format parity tests for shared supported feature subsets.

## Expected Outcome

Applying these recommendations will:

- Improve consistency between ezkvm and Proxmox adapters.
- Reduce fragile string-based reconstruction logic.
- Clarify module ownership and boundaries.
- Make future format additions easier to implement and review.
