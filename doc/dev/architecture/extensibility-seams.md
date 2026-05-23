# Extensibility Seams

Date: 2026-04-19  
Status: Active  
Implements: D-01  
Related: `doc/dev/architecture/decisions/ADR-0004-trait-seam-policy.md`, `doc/dev/architecture/module-ownership.md`, `doc/preparation/PROXMOX_PORTABILITY.md`

## Purpose

This document turns ADR-0004 into a concrete seam map for this repository.

It answers three practical questions:

1. Where trait-based extensibility is allowed now.
2. What those trait boundaries are expected to look like.
3. What must not cross those boundaries.

This is a design document for D-01. It does not require a registry implementation yet. That work remains in D-02.

## Core Rules

All extensibility work must follow these rules:

1. Traits are allowed only at explicit seams.
2. Traits are compile-time only. No runtime plugin loading.
3. Trait implementations must not invert layer direction.
4. Trait results must re-enter the normal canonical pipeline.
5. Trait seams are for extension boundaries, not for general code reuse.

The governing dependency direction remains:

`CLI -> Config -> QEMU -> Runtime -> OS`

The import seam is a bounded exception only in the sense that `import/` normalizes into `config/`. It does not grant permission for importer implementations to orchestrate runtime or emit raw process behavior directly.

## Approved Seams

## 1. Import Format Adapter Seam

### Owner

`src/import/`

### Purpose

Normalize an external VM description into canonical ezkvm config and import reporting.

### Why this seam exists

Different external formats can vary widely in parsing and source-specific interpretation, but they all must converge on the same canonical config contract.

This seam supports:

- Proxmox import
- future libvirt import
- future other external config sources

### Allowed responsibilities

- parse external source input
- build any intermediate import model required by that format
- map into canonical config types
- produce structured warnings and import metadata

### Forbidden responsibilities

- running QEMU
- mutating central config at runtime
- selecting process-level execution strategy
- bypassing canonical validation
- returning format-specific runtime schema

### Recommended trait boundary

```rust
pub trait ImportFormatAdapter: Send + Sync {
    fn format_id(&self) -> &'static str;

    fn import(
        &self,
        request: &ImportRequest,
    ) -> anyhow::Result<ImportResult>;
}
```

### Expected boundary types

```rust
pub struct ImportRequest {
    pub source_name: String,
    pub input_text: String,
    pub options: ImportRunOptions,
}

pub struct ImportResult {
    pub config: VmConfig,
    pub report: ImportReport,
}
```

### Example

- `ProxmoxImportAdapter` reads a Proxmox `.conf`, maps it into canonical `VmConfig`, and returns any unsupported-field warnings in `ImportReport`.

### Layering guarantee

This seam stops at canonical config production. Control returns to the normal config validation and CLI/application flow after import.

## 2. Runtime Capability Provider Seam

### Owner

Runtime-oriented orchestration, using validated host facts from `config/`.

### Purpose

Resolve host-specific runtime details without pushing those details into guest-semantic config or importer logic.

This seam is the extensibility boundary that matches the portability design in `doc/preparation/PROXMOX_PORTABILITY.md`.

### Why this seam exists

Portable execution needs host-specific decisions for:

- runtime directory roots
- firmware lookup
- bridge helper or alternative network backend selection
- swtpm binary and state placement
- optional helper integrations such as Looking Glass

These are host realization concerns, not guest contract concerns.

### Allowed responsibilities

- resolve effective host paths and helper binaries
- choose host-specific backend implementation from validated policy
- expose optional/required capability availability
- provide preflight diagnostics for missing capabilities

### Forbidden responsibilities

- changing guest-visible topology
- rewriting validated VM semantics
- parsing CLI arguments directly
- reading raw import formats directly
- leaking host policy concerns into canonical schema ownership

### Recommended trait boundary

```rust
pub trait RuntimeCapabilityProvider: Send + Sync {
    fn capability_id(&self) -> &'static str;

    fn resolve_runtime_context(
        &self,
        request: &RuntimeCapabilityRequest,
    ) -> anyhow::Result<RuntimeCapabilityContext>;
}
```

### Expected boundary types

```rust
pub struct RuntimeCapabilityRequest {
    pub vm_name: String,
    pub vm_config: VmConfig,
    pub host_config: CentralConfig,
}

pub struct RuntimeCapabilityContext {
    pub runtime_dir: std::path::PathBuf,
    pub firmware: FirmwareResolution,
    pub network: NetworkResolution,
    pub tpm: TpmResolution,
    pub optional_features: OptionalFeatureResolution,
}
```

### Example

- `PortableLinuxCapabilityProvider` uses validated central host config to resolve firmware files, network helper policy, swtpm paths, and optional Looking Glass integration without changing guest-visible VM topology.

### Layering guarantee

The provider consumes validated config and produces resolved host context for runtime orchestration. It does not mutate schema ownership and it does not call back upward into CLI.

## Seam Matrix

| Seam | Owner | Input | Output | Must not do |
|---|---|---|---|---|
| Import format adapter | `import/` | external config text + import options | canonical `VmConfig` + `ImportReport` | run QEMU, invent alternate schema, bypass validation |
| Runtime capability provider | runtime orchestration with `config/` inputs | canonical `VmConfig` + central host config | resolved host runtime context | change guest semantics, parse CLI, own schema merge rules |

## Explicit Non-Seams

The following areas are not approved trait seams in D-01:

1. `cli/` command parsing
2. canonical schema merge logic in `config/`
3. core QEMU arg emission hot paths in `qemu/`
4. `state/` metadata storage access
5. general validation rule dispatch

These areas should remain direct, typed code unless a later ADR explicitly opens a new seam.

## Anti-Patterns

The following patterns violate D-01 even if traits are involved:

1. Import adapters that emit raw QEMU CLI instead of canonical config.
2. Runtime providers that silently rewrite guest-visible PCI, CPU, TPM, or device semantics.
3. Trait objects stored in mutable global singletons.
4. Upward callbacks from runtime or import extensions into CLI handlers.
5. Trait proliferation inside normal hot-path code that does not need extension behavior.

## Examples Of Correct Composition

### Import flow

```text
CLI import command
  -> import adapter selected by format
  -> canonical VmConfig + ImportReport
  -> config validation
  -> output/compaction/dry-run flow
```

### Portable runtime flow

```text
CLI start command
  -> config load + profile merge + validation
  -> runtime capability provider resolves host context
  -> qemu/runtime orchestration uses resolved context
  -> OS/tool execution
```

## Review Checklist For D-01 Seams

When reviewing a new trait seam or trait impl, confirm:

1. Is this seam one of the approved seam types in this document?
2. Does the trait live at a real extension boundary instead of a convenience abstraction?
3. Does the implementation preserve dependency direction?
4. Does import still return canonical config only?
5. Does runtime capability resolution avoid rewriting guest semantics?
6. Could the same outcome be achieved more simply with plain functions? If yes, do not add a trait.

## Current Status

As of D-01:

- seam policy is accepted in ADR-0004
- concrete seam definitions now exist in this document
- registry implementation is deferred to D-02
- one real extension implementation is deferred to D-03

## References

- `doc/dev/architecture/decisions/ADR-0004-trait-seam-policy.md`
- `doc/dev/architecture/module-ownership.md`
- `doc/preparation/PROXMOX_PORTABILITY.md`
- `doc/backlog/BACKLOG.md`