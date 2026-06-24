# QEMU Cmd Import/Export Migration Plan

Date: 2026-06-23
Status: Initial phase implemented
Scope: `src/config_format/qemu_cmd/**`

## Goal

Implement a proper `qemu_cmd` importer/exporter that is consistent with existing `ezkvm` and `proxmox` adapters, using the same 4-stage adapter architecture:

- `Parser` (text -> schema)
- `RuntimeBuilder` (schema -> runtime)
- `SchemaBuilder` (runtime -> schema)
- `Marshaler` (schema -> text)

## Current State

- `qemu_cmd` uses stage modules (`schema`, `parser`, `runtime_builder`, `schema_builder`, `marshaler`) with thin importer/exporter orchestration.
- `qemu_cmd/builder.rs` legacy utilities were removed.
- Current mapping targets the initial shared subset (name, machine/chipset, cpu topology, memory).
- Parser preserves full argv in schema so unsupported flags remain available as passthrough args during parse/marshal roundtrips.

## Target Architecture

Align `qemu_cmd` with `ezkvm` and `proxmox` module shape:

- `schema.rs`
- `parser.rs`
- `runtime_builder.rs`
- `schema_builder.rs`
- `marshaler.rs`
- `importer.rs` (thin orchestrator)
- `exporter.rs` (thin orchestrator)
- `mod.rs` (module wiring + re-exports)

### Stage Responsibilities

1. `QemuParser`
- Parse command-file text into typed schema.
- Handle quoting and line continuation deterministically.
- Return parse errors distinct from semantic mapping errors.

2. `QemuRuntimeBuilder`
- Map typed schema into `RuntimeModel`.
- Start with shared supported subset first.
- Preserve clear, contextual error messages when unsupported patterns are encountered.

3. `QemuSchemaBuilder`
- Build typed qemu schema from `RuntimeModel`.
- Use typed runtime data (no string re-parsing).
- Keep deterministic argument ordering.

4. `QemuMarshaler`
- Render schema to canonical command text.
- Apply deterministic escaping and formatting.

## Shared Supported Subset (Phase 1)

Implement first for fields that already map cleanly across adapters:

- VM name
- Machine/chipset (`q35` baseline)
- CPU model/topology (`host`, `cores`, `sockets`, `threads`)
- Memory
- UEFI boot (when representable)
- Baseline storage devices (SCSI/IDE/SATA where already supported)
- Baseline virtio network bridge/tap where representable

Any feature outside this subset should fail with explicit structured errors or be preserved as passthrough metadata when that strategy is introduced.

## Importer/Exporter Orchestration Contract

Keep orchestration thin and deterministic:

### Importer

1. Read source file
2. `QemuParser.parse(source)`
3. `QemuRuntimeBuilder.build(schema)`
4. Return `RuntimeModel`

### Exporter

1. `QemuSchemaBuilder.build(runtime)`
2. `QemuMarshaler.marshal(&schema)`
3. Write output file
4. Return output path

## Decision on Legacy `builder.rs`

### Recommendation

Treat `QemuArgsBuilder` and `QemuCommandBuilder` as obsolete under the stage architecture.

### Keep only if

- They are repurposed as private internal helpers used by `QemuSchemaBuilder`, and
- They do not create a parallel architecture that bypasses stages.

### Preferred outcome

- Migrate functionality into stage modules.
- Remove `QemuArgsBuilder` trait and `QemuCommandBuilder` once replacement is complete.

## Migration Steps

1. Add stage modules for `qemu_cmd`:
- `schema.rs`
- `parser.rs`
- `runtime_builder.rs`
- `schema_builder.rs`
- `marshaler.rs`

2. Rewire `qemu_cmd/mod.rs`:
- Declare new modules.
- Re-export stage structs/types similarly to `ezkvm`/`proxmox`.

3. Implement thin orchestration in `qemu_cmd/importer.rs` and `qemu_cmd/exporter.rs`.

4. Bridge or remove legacy builder utilities:
- If reused temporarily, keep private and local to `qemu_cmd` stage implementation.
- Remove dead abstractions in same task when feasible.

5. Add tests (see strategy below).

6. Run validation gates:
- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --quiet`

## Test Strategy

### A. Parser tests (`parser.rs`)

Cover command text parsing robustness:

- basic single-line command parsing
- quoted args and escaped spaces
- multiline continuation with `\`
- malformed input failures with clear diagnostics

### B. RuntimeBuilder tests (`runtime_builder.rs`)

Cover schema -> runtime mapping:

- machine/cpu/memory mapping
- UEFI mapping when present
- storage/network mapping for shared subset
- explicit errors for unsupported/ambiguous patterns

### C. SchemaBuilder tests (`schema_builder.rs`)

Cover runtime -> schema mapping:

- deterministic token ordering
- required fields always emitted
- stable rendering inputs for marshaler

### D. Marshaler tests (`marshaler.rs`)

Cover schema -> text rendering:

- canonical formatting
- quoting/escaping correctness
- deterministic output for same schema

### E. Adapter end-to-end tests (`importer.rs` and `exporter.rs`)

- import command file to `RuntimeModel`
- export `RuntimeModel` to command file
- invalid options rejected (`InvalidFormat`)

### F. Cross-format parity tests (`src/config_format/parity_tests.rs`)

Extend shared-subset parity coverage to include qemu_cmd once implementation exists:

- qemu -> ezkvm/proxmox semantic parity for shared fields
- ezkvm/proxmox -> qemu semantic parity for shared fields
- roundtrip stability on shared subset (semantics, not format-specific extras)

## Non-Goals (for initial phase)

- Full fidelity for all qemu flags and device permutations.
- Exact preservation of arbitrary argument ordering from unknown inputs.
- Representation of every advanced accelerator/device option in phase 1.

## Completion Criteria

`qemu_cmd` is considered migrated when:

1. Importer/exporter are implemented (not placeholders).
2. Stage modules exist and are used consistently.
3. Legacy builder abstractions are removed or clearly internalized without architectural duplication.
4. Shared-subset parity tests include qemu_cmd paths.
5. fmt, clippy, and tests all pass.
