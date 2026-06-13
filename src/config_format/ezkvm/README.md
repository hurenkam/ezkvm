# Ezkvm Config Format

This module implements ezkvm YAML import/export at the config format boundary.

- Import path: ezkvm YAML -> `RuntimeConfig`
- Export path: `RuntimeConfig` -> ezkvm YAML

The importer and exporter both use the canonical runtime schema from `src/runtime_config/model.rs`.

## Files

- `importer.rs`: Parse and validate ezkvm YAML into `RuntimeConfig`.
- `exporter.rs`: Serialize `RuntimeConfig` back to YAML and write to disk.
- `diagnostics.rs`: Validation issue enrichment with source context.
- `mod.rs`: Module wiring, `EzkvmImporter`, `EzkvmExporter`, args re-exports.

## Ezkvm Runtime Schema

The ezkvm YAML schema is the canonical runtime schema represented by `RuntimeConfig`:

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
    version: null
  cpu: null
  memory:
    size: 8589934592
  devices: []
resources: []
```

### Top-level structure

- `metadata`
  - `schema_version: String`
  - `vm_name: String`
- `virtual_machine`
  - `machine: { family, chipset, version? }`
  - `cpu: Option<Cpu>`
  - `memory: Memory` (byte-based; currently uses `memory.size` in YAML)
  - `devices: Vec<Device>`
- `resources: Vec<Resource>`

### Resource variants

`resources` is a typed list:

- `Storage { storage: StorageResource }`
  - `File { path }`
  - `BlockDevice { path }`
- `Network { network: NetworkResource }`
  - `Tap { name }`
  - `Bridge { name }`
- `PciDevice { pci_device: PciDeviceResource }`
- `PcieDevice { pcie_device: PcieDeviceResource }`
- `UsbDevice { usb_device: UsbDeviceResource }`

### Device variants

`virtual_machine.devices` is a typed list:

- `Pcie { bus?, address?, device }`
- `Pci { bus?, address?, device }`
- `Usb { bus?, port?, device }`
- `Sata { bus?, address?, device }`
- `Ide { bus?, port?, device }`
- `Scsi { bus?, address?, device }`

## Importer Design

`EzkvmImporter::import` uses `ImportOptions::Ezkvm { host, vm }`.

Behavior:

1. Validate selected format is ezkvm (`InvalidFormat` otherwise).
2. Read VM YAML from `input.vm`.
3. Deserialize YAML into `RuntimeConfig` using `serde_yaml::from_str`.
4. Run runtime conformance validation via `validate_runtime_config(filename)`.
5. Enrich validation diagnostics (`diagnostics::enrich_validation_issues`) when needed.
6. Return validated `RuntimeConfig`.

Current validation highlights:

- Required non-empty strings:
  - `metadata.schema_version`
  - `metadata.vm_name`
  - `virtual_machine.machine.family`
  - `virtual_machine.machine.chipset`
- VM name must match filename stem.
- For `machine.family == "pc"`, chipset must be `q35` or `i440fx`.

## Exporter Design

`EzkvmExporter::export` uses `ExportOptions::Ezkvm { host, vm }`.

Behavior:

1. Validate selected format is ezkvm (`InvalidFormat` otherwise).
2. Resolve output path:
   - Use `output.vm` when provided.
   - Otherwise default to `<metadata.vm_name>.yaml`.
3. Serialize `RuntimeConfig` with `serde_yaml::to_string`.
4. Write YAML to destination path.
5. Return output path.

## Class Diagram (PlantUML)

```plantuml
@startuml
left to right direction
skinparam classAttributeIconSize 0

package "config_format::ezkvm" {
  class EzkvmImporter
  class EzkvmExporter
  class EzkvmInputArgs {
    + input_host: String
    + input_vm: String
  }
  class EzkvmOutputArgs {
    + output_host: String
    + output_vm: Option<String>
  }
}

package "config_format" {
  interface Importer {
    + import(args: ImportOptions): Result<RuntimeConfig, ImportError>
  }
  interface Exporter {
    + export(runtime: RuntimeConfig, args: ExportOptions): Result<PathBuf, ExportError>
  }
  enum ImportOptions
  enum ExportOptions
  class RuntimeConfig
}

package "runtime_config" {
  class ConformanceError
  class ParseError
}

Importer <|.. EzkvmImporter
Exporter <|.. EzkvmExporter
EzkvmImporter ..> ImportOptions
EzkvmExporter ..> ExportOptions
EzkvmImporter ..> RuntimeConfig : deserialize
EzkvmExporter ..> RuntimeConfig : serialize
EzkvmImporter ..> ParseError
EzkvmImporter ..> ConformanceError
@enduml
```

## Import Sequence (PlantUML)

```plantuml
@startuml
actor CLI
participant "ImportOptions::Ezkvm" as Opt
participant "EzkvmImporter" as Importer
participant "filesystem" as FS
participant "serde_yaml" as Serde
participant "RuntimeConfig::validate_runtime_config" as Validate
participant "diagnostics::enrich_validation_issues" as Diag

CLI -> Opt : import_runtime()
Opt -> Importer : import(ImportOptions::Ezkvm)
Importer -> FS : read_to_string(input.vm)
FS --> Importer : yaml text
Importer -> Serde : from_str<RuntimeConfig>(yaml)
Serde --> Importer : RuntimeConfig | ParseError
Importer -> Validate : validate_runtime_config(filename)
Validate --> Importer : Ok | ConformanceError::Validation
Importer -> Diag : enrich issues (on validation errors)
Diag --> Importer : enriched issues
Importer --> CLI : RuntimeConfig | ImportError::ImportFailed
@enduml
```

## Export Sequence (PlantUML)

```plantuml
@startuml
actor CLI
participant "ExportOptions::Ezkvm" as Opt
participant "EzkvmExporter" as Exporter
participant "serde_yaml" as Serde
participant "filesystem" as FS

CLI -> Opt : export_runtime(runtime)
Opt -> Exporter : export(runtime, ExportOptions::Ezkvm)
Exporter -> Exporter : resolve output path
Exporter -> Serde : to_string(runtime)
Serde --> Exporter : yaml text
Exporter -> FS : write(path, yaml)
FS --> Exporter : ok
Exporter --> CLI : PathBuf
@enduml
```

## Notes

- `input.host` and `output.host` are currently parsed but not yet used by ezkvm importer/exporter internals.
- Import and export are intentionally centered on canonical `RuntimeConfig` to keep format adapters thin and deterministic.

## Minimal Valid YAML

Use this as a baseline document that passes parsing and runtime conformance checks:

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "demo-vm"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: 8589934592
  devices: []
resources: []
```

If the file is named `demo-vm.yaml`, the importer validation path accepts this document.

## Common Valid YAML Example

This example is closer to a typical VM definition and includes:

- an explicit CPU model
- a SATA-attached device entry
- a bridge-backed network resource

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "workstation-01"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  cpu:
    model: "Host"
  memory:
    size: 17179869184
  devices:
    - { type: sata, driver: resource, name: "bootdisk" }
    - { type: network, driver: resource, name: "lan" }
resources:
  - { type: network, name: "lan", driver: bridge, bridge: "br0" }
  - { type: storage, name: "bootdisk", driver: raw, device: "/dev/vm0/vm-108-disk0" }
```

Notes:

- `devices` and `resources` are enum-backed and use an internally tagged representation with `type: ...`.
- The SATA device payload is currently structural (`device: {}`) in the schema and acts as a typed placement marker.
- The bridge resource represents host-side network attachment intent; actual runtime realization depends on downstream stages and host capabilities.

## Common Invalid YAML Example

This example contains several common mistakes:

```yaml
metadata:
  schema_version: ""
  vm_name: "other-name"
virtual_machine:
  machine:
    family: "pc"
    chipset: "arm-virt"
  memory:
    size: "8589934592"
  devices: []
resources: []
```

Assuming the source filename is `demo-vm.yaml`, expected validation/parsing outcomes include:

- `metadata.schema_version`: required non-empty string violation
- `metadata.vm_name`: filename stem mismatch (`demo-vm` vs `other-name`)
- `virtual_machine.machine.chipset`: invalid value for `family: pc` (must be `q35` or `i440fx`)
- Parse error for `virtual_machine.memory.size`: `expected usize` (string value instead of integer)

Example human-readable validation report shape:

```text
Validation Report: 3 issue(s)

ERRORS (3):
  - metadata.schema_version: is required and must be a non-empty string
  - metadata.vm_name: must match filename stem 'demo-vm'
  - virtual_machine.machine.chipset: must be one of [q35, i440fx] when machine family is 'pc'
```

For type mismatches such as `memory.size: "8589934592"`, parsing fails before conformance issue aggregation and returns a YAML parse error containing the field path.
