# EZKVM #

## Introduction ##

This is my attempt at creating an alternative for libvirt/virt-manager that is simpler
to use, and uses a more friendly configuration file format.
I've been using Proxmox on my cloud sever, and much like the approach to the config files
that is taken there, where only minimum info is required to define what is specific for
your VM, and defaults are given by Proxmox depending on the chosen OS template.
Proxmox however is a very heavyweight tool, great for a server, but not very suitable
for use on a laptop, and it only runs on debian.
The aim for ezkvm is to not be limited to a single distro, and ideally it would run on
any system that can run qemu.

## How does it work ##

The idea is to use a small and simple config file, which will be expanded by the ezkvm
app into a call to qemu with the appropriate arguments. It is assumed that things like
network, firewall and dhcp daemons are already setup, and so ezkvm only needs to start
the qemu process.
By default it will setup a monitor pipe, so that you can connect to the qemu monitor to
do some advanced administration, either manually, or through a helper tool (I may extend
the tool in the future to make some of those administration tasks more user friendly).

## Config file syntax ##

The syntax is still much in flux, and meanwhile I would like to use [StrictYAML](https://hitchdev.com/strictyaml/)
however there does not seem to be a good parser crate for rust yet. So for now I'm using
the serde-yaml parser, but restrict the config file to not use advanced yaml features.

An example can be found in etc/wakazi.yaml, this is a config file that i derived from my
proxmox windows 11 gaming VM, and with the current ezkvm in combination with this config
file, i can run that VM just fine from my EndeavourOS Gemini installation.
Similarly, i have a gyndine.yaml config file defined, that uses virtio-gpu-gl as driver,
and runs openTumbleweed with KDE Plasma (no hw passthrough).

I do still need to execute `systemctl start libvirtd` and run `virsh net-start default`
to get the default bridge up. I'm still looking for a way to avoid the dependency on
libvirt, but for now I'll focus on maturing the parser and config file syntax.

To start the qemu VM, it should suffice to run `ezkvm <configfile>`.

## Building ezkvm ##

The ezkvm application is written in [rust](https://www.rust-lang.org/), and so you need
to install rustc and cargo to build it. See [here](https://www.rust-lang.org/tools/install)
on how to install rust on your OS. More info can be found on
the [Arch Linux wiki](https://wiki.archlinux.org/title/rust).

Once the toolchain is in place, simply run `cargo build` from the root of the
repo, and you should find the ezkvm executable in the `target/debug` directory after
a successful build.

Note: Although it is my intention for ezkvm to eventually work on any system that can run
qemu, my initial focus will be on Linux, and more specifically EndeavourOS.
Any diversity needed to run on other distro's or non-Linux platforms will be added once
that is mature.
That said, I am of course willing to review, and if found ok, to merge pull requests that
incorporate changes to make it run on other distro's or platforms.

## Running ezkvm ##

The ezkvm application will look for config files in /etc/ezkvm, some example files can be
found in the repository:

- wakiza.yaml: This is a config file for my windows 11 vm, which uses gpu passthrough & looking-glass
- gyndine.yaml: This is a config file for my linux (suse tumbleweed) vm, which uses virgl-gpu

The application will also expect some files in /usr/share/ezkvm:

- pve-q35-4.0.cfg
- OVMF_CODE.secboot.4m.fd

These are files that i copied over from proxmox. In the future ezkvm should become
independent on these files, and either use the distro's defaults or a to be developed
custom ezkvm variant for these files.

Note that to allow gpu passthrough, or access to lvm volumes as disk backing, you need to
setup permissions correctly. The ezkvm application can be used in two ways:

1) using group permissions in combination with custom udev rules (recommended)

   create the ezkvm group:
   ```
   sudo groupadd ezkvm
   ```

   make the user from which you start ezkvm part of this group:
   ```
   sudo useradd -G ezkvm <your_user_name>
   ```

   add the following items in your /etc/security/limits.conf:
   ```
   @ezkvm          hard    memlock         100000000
   @ezkvm          soft    memlock         100000000
   ```
   and add this in /etc/udev/rules.d/92_ezkvm.rules:
   ```
   SUBSYSTEM=="block", ENV{DM_LV_NAME}=="vm-*", MODE="660", GROUP="ezkvm"
   SUBSYSTEM=="vfio", MODE="660", GROUP="ezkvm"
   SUBSYSTEM=="vfio-dev", MODE="660", GROUP="ezkvm"
   ```

   After setting all this up, you should reboot for all changes to take effect.


2) setup ezkvm with suid root permissions

   change ownership and permissions of the ezkvm binary:
   ```
   sudo chown root:root ./target/debug/ezkvm
   sudo chmod a+s ./target/debug/ezkvm
   ```
   now the qemu process will be started with root permissions
   so that all needed devices can be claimed.
   Note: If you use the 'gtk' ui option, the qemu process will still be started
   as the normal user, as the ui won't start when executed with root permissions.

## Todo ##

- ~~Drop priviledges where appropriate; Currently some use cases require qemu to run with
  root priviledges (e.g. pci passthrough), some other use cases refuse to run with root
  priviledges (Gtk ui). Also swtpm & lg client don't need to run with root priviledges.~~
- ~~Add unit tests~~
- Refactor config files and move them into config directory as done in poc branch

  Todo:
    - network
    - storage
- Merge other improvements from the poc branch into the stable branch
- Run macos using ezkvm (and create an example config file for it)
- Check out proxmox OVMF patches so that a compatible OVMF can be provided through ezkvm
- Create installers for popular distro's:
    - Arch based distro's (since i develop on EndeavorOS)
    - Debian based distro's
    - Others by popular demand (please submit a feature request)
- Create examples using intel 440fx and seabios
- Add missing features by popular demand (please submit a feature request)
