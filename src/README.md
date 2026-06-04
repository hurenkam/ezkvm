src/README.md

# EZKVM

## High level requirements
- Easily create a qemu based VM on any linux distro.
- Easily convert between ezkvm, proxmox, libvirt and qemu commandline syntax.

## Command syntax requirements

### Importing
1. Import ezkvm config file and run validation:
```ezkvm import --input-type ezkvm --input-host /etc/ezkvm/host.yaml --input-vm <name>.yaml --validate```

2. Import ezkvm config file and show runtime:
```ezkvm import --input-type ezkvm --input-host /etc/ezkvm/host.yaml --input-vm <name>.yaml --show-runtime```

3. Import ezkvm config file and export qemu commandline:
```ezkvm convert --input-type ezkvm --input-host /etc/ezkvm/host.yaml --input-vm <name>.yaml --output-type qemu```
This command shall import the given ezkvm config file using the ezkvm importer, and output the results through ezkvm config_exporter to ```<name>.qemu.cmd```.
Note that the vm input path is provided with ```--input-vm```, and host settings are provided with ```--input-host```.

4. Import proxmox config file, and save as ezkvm yaml file:
```ezkvm convert --input-type proxmox --input-storage /etc/pve/storage.cfg --input-vm <name>.conf --output-type ezkvm --output-host /etc/ezkvm/host.yaml --output-vm <name>.yaml```
This command shall result import the given (```config=```) proxmox config file using the proxmox importer, and output the results through ezkvm config_exporter to ```<name>.yaml```.
Note that ```--input-storage``` is used to locate the proxmox ```storage.cfg``` file which explains how to translate storage paths in the proxmox vm config file to actual device locations.

### Runtime operations
1. Show runtime:
```ezkvm show-runtime --name <name>```

2. Start vm:
```ezkvm start --name <name>```

3. Stop vm:
```ezkvm stop --name <name>```

4. Reset vm:
```ezkvm reset --name <name>```

5. Shutdown vm:
```ezkvm shutdown --name <name>```

## Design

```plantuml
@startuml

@enduml
```

```
```