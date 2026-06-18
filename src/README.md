src/README.md

# EZKVM

## High level requirements
- Easily create a qemu based VM on any linux distro.
- Easily convert between ezkvm, proxmox, libvirt and qemu commandline syntax.

## Command syntax requirements

### Importing
1. Import ezkvm config file and run validation:
```ezkvm import --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --validate```

2. Import ezkvm config file and show runtime:
```ezkvm import --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --show-runtime```

3. Import ezkvm config file and export qemu commandline:
```ezkvm convert --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --output.type qemu```
This command imports the given ezkvm config file through the `config_format/ezkvm` importer and exports via the `config_format/qemu_cmd` exporter to ```<name>.qemu.cmd```.
Note that the vm input path is provided with ```--input.vm```, and host settings are provided with ```--input.host```.

4. Import proxmox config file, and save as ezkvm yaml file:
```ezkvm convert --input.type proxmox --input.storage /etc/pve/storage.cfg --input.vm <name>.conf --output.type ezkvm --output.host /etc/ezkvm/host.yaml --output.vm <name>.yaml```
This command imports the given proxmox config file through the `config_format/proxmox` importer and exports via the `config_format/ezkvm` exporter to ```<name>.yaml```.
Note that ```--input.storage``` is used to locate the proxmox ```storage.cfg``` file which explains how to translate storage paths in the proxmox vm config file to actual device locations.
Current import coverage includes machine/cpu/memory plus baseline UEFI firmware disk, TPM state, SCSI disks, and bridged network adapters where present in the source config.

5. Import ezkvm config and export as libvirt xml:
```ezkvm convert --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --output.type libvirt --output.vm <name>.xml```

6. Standalone export subcommand:
```ezkvm export --output.type proxmox --output.storage /etc/pve/storage.cfg --output.vm <name>.conf```
This subcommand is currently a placeholder and returns an error. Use `convert` for import+export flow.

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