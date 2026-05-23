# QEMU CMD Import

## Summary

Implement qemu-cmd import as a separate importer stack from Proxmox import.
When behavior overlaps, extract only importer-agnostic utilities into a shared common area and let both importers depend on that common layer.
Keep documentation separate as two distinct user workflows.

## Scope

- Add a new qemu command-line importer that reads captured `.qemu.cmd` files and produces ezkvm YAML.
- Keep qemu-cmd import implementation fully independent from Proxmox importer internals.
- Extract only generic import workflow pieces into shared `src/import/common/` modules.
- Add dedicated CLI, tests, fixtures, and user docs for qemu-cmd import.

## Non-Goals

- Rewriting Proxmox import around qemu-cmd import.
- Supporting raw command strings or stdin input in the first release.
- Making qemu-cmd import depend on Proxmox parser/model/mapper modules.

## Design Principles

1. Hard separation between importers:
- `src/import/qemu_cmd/*` must not import Proxmox parser/model/mapper code.
- Proxmox and qemu-cmd importers are peers under `src/import/`.

2. Shared logic only through neutral modules:
- Common helper code lives under `src/import/common/`.
- Shared modules are importer-agnostic in API, naming, and docs.

3. Independent documentation:
- qemu-cmd import docs are standalone and do not require reading Proxmox docs.

## M-01 Deliverables

Status: Done (2026-05-20)

This section captures the implementation contract artifacts required by `M-01`.

### 1. Importer Boundary Contract

Module ownership:
- `src/import/proxmox/**`: Proxmox-only parsing/mapping behavior.
- `src/import/qemu_cmd/**`: qemu-cmd-only parsing/mapping behavior.
- `src/import/common/**`: importer-agnostic orchestration helpers.

Separation rules:
- `src/import/qemu_cmd/**` must not import from:
	- `crate::import::proxmox::parser`
	- `crate::import::proxmox::model`
	- `crate::import::proxmox::mapper`
- `src/import/proxmox/**` must not import qemu-cmd parser/model/mapper internals.
- Cross-importer reuse is only allowed through `src/import/common/**`.

Allowed coupling points:
- Both importers may depend on shared app-level modules such as:
	- `crate::config::*`
	- `crate::qemu::*` (for validation/parity tests)
	- `crate::import::common::*`

Review gate for separation:
- Any PR touching qemu-cmd importer must include a quick audit note confirming no Proxmox parser/model/mapper dependency was introduced.

### 2. qemu-cmd Public API Contract

Public module contract (`src/import/qemu_cmd/mod.rs`):
- Re-export only high-level API:
	- `ImportRunOptions`
	- `ImportRunResult`
	- `ImportOutputMode` (qemu-cmd scoped or importer-common alias)
	- `ImportError`
	- `run_import_from_file(input_path: &str, options: &ImportRunOptions) -> Result<ImportRunResult, ImportError>`

Public type contract:
- `ImportRunOptions`:
	- `output_path: Option<String>`
	- `strict: bool`
	- `dry_run: bool`
	- `compact_lists: bool`
	- `output_mode: ImportOutputMode`
- `ImportRunResult`:
	- `output_path: String`
	- `yaml: String`
	- `warnings: Vec<MappingWarning>`
- `MappingWarning` minimum fields:
	- `source_field: String`
	- `message: String`

Error taxonomy contract:
- Parse failures: malformed/unsupported command-line syntax.
- Mapping failures: unsupported shape that cannot be represented.
- Validation failures: generated YAML fails deserialize/validate.
- I/O failures: file read/write issues.

Surface restriction:
- Parser/model/mapper implementation modules remain non-public to keep refactor freedom for M-02+.

### 3. Importer-Common Extraction Criteria (Checklist)

A function may move to `src/import/common/**` only if all checks pass:
- No Proxmox-only field assumptions.
- No qemu-cmd-only token/model assumptions.
- Name and API are source-agnostic.
- Works with canonical YAML/`VmConfig` contracts only.
- Has tests that cover reuse from at least one importer path, with second importer adoption planned.

Candidate common areas for M-02:
- Strict-warning evaluation helper.
- Output-path derivation helper.
- Generated YAML deserialize + validate helper.
- Output-mode render post-processing hooks (generic only).

Must remain importer-specific:
- Source parser/tokenization.
- Source intermediate model.
- Source-to-canonical mapping and warning classification.

## M-02 Deliverables

Status: Done (2026-05-20)

This section captures the implementation artifacts required by `M-02`.

### 1. Introduced importer-common module

Added neutral shared orchestration helpers under `src/import/common/**`:
- `src/import/common/io_contract.rs`
- `src/import/common/render.rs`
- `src/import/common/validate.rs`

Module wiring update:
- `src/import/mod.rs` now exports `common` and `proxmox` peer modules.

### 2. Extracted shared orchestration primitives

The following importer-agnostic concerns moved into `src/import/common/**`:
- Output path derivation:
	- default output file naming from input path.
- Output write contract:
	- dry-run aware write helper with deterministic error context.
- Strict-warning enforcement:
	- generic strict-mode warning gate with caller-provided warning formatting.
- Validation contract:
	- canonical YAML deserialize + validate helper based on `VmConfig` + config validation.
- Render post-processing helpers:
	- optional compact-list pass helper.
	- optional preamble injection helper.

### 3. Proxmox importer refactor

`src/import/proxmox/io.rs` now consumes shared helpers for:
- generated YAML validation,
- strict warning failure handling,
- output-path derivation,
- dry-run aware output write,
- render post-processing orchestration.

Kept Proxmox-specific in Proxmox importer:
- source parse/mapping flow,
- Proxmox compact/canonical/debug rendering rules,
- Proxmox-specific post-processing (`omit_reconstructable_hostpci_bus_addr`, `omit_default_spice_addr`, and controller/drive reshape).

### 4. Behavioral validation summary

No public `import-proxmox` API changes were introduced.
Focused importer tests passed after refactor:
- Proxmox import I/O tests (`import::proxmox::io::tests`).
- Broader Proxmox importer test subset (`import::proxmox::`).

## M-03 Deliverables

Status: Done (2026-05-20)

This section captures the implementation artifacts required by `M-03`.

### 1. Added qemu-cmd parser/model modules

Introduced qemu-cmd parser/model foundation under `src/import/qemu_cmd/**`:
- `src/import/qemu_cmd/mod.rs`
- `src/import/qemu_cmd/error.rs`
- `src/import/qemu_cmd/model.rs`
- `src/import/qemu_cmd/parser.rs`

Module wiring update:
- `src/import/mod.rs` now exports `qemu_cmd` alongside `common` and `proxmox`.

### 2. Parser capabilities implemented

`parse_qemu_cmd` now supports:
- shell-quoted tokenization with single quotes, double quotes, and escaped characters,
- repeated flags preserved in ordered option list,
- JSON payload parsing for structured option families (for example `-blockdev` and JSON-form `-object`),
- CSV-style argument decomposition for option families such as:
	- `-drive`
	- `-device`
	- `-netdev`
	- `-chardev`
	- `-machine`
	- `-cpu`
	- `-object`
	- `-spice`

The intermediate model preserves:
- executable path,
- ordered option groups,
- raw option values,
- parsed value shapes (`None`, `Scalar`, `Csv`, `Json`).

### 3. Error handling and malformed-input diagnostics

Parser returns actionable parse errors for malformed input, including:
- unterminated quoted strings,
- unterminated escape sequences,
- invalid JSON payloads for JSON-targeted options,
- unexpected non-flag tokens in option position.

### 4. Validation summary

Added parser unit tests that cover:
- quoted tokenization,
- repeated flags,
- JSON payload parsing,
- CSV decomposition into bare/key-value parts,
- malformed input diagnostics.

Added representative fixture parsing test over captured command lines:
- `input/felucia/108.qemu.cmd`
- `input/zbp-server-mh2/201.qemu.cmd`
- `input/coruscant/505.qemu.cmd`

Validation results during implementation:
- Focused qemu-cmd parser test subset passed (`cargo test --quiet import::qemu_cmd::`).
- Focused Proxmox importer regression subset remained green (`cargo test --quiet import::proxmox::`).

## M-04 Deliverables

Status: Done (2026-05-21)

This section captures the implementation artifacts required by `M-04`.

### 1. Added independent qemu-cmd mapper module

Introduced qemu-cmd mapping module under `src/import/qemu_cmd/mapper.rs` and wired it through `src/import/qemu_cmd/mod.rs`.

High-level contract added:
- `map_qemu_cmd_to_canonical_yaml(&QemuCmdModel) -> Result<CanonicalMappingResult, ImportError>`

### 2. Implemented warning taxonomy

Added qemu-cmd specific structured warning model:
- `MappingWarning { source_field, kind, message }`
- `MappingWarningKind` variants:
	- `UnsupportedFlag`
	- `UnsupportedValue`
	- `AmbiguousPairing`

Warning taxonomy use cases now covered:
- unsupported option flags that are not yet mapped,
- invalid/unsupported option payload values,
- ambiguous `-device`/`-netdev` pairings.

### 3. Implemented initial canonical mapping surface

Mapper now converts qemu-cmd intermediate model into canonical `VmConfig` YAML for supported option families:
- VM identity and core system shape:
	- `-name`, `-machine`, `-cpu`, `-m`, `-smp`
- network backend/device mapping:
	- `-netdev` + matching `-device ... netdev=<id>`
- SPICE mapping:
	- `-spice`

Network mapping behavior:
- maps supported models and backend fields into canonical `NetworkConfig` + `NetworkBackendConfig`,
- skips unsupported/ambiguous network forms with structured warnings.

### 4. Validation and test evidence

Added mapper-focused tests in `src/import/qemu_cmd/mapper.rs` for:
- canonical mapping of core/system/network/spice options,
- structured warnings for unsupported and ambiguous inputs,
- representative fixture mapping + canonical YAML validation for:
	- `input/felucia/108.qemu.cmd`
	- `input/zbp-server-mh2/201.qemu.cmd`
	- `input/coruscant/505.qemu.cmd`

Validation results during implementation:
- Focused qemu-cmd parser+mapper subset passed (`cargo test --quiet import::qemu_cmd::`).
- Generated YAML from supported fixture mapping paths deserialized and passed config validation in mapper tests.

## M-05 Deliverables

Status: Done (2026-05-21)

This section captures the implementation artifacts required by `M-05`.

### 1. Added qemu-cmd I/O pipeline module

Introduced `src/import/qemu_cmd/io.rs` with high-level orchestration contract:
- `run_import_from_files(input_path, options)`

Pipeline sequence implemented:
- read input file,
- parse via `parse_qemu_cmd`,
- map via `map_qemu_cmd_to_canonical_yaml`,
- validate generated YAML,
- enforce strict-warning policy,
- render output-mode YAML,
- write output unless dry-run.

### 2. Added qemu-cmd run contracts

`src/import/qemu_cmd/io.rs` now defines qemu-cmd specific contracts:
- `ImportOutputMode` (`Canonical`, `Compact`, `DebugCanonical`),
- `ImportRunOptions` (`output_path`, `strict`, `dry_run`, `output_mode`),
- `ImportRunResult` (`output_path`, `yaml`, `warnings`).

Module wiring update:
- `src/import/qemu_cmd/mod.rs` now exports `io` and re-exports high-level qemu-cmd run contracts.

### 3. Reused importer-common helpers only

The qemu-cmd I/O flow reuses importer-common helpers for shared concerns:
- strict-mode enforcement,
- default output-path derivation,
- dry-run-aware write behavior,
- generated YAML validation,
- debug preamble rendering.

No Proxmox parser/model/mapper internals are referenced in qemu-cmd I/O orchestration.

### 4. Focused M-05 tests added

Added I/O-focused tests in `src/import/qemu_cmd/io.rs` for:
- strict mode failure when warnings are present,
- deterministic default output-path behavior,
- dry-run success without writing output,
- non-dry-run output file write behavior.

## M-06 Deliverables

Status: Done (2026-05-21)

This section captures the implementation artifacts required by `M-06`.

### 1. Added dedicated CLI subcommand

Introduced `import-qemu-cmd` in `src/cli/types.rs` with mode-specific examples and flags:
- positional input path (`<vm>.qemu.cmd`),
- `--output` optional output path,
- `--dry-run`,
- `--strict`,
- `--output-mode {canonical|compact|debug}`.

### 2. Added qemu-cmd command handler and dispatch wiring

Added handler module:
- `src/cli/commands/import_qemu_cmd.rs`

Wired module exports and execute dispatch:
- `src/cli/commands/mod.rs`
- `src/cli/execute.rs`

Handler behavior mirrors existing importer UX:
- deterministic dry-run preamble and YAML emission,
- deterministic non-dry-run completion summary,
- structured warning printing in both modes,
- actionable command-level error prefix (`qemu-cmd import failed: ...`).

### 3. Added CLI parse coverage for M-06 flags

Added CLI parser tests in `src/cli/tests/mod.rs` for:
- default `import-qemu-cmd` invocation shape,
- explicit flag parsing (`--output`, `--dry-run`, `--strict`, `--output-mode debug`).

These tests verify command/flag wiring independently of mapper/runtime behavior.

## M-07 Deliverables

Status: Done (2026-05-21)

This section captures the implementation artifacts required by `M-07`.

### 1. Added representative qemu-cmd fixture set

Introduced fixture directory:
- `tests/fixtures/qemu_cmd_import/`

Added representative captured command fixtures:
- `01-wakiza.qemu.cmd`
- `02-zbp-201.qemu.cmd`
- `03-felucia-505.qemu.cmd`

These fixtures cover warning-rich and mixed-option command lines including repeated flags, CSV payloads, and JSON payload options.

### 2. Added fixture-driven integration flow coverage

Added integration module:
- `tests/integration/qemu_cmd_import.rs`

Coverage includes:
- import run over each qemu-cmd fixture,
- generated YAML deserialize + validation,
- command-builder dry-run path (`QemuManager::build_command`) from imported config,
- strict-mode failure behavior on warning-rich fixture input.

### 3. Added output-mode runtime equivalence coverage

Added integration module:
- `tests/integration/qemu_cmd_import_output_modes.rs`

Coverage includes:
- `canonical`, `compact`, and `debug` import output modes,
- debug comment stripping for semantic load,
- command generation equivalence assertion across modes.

### 4. Integration harness wiring

Wired new modules in:
- `tests/integration_tests.rs`

This keeps qemu-cmd integration coverage in the same integration test entrypoint used by existing importer suites.

## M-08 Deliverables

Status: Done (2026-05-21)

This section captures the implementation artifacts required by `M-08`.

### 1. Added standalone user documentation page

Introduced dedicated qemu-cmd import guide:
- `doc/user/how-to/import/qemu-cmd.md`

Guide content includes:
- workflow separation from Proxmox import,
- first-release mapping scope and limitations,
- CLI usage examples,
- output-mode behavior,
- warning taxonomy and interpretation,
- post-import validation checklist and troubleshooting pointers.

### 2. Updated user docs index and cross-links

Updated:
- `doc/user/reference/config/README.md`

Changes include:
- separate special-topic entry for `import-qemu-cmd.md`,
- quick-link entry for captured QEMU command import,
- reference-table row for qemu-cmd import guide.

### 3. Independence contract reflected in user docs

Documentation now allows users to execute qemu-cmd import workflow without reading Proxmox import guide.

Proxmox and qemu-cmd guides remain separate and cross-linked only as related alternatives.

## M-09 Deliverables

Status: Done (2026-05-21)

This section captures the implementation artifacts required by `M-09`.

### 1. Added qemu-cmd profile inference hardening

Updated `src/import/qemu_cmd/mapper.rs` to infer profiles from qemu-cmd-native signals:
- `windows-common` and `windows-11` for Hyper-V + secure-boot + TPM style workloads,
- `linux-l26-common` for guest-agent Linux-style workloads without Windows/macOS signals,
- `macos-kvm` when Apple SMC or SMBIOS type-2 signals are present.

The implementation remains independent from Proxmox parser/model/mapper internals.

### 2. Added targeted profile inference tests

Added coverage in both mapper unit tests and integration tests:
- mapper unit tests validate inference branches directly,
- `tests/integration/qemu_cmd_import_profiles.rs` validates three distinct workload classes (Windows, Linux, macOS) from captured command fixtures.

### 3. Added parity and warning stability assertions

Extended `tests/integration/qemu_cmd_import_output_modes.rs` to assert:
- warning `source_field` stability across `canonical`, `compact`, and `debug` output modes,
- deterministic dry-run parity behavior by requiring stable generated QEMU args across repeated imports for selected fixtures.

## Implementation Plan

1. Define hard module boundaries.
- Create a dedicated qemu-cmd importer under `src/import/qemu_cmd` with its own parser, model, mapper, io, and error modules.
- Prevent any direct reuse of Proxmox parser/model/mapper modules from qemu-cmd importer code.

2. Extract overlap into shared neutral modules.
- Identify generic import orchestration logic currently embedded in Proxmox flow (strict warning handling, output path derivation, generated-YAML validation, render pipeline steps).
- Move only generic pieces into `src/import/common`.
- Refactor Proxmox importer to use `src/import/common`, then build qemu-cmd importer on the same common contracts.

3. Build qemu-cmd parser and model.
- Parse file-based command input from `input/<host>/*.qemu.cmd`.
- Support shell quoting, repeated flags, JSON payloads, and ordered option groups.
- Produce qemu-cmd-specific diagnostics and warnings.

4. Build independent qemu-cmd mapper.
- Map representable options into canonical `VmConfig`.
- Keep unsupported and ambiguous constructs as explicit warnings.
- Keep qemu-cmd profile inference implementation separate from Proxmox profile inference logic.

5. Add independent qemu-cmd import orchestration.
- Add a high-level run function in qemu-cmd io module.
- Reuse only extracted common helpers for validation and rendering.
- Keep qemu-cmd option/result structs distinct from Proxmox option/result structs.

6. Add separate CLI command.
- Add `import-qemu-cmd` command in CLI types and execute dispatch.
- Add separate handler file for qemu-cmd import command.
- Reuse generic CLI enums/flags where applicable, but keep command help and semantics mode-specific.

7. Add dedicated qemu-cmd tests and fixtures.
- Add integration suites parallel to Proxmox coverage but in separate files.
- Add dedicated fixtures folder for qemu-cmd import.
- Add parser unit tests and common-helper unit tests.

8. Add separate qemu-cmd documentation.
- Create a standalone qemu-cmd import doc page.
- Keep Proxmox import doc focused on Proxmox config import only.
- Update docs index pages to link both modes independently.

9. Add separate backlog/design track.
- Add a dedicated feature doc and ticket chain for qemu-cmd import.
- Track shared-common extraction as prerequisites in backlog dependencies.

## Relevant Files

### Existing files to modify

- `src/import/mod.rs`
- `src/import/proxmox/mod.rs`
- `src/import/proxmox/io.rs`
- `src/cli/types.rs`
- `src/cli/execute.rs`
- `src/cli/commands/mod.rs`
- `tests/integration_tests.rs`
- `doc/user/reference/config/README.md`
- `doc/backlog/BACKLOG.md`
- `doc/backlog/TRACKING_BOARD.md`

### New files/directories to create

- `src/import/common/mod.rs`
- `src/import/common/io_contract.rs`
- `src/import/common/render.rs`
- `src/import/common/validate.rs`
- `src/import/qemu_cmd/mod.rs`
- `src/import/qemu_cmd/error.rs`
- `src/import/qemu_cmd/model.rs`
- `src/import/qemu_cmd/parser.rs`
- `src/import/qemu_cmd/mapper.rs`
- `src/import/qemu_cmd/io.rs`
- `src/cli/commands/import_qemu_cmd.rs`
- `tests/integration/qemu_cmd_import.rs`
- `tests/integration/qemu_cmd_import_output_modes.rs`
- `tests/integration/qemu_cmd_import_profiles.rs`
- `tests/fixtures/qemu_cmd_import/`
- `doc/user/how-to/import/qemu-cmd.md`

## Verification

1. Confirm no qemu-cmd importer module imports Proxmox parser/model/mapper modules.
2. Confirm extracted common modules are importer-agnostic and used by both importers.
3. Parser unit tests: quoting, escaped payloads, repeated flags, JSON payloads, malformed lines.
4. Integration tests: qemu-cmd fixture to YAML to `VmConfig` load to dry-run QEMU args build.
5. Output-mode equivalence tests for canonical, compact, and debug under qemu-cmd importer.
6. Docs check: qemu-cmd docs are complete and independent from Proxmox docs.
7. Backlog/tracking consistency check for new qemu-cmd feature chain.

## Decisions

- qemu-cmd import is fully separate from Proxmox import implementation.
- Shared functionality is extracted into importer-common modules when overlap is real.
- qemu-cmd documentation is standalone.
- Initial input contract remains file path to captured `.qemu.cmd` files.

## Further Considerations

1. Shared-module granularity: start with minimal common extraction (`validate`, `render`, `io_contract`) and expand only when duplication is proven.
2. Runtime-target semantics for qemu-cmd import: keep independent defaults and document them in qemu-cmd guide.
3. Snapshot strategy: normalize arguments semantically to avoid false diffs caused by quoting/order artifacts in original captured command text.
