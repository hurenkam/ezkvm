src/config_importer/README.md

# Config Importer

## Requirements
This document describes how the config importer stage is modeled.

First some requirements:
1. Ezkvm needs to support multiple import formats, for now they consist of the following:
	1. ezkvm virtual machine config files (yaml)
	2. proxmox config files
	3. qemu command line
	4. libvirt config files
 2. Each importer should return a generic ezkvm machine runtime model
 3. Each importer is passed relevant command line args (any argument passed using the ```--import:type=<importer>[,<args>]``` flag. Where ```<args>``` is a comma seperated list of arguments to be passed to the importer.
 4. The importers shall each implement the generic ConfigImporter trait, that defines how arguments are passed to the importer, and how the resulting runtime is returned. This trait shall be generic so that it does not depend on the actual implementation of the importer, this to allow easy extensibility.
 5. Each implementation shall be in a separate subdirectory, the current directory shall only be used for generic types and traits that are shared by more than one implementation.

## Command line arguments

Import command line arguments will typically look like this:

```
--input:type=proxmox,config=<name>.conf,storage=/etc/pve/storage.cfg
--input:type=ezkvm,config=<name>.yaml,host=/etc/ezkvm/host.yaml,profiles=/etc/ezkvm/profiles.d
```

## Design

### Importer trait

```
pub struct ConfigArgs {
	pub args: Vec<String>
}

pub trait ConfigImporter {
	type ConfigError;

	fn import_config(
		&self,
		config_args: ConfigArgs
	) -> Result<RuntimeConfig, Self::ConfigError>;
}
```

### Class Diagram

```plantuml
@startuml
interface ConfigImporter << (T,#FFB347) >> {
	+import_config(&self, config_args: ConfigArgs) -> Result<RuntimeConfig, ConfigError>
}

class EzkvmConfigImporter << (S,#98FB98) >>
class ProxmoxConfigImporter << (S,#98FB98) >>
class QemuConfigImporter << (S,#98FB98) >>
class LibvirtConfigImporter << (S,#98FB98) >>

ConfigImporter <|-- EzkvmConfigImporter
ConfigImporter <|-- ProxmoxConfigImporter
ConfigImporter <|-- QemuConfigImporter
ConfigImporter <|-- LibvirtConfigImporter
@enduml
```
