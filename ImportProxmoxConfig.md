# Import Proxmox Config Plan

Date: 2026-04-08
Owner: ezkvm
Status: Proposed

## 1. Goal
Implement a new import function that reads existing Proxmox VM configuration files (for example `qm config` style), converts them into ezkvm YAML, and writes a valid ezkvm config file that can be reviewed and started with the existing CLI workflow.

## 2. Scope
In scope:
- Parse Proxmox VM config text (flat key-value syntax).
- Map supported Proxmox keys into ezkvm config sections.
- Emit deterministic ezkvm YAML.
- Produce an import report listing mapped, skipped, and passthrough fields.
- Add CLI entry point for import.

Out of scope (phase 1):
- Perfect 1:1 coverage of all Proxmox keys/devices.
- Importing Proxmox storage/network backend definitions from cluster-wide files.
- Host-side validation of every referenced device/file path.

## 3. Current Integration Points
- CLI and command dispatch: [src/args.rs](src/args.rs), [src/main.rs](src/main.rs)
- Config model and YAML shape: [src/vm/config.rs](src/vm/config.rs)
- System schema: [src/vm/config/system.rs](src/vm/config/system.rs)
- Storage/network submodels: [src/vm/config/storage.rs](src/vm/config/storage.rs), [src/vm/config/network.rs](src/vm/config/network.rs)
- User-facing config documentation: [doc/CONFIGURATION.md](doc/CONFIGURATION.md)

## 4. Proposed Design (Rust-first)
Use small composable modules (no heavyweight framework):

1. Proxmox parser module
- Input: raw config text.
- Output: normalized intermediate model (`ProxmoxVmConfig`).
- Responsibilities:
  - Parse `key: value` lines.
  - Split nested option lists (for example `cpu: host,flags=...`).
  - Parse numbered device keys (`scsi0`, `sata1`, `net0`, `hostpci0`, `usb0`).

2. Mapping module
- Input: `ProxmoxVmConfig`.
- Output: `EzkvmImportResult`:
  - `config`: ezkvm config struct or YAML-ready DTO.
  - `warnings`: lossy conversions, unsupported keys.
  - `extras`: explicit raw args needed for unsupported-but-preservable behavior.
- Responsibilities:
  - Deterministic mapping rules with safe defaults.
  - Preserve unrecognized-yet-important semantics as warnings and optional extras.

3. YAML writer module
- Serialize resulting model to YAML using existing serde behavior.
- Ensure stable field ordering where possible for readable diffs.

4. Import command orchestration
- Read input file.
- Parse -> map -> validate -> write output.
- Print summary with mapped counts, warnings, and output path.

## 5. Suggested Module Layout
- `src/import/mod.rs`
- `src/import/proxmox_parser.rs`
- `src/import/proxmox_model.rs`
- `src/import/mapper.rs`
- `src/import/report.rs`
- `src/import/io.rs`

Minimal entrypoint surface:
- Add `ImportProxmoxConfig` variant to CLI command enum.
- Dispatch in [src/main.rs](src/main.rs) to an import service function.

## 6. Mapping Strategy (Phase 1)

### 6.1 Top-level identity and basics
- `name` -> `general.name`
- `uuid` / `smbios1` uuid -> `general.uuid` (if present)
- `memory` (MiB) -> `system.memory.max`
- `cores` + `sockets` -> `system.cpu.cores` + `system.cpu.sockets`
- `cpu` -> `system.cpu.model` and inferred flags when parseable

### 6.2 Firmware and machine
- `bios: ovmf` -> `system.bios.type: ovmf`
- `bios: seabios` -> `system.bios.type: seabios`
- `machine: q35` -> `system.chipset.type: q35`
- Proxmox-specific machine details not modeled in ezkvm -> warning, optional extras

### 6.3 TPM
- Proxmox `tpmstate0` -> map to `system.tpm`:
  - Prefer `swtpm` when backing state/socket semantics are representable.
  - Otherwise warn and preserve details in report/extras.

### 6.4 Disks/controllers
- Keys like `scsi0`, `sata0`, `ide0` -> ezkvm `storage` controllers + drives
- Infer drive type (`hd` vs `cd`) from media/options where possible
- Map common options:
  - `discard=on` -> drive discard
  - cache mode -> drive cache
  - format hints when known
  - boot order hints -> `boot_index` when derivable
- If controller semantics do not map exactly, keep best-fit and warn

### 6.5 Network
- `netN` parse model/bridge/mac/firewall/tag/etc.
- Phase 1 target:
  - bridge-backed adapters -> ezkvm `network` type `bridge`
  - set MAC via footer field
  - map model to best available driver or warn if degraded
- Unsupported network modes (NAT, SDN details, rate limits, advanced multiqueue) -> warning/extras

### 6.6 Host passthrough
- `hostpciN` -> `host.pci[]` entries when identifiers are parseable
- `usbN` -> `host.usb[]` where bus/port data can be extracted
- Keep unresolved passthrough options in report

### 6.7 Display/GPU/remote access
- `vga` -> ezkvm `gpu` best-fit mapping (`virtio`, `vmware-svga`, or warning)
- `spice` and `vnc` settings -> map when shape is compatible
- Fallback: use `display.type: no_display` with warning if endpoint cannot be represented safely

### 6.8 Unknown and unsupported keys
- Maintain explicit allowlist/blocklist behavior:
  - Known + mapped
  - Known + not yet supported
  - Unknown
- Emit all non-mapped items in import report.
- Optionally append safe raw args into `extras` only when conversion is deterministic.

## 7. CLI and UX Proposal
Add command flags in [src/args.rs](src/args.rs):
- `--import-proxmox <file>`: path to Proxmox VM config input.
- `--output <file>`: output ezkvm yaml path (default: `<name>.yaml` in cwd).
- `--import-name <name>`: override generated VM name.
- `--strict`: fail on unsupported keys instead of producing warnings.
- `--dry-run`: print YAML + report without writing file.

Main dispatch in [src/main.rs](src/main.rs):
- `EzkvmCommand::ImportProxmoxConfig { ... }` -> call import service.

Output should include:
- Number of mapped keys
- Number of warnings
- List of unsupported keys
- Output path

## 8. Validation and Safety Rules
Validation before writing:
- Ensure generated YAML deserializes back into the current config model.
- Ensure required minimum fields exist (`general`, `system` defaults acceptable).
- Ensure no duplicate storage/network IDs in generated structure.

Safety defaults:
- Prefer conservative behavior over feature parity.
- If uncertain mapping could break boot/device correctness, emit warning and omit field.
- Do not silently drop high-impact fields (boot, disk, passthrough, display) without report entry.

## 9. Test Plan

### 9.1 Unit tests: parser
- Parse primitive key-value lines.
- Parse numbered keys (`scsi0`, `net1`, `hostpci2`).
- Parse comma-separated option lists with quoted values.
- Handle malformed lines with useful errors.

### 9.2 Unit tests: mapping
- CPU/memory/firmware mapping cases.
- Disk mappings for scsi/sata/ide variants.
- Bridge network mapping with MAC preservation.
- Host PCI/USB passthrough mapping.
- Unknown-key warning and strict mode behavior.

### 9.3 Round-trip tests
- Proxmox sample input -> generated YAML -> deserialize via current config model.
- Assert key behavioral fields are preserved.

### 9.4 Golden tests
- Keep fixture inputs under `tests/fixtures/proxmox/`.
- Compare generated YAML against expected outputs under `tests/fixtures/ezkvm/`.

## 10. Implementation Phases

Phase 1: foundation
- Add parser + intermediate model.
- Add mapping for general/system/storage/network basics.
- Add report generation.

Phase 2: CLI integration
- Add command-line options and command dispatch.
- Add dry-run/strict/output behavior.
- Write generated YAML.

Phase 3: coverage expansion
- Add passthrough and remote display mappings.
- Improve option fidelity and extras mapping.
- Expand fixture corpus from real-world Proxmox configs.

Phase 4: docs and examples
- Document import command in README and user manual.
- Add migration guide from Proxmox config to ezkvm.

## 11. Risks and Mitigations
- Risk: Proxmox syntax variability across versions.
  - Mitigation: intermediate model + fixture corpus from multiple samples.
- Risk: lossy conversion for unsupported keys.
  - Mitigation: explicit warnings, strict mode, and optional extras.
- Risk: generated config starts but differs behaviorally.
  - Mitigation: report high-impact conversions and add integration fixtures.

## 12. Success Criteria
- User can run one command to convert a Proxmox VM config file into ezkvm YAML.
- Generated YAML deserializes with current ezkvm config model.
- Import report clearly shows what was mapped, degraded, or skipped.
- At least 10 representative Proxmox fixture configs pass parser and mapping tests.
