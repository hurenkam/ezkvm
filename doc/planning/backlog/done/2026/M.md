# Epic M Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth: this file (doc/planning/backlog lifecycle).

## Completed Tickets

| ID | Title | Completion Date | Notes |
|---|---|---|---|
| M-01 | Define qemu-cmd importer boundary and public contracts | 2026-05-20 | Boundary contract and importer-common extraction checklist defined |
| M-02 | Extract importer-common orchestration helpers | 2026-05-20 | Shared importer helpers extracted and Proxmox pipeline refactored to consume them |
| M-03 | Implement qemu-cmd parser and intermediate model | 2026-05-20 | Parser/model coverage added for representative qemu command families |
| M-04 | Implement qemu-cmd mapper and warning taxonomy | 2026-05-21 | Canonical mapper and structured warning taxonomy implemented |
| M-05 | Implement qemu-cmd import I/O pipeline | 2026-05-21 | End-to-end qemu-cmd import orchestration with dry-run and strict mode added |
| M-06 | Add import-qemu-cmd CLI command | 2026-05-21 | Dedicated CLI subcommand and command handler integrated |
| M-07 | Add qemu-cmd fixtures and integration coverage | 2026-05-21 | Fixture-driven integration suite and output-mode equivalence coverage added |
| M-08 | Publish standalone qemu-cmd import documentation | 2026-05-21 | Standalone user documentation and doc-index links published |
| M-09 | Harden qemu-cmd profile inference and parity assertions | 2026-05-21 | Profile inference and parity/warning stability regression assertions hardened |

## Ticket Definitions

### M-01 Define qemu-cmd importer boundary and public contracts
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: B-40

Scope:
- Define strict separation rules between `src/import/qemu_cmd/` and `src/import/proxmox/`.
- Specify qemu-cmd high-level API contracts (`run options`, `result`, `warnings`, `errors`).
- Document importer-common extraction criteria (what is generic vs Proxmox-specific).

Acceptance Criteria:
- Boundary contract documented and linked from feature design notes.
- qemu-cmd importer API shape approved for implementation.
- Common-extraction checklist added for review use.

Estimate: 1 day

Completion Notes (2026-05-20):
- Feature document moved to in-progress: `doc/backlog/in_progress_features/QEMU_CMD_IMPORT.md`.
- Added explicit importer boundary contract, qemu-cmd public API contract, and importer-common extraction checklist.
- M-02 and M-03 can proceed using the documented contracts.

### M-02 Extract importer-common orchestration helpers
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-01

Scope:
- Extract importer-agnostic logic into `src/import/common/` (validation, render pipeline helpers, output-path and strict-warning handling primitives).
- Refactor Proxmox importer to consume extracted helpers without behavior regressions.

Acceptance Criteria:
- `src/import/common/` introduced with neutral naming and no Proxmox semantics.
- Proxmox importer uses shared helpers and existing tests remain green.
- No public API regressions for `import-proxmox` command.

Estimate: 2 days

Completion Notes (2026-05-20):
- Added importer-common module under `src/import/common/` with shared `io_contract`, `render`, and `validate` helpers.
- Refactored `src/import/proxmox/io.rs` to consume shared validation, strict-warning, output-path/write, and render post-processing primitives.
- Kept Proxmox-specific mapping/render behavior in Proxmox modules; no `import-proxmox` API shape change.
- Focused importer test suites remained green after refactor.

### M-03 Implement qemu-cmd parser and intermediate model
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-01

Scope:
- Add qemu-cmd parser/model modules under `src/import/qemu_cmd/`.
- Support shell-quoted tokenization, repeated flags, and JSON payload options.
- Parse representative command families (`-drive`, `-blockdev`, `-device`, `-netdev`, `-chardev`, `-machine`, `-cpu`, `-object`, `-spice`).

Acceptance Criteria:
- Parser converts representative `input/<host>/*.qemu.cmd` samples into intermediate model.
- Malformed input yields actionable parser errors.
- Unit tests cover quoting and complex option payloads.

Estimate: 3 days

Completion Notes (2026-05-20):
- Added qemu-cmd parser/model foundation under `src/import/qemu_cmd/{error,model,parser}.rs` and wired module export via `src/import/mod.rs`.
- Implemented shell-quoted tokenization, repeated-flag capture, JSON payload parsing (`-blockdev`/JSON `-object`), and CSV decomposition for representative command families (`-drive`, `-blockdev`, `-device`, `-netdev`, `-chardev`, `-machine`, `-cpu`, `-object`, `-spice`).
- Added unit coverage for malformed quotes, invalid JSON payloads, repeated flags, and CSV decomposition, plus representative fixture parsing for `input/felucia/108.qemu.cmd`, `input/zbp-server-mh2/201.qemu.cmd`, and `input/coruscant/505.qemu.cmd`.
- Focused parser and Proxmox regression subsets were green after implementation.

### M-04 Implement qemu-cmd mapper and warning taxonomy
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-03

Scope:
- Map qemu-cmd intermediate model to canonical `VmConfig`.
- Define qemu-cmd-specific warning taxonomy for unsupported and ambiguous options.
- Keep profile inference logic independent from Proxmox mapper implementation.

Acceptance Criteria:
- Generated YAML deserializes into `VmConfig` and passes validation for supported fixtures.
- Unsupported options are surfaced via structured warnings.
- Mapper code has no dependency on Proxmox parser/model/mapper modules.

Estimate: 3 days

Completion Notes (2026-05-21):
- Added independent qemu-cmd mapper module at `src/import/qemu_cmd/mapper.rs` and wired it via `src/import/qemu_cmd/mod.rs`.
- Implemented canonical mapping for core/system options (`-name`, `-machine`, `-cpu`, `-m`, `-smp`), network pairing (`-netdev` + `-device ... netdev=<id>`), and `-spice`.
- Added structured warning taxonomy (`MappingWarningKind::{UnsupportedFlag, UnsupportedValue, AmbiguousPairing}`) and warning records with source field + message.
- Added mapper tests verifying canonical YAML validation for supported fixture mappings from `input/felucia/108.qemu.cmd`, `input/zbp-server-mh2/201.qemu.cmd`, and `input/coruscant/505.qemu.cmd`.
- Mapper implementation is isolated to qemu-cmd modules and does not import Proxmox parser/model/mapper internals.

### M-05 Implement qemu-cmd import I/O pipeline
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-02, M-04

Scope:
- Add high-level qemu-cmd import orchestration (`read -> parse -> map -> validate -> render -> write/dry-run`).
- Reuse only extracted `src/import/common/` helpers for shared behavior.
- Provide qemu-cmd specific run options and result contracts.

Acceptance Criteria:
- qemu-cmd import run function supports dry-run, strict mode, and output-path selection.
- Strict mode fails when warnings are present.
- Canonical output path behavior is deterministic.

Estimate: 2 days

Completion Notes (2026-05-21):
- Added qemu-cmd I/O pipeline module at `src/import/qemu_cmd/io.rs` implementing `read -> parse -> map -> validate -> render -> write/dry-run` orchestration.
- Added qemu-cmd specific run contracts: `ImportRunOptions`, `ImportRunResult`, and `ImportOutputMode`.
- Reused importer-common helpers for strict-mode warning enforcement, output-path derivation, dry-run-aware write behavior, and generated YAML validation.
- Added focused I/O tests covering strict-mode failure on warnings, deterministic default output path behavior, dry-run non-write behavior, and non-dry-run output writing.

### M-06 Add `import-qemu-cmd` CLI command
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-05

Scope:
- Add dedicated CLI subcommand and handler.
- Wire clap types, command dispatch, and import invocation flow.
- Keep help/examples specific to qemu-cmd import workflow.

Acceptance Criteria:
- `ezkvm --help` includes `import-qemu-cmd` with examples.
- Command supports dry-run and output mode flags.
- Error messages are deterministic and actionable.

Estimate: 1 day

Completion Notes (2026-05-21):
- Added dedicated `import-qemu-cmd` CLI subcommand in `src/cli/types.rs` with mode-specific help/examples.
- Implemented command handler at `src/cli/commands/import_qemu_cmd.rs` and wired dispatch in `src/cli/execute.rs`.
- Reused qemu-cmd import run contracts from `src/import/qemu_cmd/io.rs` with deterministic dry-run and non-dry-run output reporting.
- Added CLI parse tests covering default and explicit `import-qemu-cmd` flags in `src/cli/tests/mod.rs`.

### M-07 Add qemu-cmd fixtures and integration coverage
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-06

Scope:
- Add fixture set under `tests/fixtures/qemu_cmd_import/` from representative `.qemu.cmd` captures.
- Add integration tests for import-to-validate-to-dry-run flow.
- Add output-mode equivalence and round-trip semantic checks.

Acceptance Criteria:
- Fixture-driven integration tests pass for representative Linux and Windows command captures.
- Output modes (`canonical`, `compact`, `debug`) preserve runtime equivalence.
- Snapshot drift is deterministic and reviewable.

Estimate: 3 days

Completion Notes (2026-05-21):
- Added representative fixture set under `tests/fixtures/qemu_cmd_import/` from captured command lines (`01-wakiza`, `02-zbp-201`, `03-felucia-505`).
- Added fixture-driven integration coverage in `tests/integration/qemu_cmd_import.rs` for import -> YAML validate -> QEMU dry-run command build flow.
- Added output-mode runtime equivalence test in `tests/integration/qemu_cmd_import_output_modes.rs` comparing `canonical`, `compact`, and `debug` command generation.
- Wired new integration modules via `tests/integration_tests.rs` and verified focused qemu-cmd integration suites pass.

### M-08 Publish standalone qemu-cmd import documentation
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-06

Scope:
- Add dedicated user guide for qemu-cmd import under `doc/user/how-to/import/`.
- Update docs index so qemu-cmd and Proxmox import are separate entries.
- Document separation contract and limitations for first release scope.

Acceptance Criteria:
- New qemu-cmd doc is complete without requiring Proxmox import doc.
- Docs include examples, warning model, and post-import validation checklist.
- User docs cross-links are valid and consistent.

Estimate: 1 day

Completion Notes (2026-05-21):
- Added standalone user guide at `doc/user/how-to/import/qemu-cmd.md` covering scope, separation contract, first-release limitations, CLI examples, warning model, and post-import validation checklist.
- Updated config docs index at `doc/user/reference/config/README.md` to include the qemu-cmd import guide as a separate special-topic entry alongside Proxmox import.
- Added quick-link and table cross-references so users can discover qemu-cmd import flow without relying on Proxmox documentation.

### M-09 Harden qemu-cmd profile inference and parity assertions
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:qemu-cmd-import, phase:2-hardening
Assignee: unassigned
Dependencies: M-07

Scope:
- Add targeted tests for profile inference from qemu-cmd imports.
- Add assertions for stable runtime parity behavior on selected fixtures.
- Ensure warning/report output remains stable across output modes.

Acceptance Criteria:
- Profile inference tests cover at least 3 distinct workload classes.
- Parity assertions catch regressions in generated dry-run arguments.
- Warning field assertions are stable across output modes.

Estimate: 2 days

Completion Notes (2026-05-21):
- Added qemu-cmd profile inference in `src/import/qemu_cmd/mapper.rs` for three workload classes: Windows (`windows-common` + `windows-11`), Linux (`linux-l26-common`), and macOS (`macos-kvm`) using qemu-cmd-local signals.
- Added mapper unit tests covering profile inference heuristics and kept validation on representative fixtures with repo profile context.
- Added integration profile-inference coverage in `tests/integration/qemu_cmd_import_profiles.rs` validating distinct Windows/Linux/macOS fixture classes.
- Expanded output-mode integration assertions in `tests/integration/qemu_cmd_import_output_modes.rs` to enforce warning-field stability across `canonical`, `compact`, and `debug` modes.
- Added deterministic dry-run parity assertions on selected qemu-cmd fixtures by requiring repeated import+command-build argument equality.

## Completion Detail

Milestone-level completion notes:
- `M-01` to `M-03` established importer contracts, shared helpers, and parser foundations.
- `M-04` to `M-06` delivered mapping, I/O orchestration, and CLI integration.
- `M-07` to `M-09` completed fixture coverage, docs, and parity hardening.

Archive note:
- Epic M has no remaining active continuation tickets.

## Completion Detail

Milestone-level completion notes:
- `M-01` to `M-03` established the qemu-cmd importer contract and parser/model foundation.
- `M-04` to `M-06` delivered mapper, I/O pipeline, and CLI integration for usable import flow.
- `M-07` to `M-09` completed fixture coverage, docs, and parity-hardening assertions.

Archive note:
- Epic M has no remaining active continuation tickets.