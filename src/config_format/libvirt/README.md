# Libvirt Config Format

This module provides the libvirt adapter boundary for the config format pipeline.

Current implementation status:

- input argument model is defined
- output argument model is defined
- loader and saver structs are present
- actual runtime mapping is not implemented yet

## Current File Layout

- `mod.rs`
  - module wiring and public re-exports
- `loader.rs`
  - `LibvirtInputArgs` and `LibvirtLoader`
- `saver.rs`
  - `LibvirtOutputArgs` and `LibvirtSaver`

## Loader

`LibvirtLoader` implements `RuntimeModelLoader` with:

- `Args = LibvirtInputArgs`
- `Error = ImportError`

`LibvirtInputArgs` currently expects:

- `input.vm` -> path to the libvirt XML file

Current behavior:

- `load(...)` always returns:
  - `ImportError::UnsupportedImporter("libvirt")`

## Saver

`LibvirtSaver` implements `RuntimeModelSaver` with:

- `Args = LibvirtOutputArgs`
- `Error = ExportError`

`LibvirtOutputArgs` currently expects:

- `output.vm` -> destination file path

Current behavior:

- `save(...)` currently returns:
  - `ExportError::UnsupportedExporter(...)`
- error includes requested destination path in the message

## Class Diagram (PlantUML)

```plantuml
@startuml
left to right direction
skinparam classAttributeIconSize 0

package "config_format" {
  interface RuntimeModelLoader {
    +load(args) -> Result<RuntimeModel, ImportError>
  }
  interface RuntimeModelSaver {
    +save(runtime, args) -> Result<(), ExportError>
  }
  class ImportError
  class ExportError
}

package "config_format::libvirt" {
  class LibvirtInputArgs {
    +input_vm: String
  }
  class LibvirtOutputArgs {
    +output_vm: String
  }
  class LibvirtLoader
  class LibvirtSaver
}

RuntimeModelLoader <|.. LibvirtLoader
RuntimeModelSaver <|.. LibvirtSaver

LibvirtLoader ..> LibvirtInputArgs
LibvirtLoader ..> ImportError
LibvirtSaver ..> LibvirtOutputArgs
LibvirtSaver ..> ExportError
@enduml
```

## Load Sequence (PlantUML)

```plantuml
@startuml
actor Caller
participant "LibvirtLoader" as Loader
participant "ImportError" as Err

Caller -> Loader : load(LibvirtInputArgs)
Loader -> Loader : not implemented
Loader -> Err : UnsupportedImporter("libvirt")
Loader --> Caller : Err(ImportError)
@enduml
```

## Save Sequence (PlantUML)

```plantuml
@startuml
actor Caller
participant "LibvirtSaver" as Saver
participant "ExportError" as Err

Caller -> Saver : save(runtime, LibvirtOutputArgs)
Saver -> Saver : compute output path
Saver -> Saver : not implemented
Saver -> Err : UnsupportedExporter("libvirt export to ...")
Saver --> Caller : Err(ExportError)
@enduml
```

## Implementation Notes

When libvirt mapping work starts, this module should grow with the same staged shape used elsewhere:

- parser (XML -> schema)
- runtime builder (schema -> runtime)
- schema builder (runtime -> schema)
- marshaler (schema -> XML)

Until then, callers should treat libvirt mode as explicitly unsupported at runtime.
