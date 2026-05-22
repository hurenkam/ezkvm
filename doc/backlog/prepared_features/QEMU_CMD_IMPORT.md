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
- `doc/user/config/README.md`
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
- `doc/user/config/import-qemu-cmd.md`

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
