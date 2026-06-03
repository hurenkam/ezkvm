src/README.md

# EZKVM

## High level requirements
- Easily create a qemu based VM on any linux distro.
- Easily convert between ezkvm, proxmox, libvirt and qemu commandline syntax.

## Command syntax requirements

### Importing
1. Import ezkvm config file and run validation:
```ezkvm --input:type=ezkvm,config=<name>.yaml,host=/etc/ezkvm/host.yaml,profiles=/etc/ezkvm/profiles.d --validate```

2. Import ezkvm config file and show runtime:
```ezkvm --input:type=ezkvm,config=<name>.yaml,host=/etc/ezkvm/host.yaml,profiles=/etc/ezkvm/profiles.d --show-runtime```

3. Import ezkvm config file and export qemu commandline:
```ezkvm --input:type=ezkvm,config=<name>.yaml,host=/etc/ezkvm/host.yaml,profiles=/etc/ezkvm/profiles.d --output:type=qemu```
This command shall import the given ezkvm config file using the ezkvm importer, and output the results through ezkvm config_exporter to ```<name>.qemu.cmd```.
Note that the ```config=``` argument is used to locate the config file to be imported, the ```host=``` argument is used to locate the host settings to use for the import, and the ```profiles=``` argument is used to locate the profiles directory to be used for parsing the input.

4. Import proxmox config file, and save as ezkvm yaml file:
```ezkvm --import:type=proxmox,config=<name>.conf,storage=/etc/pve/storage.cfg --output:type=ezkvm```
This command shall result import the given (```config=```) proxmox config file using the proxmox importer, and output the results through ezkvm config_exporter to ```<name>.yaml```.
Note that the ```storage=``` argument is used to locate the proxmox ```storage.cfg``` file which explains how to translate the storage paths in the proxmox vm config file to actual device locations.

## Design

```plantuml
@startuml

@enduml
```

```
```