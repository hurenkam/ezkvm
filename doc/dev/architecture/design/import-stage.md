# Import Stage

Back to index: [Current Implementation Architecture](./current-implementation-architecture.md)

## Scope

This note covers source import adapters in:

- `src/import_stage/mod.rs`
- `src/import_stage/canonical_yaml.rs`
- `src/import_stage/proxmox_conf.rs`

The import stage takes source text plus source name and returns a canonical `CanonicalDocument` or a typed adapter error.

## Stage Structure

```plantuml
@startuml
skinparam classAttributeIconSize 0

class ImportRequest {
  +source_text: &str
  +source_name: &Path
}

interface ImportStage {
  +import(request: ImportRequest) -> Result<CanonicalDocument, Error>
}

class ImportStageError {
}

class CanonicalYamlImportStage {
  +import(request) -> Result<CanonicalDocument, ImportStageError>
}

class ProxmoxConfImportStage {
  +parse(request) -> Result<CanonicalDocument, ProxmoxConfImportError>
}

class ProxmoxConfImportError {
}

class CanonicalDocument

ImportStage <|.. CanonicalYamlImportStage
ImportStage <|.. ProxmoxConfImportStage
ImportStageError --> ProxmoxConfImportError : wraps
ImportStageError --> ConformanceError : wraps
CanonicalYamlImportStage ..> validate_canonical_yaml
ProxmoxConfImportStage ..> parse_machine_value
ProxmoxConfImportStage ..> CanonicalDocument
ImportRequest --> Path
@enduml
```

## Current Adapter Coverage

`CanonicalYamlImportStage` delegates to `validate_canonical_yaml()`.

`ProxmoxConfImportStage` currently maps these keys:

- `name` -> canonical VM name
- `machine` -> machine family and chipset
- `cpu` -> CPU model
- `memory` -> memory minimum
- `scsi*`, `net*`, `hostpci*`, `usb*` -> canonical ID lists

Malformed lines, unsupported machine values, missing required fields, and invalid memory values are rejected.

## Proxmox Parse Flow

```plantuml
@startuml
actor Caller
participant "ProxmoxConfImportStage" as Stage
participant "parse_machine_value()" as MachineParser

Caller -> Stage : parse(ImportRequest)
loop each non-empty non-comment line
  Stage -> Stage : split key:value
  alt key = machine
    Stage -> MachineParser : parse_machine_value(value)
    MachineParser --> Stage : Machine
  else key = name/cpu/memory
    Stage -> Stage : store scalar field
  else key matches scsi*/net*/hostpci*/usb*
    Stage -> Stage : append ID entry
  else unsupported key
    Stage -> Stage : ignore
  end
end
Stage --> Caller : CanonicalDocument or ProxmoxConfImportError
@enduml
```

## Related Docs

- [Import Adapters Contract](../import-adapters.md)
- [VM Spec, Parsing, And Validation](./vm-spec-parsing-validation.md)
