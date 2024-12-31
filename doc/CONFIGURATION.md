# Configuration

The configuration files are kept in `/etc/ezkvm` and the syntax is yaml based.

There are currently 10 sections defined in the config file.

1) `general`

   This section contains generic info about the vm.

   Defined items in this section:
    - `name`: A user defined name for this VM
    - `uuid`: A (usually generated) unique uuid for this VM

2) `system`

   This section defines the main characteristics of the VM, the
   architecture, the emulated chipset and cpu, and amount of ram.

   Defined items in this section:

    - `chipset`: Can be Q35 or i440fx.
    - `bios`: Can be SeaBios or OVMF (UEFI)
    - `memory`: Amount of memory assigned (in Mib)
    - `cpu`: Cpu type and amount of sockets/cores/threads
    - `tpm`: Can be None or Swtpm. Passthrough is planned.
    - `applesmc`: Here you can provide the osk key for macOS.

3) `gpu`

   This item defines the GPU that the VM is to use, not all qemu
   options are currently available, but there is support for virtio,
   vmware-svga, and GPU Passthrough.
   Not every GPU works with every Display though.

   Defined items in this section:

    - `no_gpu`: For headless systems, no gpu defined.
    - `passthrough`: Passthrough an existing GPU to the VM, a list of pci id's can be given.
    - `virtio`: Virtual GPU with GL support, best option if supported by VM OS
    - `vmware_svga`: VMWare SVGA device, requires VMWare driver in VM OS for best performance.

4) `display`

   This defines how the screen is viewed, a number of methods are supported,
   some of them accellerated (provided a recent remote-viewer and qemu are present
   on the host system)

   Defined items in this section:

    - `gtk`: This uses the qemu built-in gtk ui to display the guest
      OS screen.
    - `looking_glass`: This uses the Looking Glass client to view the screen of a
      passed through GPU. Does require the Looking Glass host applicatoin to be installed
      in the guest VM.
    - `remote_viewer`: This is the virt-viewer from the virt-manager package, if installed
      on the host it can be used to connect to a vnc or spice socket or port and view the
      guest VM. If using a local socket, then OpenGL acceleration is supported.
    - `no_display`: If you don't need to see the screen.

5) `spice`

   Defined items in this section:

    - `socket`: Socket or port where the spice display is exposed.
    - `gl`: Enable/Disable OpenGL support

6) `vnc`

   Defined items in this section:

    - `socket`: Socket or port where vnc display is exposed.

7) `host`

   Defined items in this section:

    - `usb`: List of passed through usb devices
    - `pci`: List of passed through pci(e) devices

8) `storage`

   This consists of a list of controllers, where each controller can have a list of
   storage devices (currently supports cd/hd/ssd).

   Defined controllers:

    - `pvscsi`: This uses the vmware pvscsi controller to emulate a scsi bus. Note that
      OVMF has recently dropped support for this, so if you wish to use it you must either
      install the ezkvm_ovmf package, a proxmos_ovmf package, or built your own.
    - `sata`: This enables a simulated sata bus on the host.
    - `ide`: This uses a simulated ide bus.
    - `virtio`: This does not really simulate a bus, but instantiates a `virtio-pci-blk`
      instance for each listed device.

9) `network`

   Here network devices can be listed, this has not had much testing though, only
   tested configuration is the network bridge. More to follow in later releases.

10) `extra`

    Because the predefined sections hardly cover everything, in this option any
    raw qemu arguments can be listed that are to be appended at the end.

Example configuration file:
(more examples can be found [here](etc))

```
general:
  name: ubuntu-oriole-desktop

system:
  bios: { type: "ovmf", uuid: "c0e240a5-859a-4378-a2d9-95088f531142", file: "/dev/vm1/ubuntu-oriole-desktop-efidisk" }
  cpu: { cores: 8 }

gpu:
  type: "virtio"
  gl: yes
  pcie: { }

display:
  type: "gtk"

storage:
  - controller: pvscsi
    drives:
      - { type: hd, file: "/dev/vm1/ubuntu-oriole-desktop-boot", discard: "on", boot_index: 0 }

network:
  - { type: "bridge", mac: "BC:24:11:FF:76:89" }

```