# Current Implementation Architecture

This page is the index for implementation design notes that describe the current code shape under `src/`.

## ⚠️ OUTDATED: Architecture Refactored

The implementation has been refactored from a legacy importer/exporter pattern to **bidirectional RuntimeBuilder/SchemaBuilder stages**. Much documentation on this index is **outdated**. For current information, see:

- [src/README.md: Config Format Architecture](../../../src/README.md#config-format-architecture) — current overview
- [src/config_format/ezkvm/README.md](../../../src/config_format/ezkvm/README.md) — ezkvm implementation details

## Scope (Updated)

The current implementation is organized as a bidirectional stage-based pipeline:

- **entrypoint and crate wiring** in `src/main.rs` and `src/lib.rs`
- **bidirectional format adapters** in `src/config_format/<format>/stages/`
- **canonical runtime model** in `src/runtime_model/`
- **deterministic output rendering** to CLI arguments, files, or other formats

**New Bidirectional Flow** (replaces old unidirectional import):

1. Schema text → **RuntimeBuilder** → RuntimeModel (format-specific)
2. RuntimeModel → **SchemaBuilder** → Schema output (format-specific)
3. Supports lossless round-trip conversion between formats
4. ResourceIndex pattern: memoized resource ID synthesis (storage0, net0, hostpci0)
5. BusRegister pattern: device registration across bus topologies

## Module Notes

- [Entrypoint And Wiring](./entrypoint-and-wiring.md)
- [Import Stage](./import-stage.md)
- [VM Spec, Parsing, And Validation](./vm-spec-parsing-validation.md)
- [Runtime Resolution And Render Stage](./runtime-resolution-and-render-stage.md)

## Module Map

| Module | Current responsibility | Key types / functions |
| --- | --- | --- |
| `src/lib.rs` | Crate wiring | `app_name()` |
| `src/main.rs` | CLI entrypoint | `main()` |
| `src/config_format/mod.rs` | Import/export stage boundary | `Importer`, `Exporter`, `ImportOptions`, `ExportOptions` |
| `src/config_format/ezkvm/mod.rs` | ezkvm staged adapter | `EzkvmImporter`, `EzkvmExporter` |
| `src/config_format/proxmox/mod.rs` | Proxmox staged adapter | `ProxmoxImporter`, `ProxmoxExporter` |
| `src/config_format/qemu_cmd/mod.rs` | QEMU command staged adapter | `QemuImporter`, `QemuExporter`, `QemuParser`, `QemuRuntimeBuilder`, `QemuSchemaBuilder`, `QemuMarshaler` |
| `src/config_format/libvirt/mod.rs` | Libvirt adapter boundary | `LibvirtImporter`, `LibvirtExporter` |
| `src/runtime_config/mod.rs` | Canonical schema exports | `CanonicalDocument`, `ParseError`, `ValidationIssue`, `ConformanceError`, `validate_canonical_yaml()`, `validate_canonical_document()` |
| `src/runtime_config/model.rs` | Canonical data model | `Metadata`, `VirtualMachine`, `System`, `Machine`, `Cpu`, `Memory`, `StorageEntry`, `NetworkEntry`, `ResourceRef` |
| `src/runtime_config/parsing.rs` | YAML parsing and structural validation | `Severity`, `ValidationIssue`, `ParseError`, `parse_canonical_document_from_yaml()`, `parse_canonical_document()`, `enrich_validation_issues()` |
| `src/runtime_config/validation.rs` | Semantic validation and report formatting | `ValidationSummary`, `ValidationReport`, `ReportFormatter`, `DefaultReportFormatter`, `ConformanceError` |
| `src/runtime_resolution/mod.rs` | Runtime boundary scaffold | `RuntimeResolutionStage`, `EffectiveRuntimeModel` |
| `src/render_stage/mod.rs` | Render stage boundary | `RenderRequest`, `RenderStage` |
| `src/render_stage/deterministic.rs` | Deterministic renderer | `DeterministicRenderStage` |

## Current Constraints

- The binary does not yet invoke import, validation, runtime resolution, or rendering.
- The runtime-resolution module is a placeholder boundary, not a working resolver.
- The Proxmox adapter covers a small, deterministic subset of `.conf` keys only.
- The canonical YAML path is the only path that currently runs full parse-plus-validation behavior in one call.

## Related Architecture Docs

- [Model Separation Pipeline Contract](../model-separation-pipeline.md)
- [Import Adapters Contract](../import-adapters.md)
- [Validation Reporting](../validation-reporting.md)
- [Validation Examples](../validation-examples.md)