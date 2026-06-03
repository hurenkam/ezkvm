# Canonical YAML Validation and Schema Conformance Matrix

Status: Done
Date: 2026-05-29
Started: 2026-05-29
Completed: 2026-05-29

## Purpose

Establish comprehensive validation for the canonical YAML contract with an executable conformance matrix backed by real-world corpus examples. Ensure validation covers schema shape, required fields, consistency rules, and deterministic diagnostic reporting before expanding to host-resource boundary modeling.

## Requirement IDs

- FR-003 Canonical YAML Core Config
- FR-004 Model Separation Pipeline
- FR-006 Validation Before Execution
- NFR-001 Usability

## Final Status

**Core validation implemented:** ✅
- YAML structure parsing with field-path aware error reporting
- Required field validation (metadata, virtual_machine, system, machine, cpu, memory)
- Machine consistency rules (family=pc with chipset=[q35, i440fx])
- Resource ID uniqueness validation across storage, network, resources
- vm_name filename matching validation
- Contract edge-case coverage for optional sections, invalid field types, and malformed YAML

**Validation reporting implemented:** ✅
- Severity-aware issue model with remediation hints
- Human-readable and JSON report formatting
- Best-effort line-number and nearby-context enrichment from YAML field paths
- Stable summary/count helpers for downstream callers

**Corpus-backed conformance coverage implemented:** ✅
- Scalable corpus expectations for three Proxmox `.conf` samples across three hosts
- Explicit regression coverage for canonical-output filename validation versus numeric `.conf` source paths

**Documentation synchronized:** ✅
- Validation rules, examples, reporting behavior, and coding guidance aligned with implemented behavior

## Completed Work

### Item 1: Formalize CT-001..005 Test Naming Convention
**Assigned To:** Vasquez (Tester)

**Completion Summary:**
- Added CT requirement comments to all 9 tests in `src/runtime_config/validation.rs`
- Created contract specification document: `doc/dev/requirements/canonical-yaml-schema-contract.md`
- Established test-to-requirement mapping for CT-001 through CT-005

### Item 2: Add Contract Coverage for Edge Cases
**Assigned To:** Vasquez (Tester)

**Completion Summary:**
- Confirmed missing optional sections remain valid
- Added container-type mismatch coverage with precise field-path assertions
- Added malformed YAML regression coverage at the parse boundary
- Kept oversized value validation explicitly deferred by contract design

### Item 3: Validation Report Enhancement
**Assigned To:** Hicks (Backend Dev)

**Completion Summary:**
- Added best-effort YAML path resolution so parse-layer and conformance-layer issues carry source line numbers and nearby context snippets
- Added typed `ValidationReport` and `ValidationSummary` helpers while keeping human-readable and JSON formatter output stable for callers
- Filled remediation guidance for missing required parse-layer fields and expanded regression coverage for contextual issue enrichment and summary counts

### Item 4: Integrate Corpus-Based Test Suite
**Assigned To:** Bishop (Systems Adapter Dev)

**Completion Summary:**
- Added table-driven corpus coverage in `src/config_importer/proxmox/mod.rs` for `felucia/108.conf`, `coruscant/3101.conf`, and `zbp-server-mh2/103.conf`
- Verified each imported document conforms when validated against its canonical output filename (`wakiza.yaml`, `gyndine.yaml`, `desktop-markh-3.yaml`)
- Added an explicit regression proving the current numeric-source-name mismatch is expected when validation runs against original `.conf` source paths

### Item 5: Documentation Sync
**Assigned To:** Newt (Docs and DevRel)

**Completion Summary:**
- Corrected validation docs to distinguish YAML parse failures from aggregated field-level issues
- Documented the implemented formatter baseline, including severity/remediation support and line/snippet enrichment behavior
- Added operator-facing examples for malformed YAML and filename-stem warning cases
- Kept the corpus matrix and import-boundary notes aligned as Item 4 landed

## Dependencies

- [canonical-yaml-schema-contract](../../../dev/requirements/canonical-yaml-schema-contract.md)
- [coding-guidelines](../../../dev/architecture/coding-guidelines.md)
- [input/felucia/108.conf](../../../../input/felucia/108.conf)
- [input/coruscant/3101.conf](../../../../input/coruscant/3101.conf)
- [input/coruscant/3101.qemu.cmd.split](../../../../input/coruscant/3101.qemu.cmd.split)
- [input/felucia/108.qemu.cmd.split](../../../../input/felucia/108.qemu.cmd.split)

## Test And Evidence

**Corpus Baseline:**

| Corpus file | Expected canonical outcome | Validation expectation |
| --- | --- | --- |
| `input/felucia/108.conf` | `vm_name=wakiza`, `chipset=q35`, storage=`scsi0`,`scsi1`, network=`net0`, resources=`hostpci0`,`usb0` | Passes conformance when validated as `wakiza.yaml`; expected filename mismatch when validated as `108.conf` |
| `input/coruscant/3101.conf` | `vm_name=gyndine`, `chipset=q35`, storage=`scsi0`, no network entries, resources=`hostpci0`..`hostpci11` | Passes conformance when validated as `gyndine.yaml`; expected filename mismatch when validated as `3101.conf` |
| `input/zbp-server-mh2/103.conf` | `vm_name=desktop-markh-3`, `chipset=q35`, storage=`scsi0`,`scsi1`, network=`net0`, no passthrough resources | Passes conformance when validated as `desktop-markh-3.yaml`; expected filename mismatch when validated as `103.conf` |

**Representative Test Cases:**
- CT-001: Valid canonical example
- CT-002: Missing required field detection
- CT-003: Duplicate ID detection
- CT-004: Machine consistency validation
- CT-005: Type validation with field paths
- Edge case: Optional sections omitted
- Edge case: Container type mismatch path reporting
- Edge case: Malformed YAML rejection
- Corpus baseline preservation and canonical-path conformance in `src/config_importer/proxmox/mod.rs`

## Outcome

This feature is complete and serves as the implemented contract-validation gate for canonical YAML input. Follow-on planning should treat it as completed prerequisite work for host-resource boundary modeling and any future schema expansion.
