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

### Building a package for a Linux distro ####

No packages have been released yet, so installation is pretty much a manual job for now.
That said, the Cargo.toml has been updated to work with some of the cargo packagers:

#### Building a package for debian based distributions ####

___Note:
This does depend on the necessary build tooling being installed in accordance
with debian package build guidelines, as well as a properly installed rust/cargo
toolchain (version 1.83) on the build host.___

To build a debian ezkvm package:

- create an empty build directory:
    - `mkdir build && cd build`
- fetch the build script and execute it:
    - `wget https://raw.githubusercontent.com/hurenkam/ezkvm/refs/heads/stable/debian/build.sh`
    - `sh build.sh`

The resulting `ezkvm_<version>_amd64.deb` package should work on debian
bookworm (12) and on ubuntu noble numbat (24.04). It will likely also work on
later versions, and possibly other debian based distro's but that has not been tested.

#### Building a package for arch based distributions ####

___Note:
This does depend on the necessary build tooling being installed in accordance
with arch package build guidelines, as well as a properly installed rust/cargo
toolchain (version 1.83) on the build host.___

To build an arch ezkvm package:

- create an empty build directory:
    - `mkdir build && cd build`
- fetch the build script and execute it:
    - `wget https://raw.githubusercontent.com/hurenkam/ezkvm/refs/heads/stable/arch/build.sh`
    - `sh build.sh`

You will find a `ezkvm-<version>-x86_64.tar.zst` package in the build directory.

#### Building a fedora/opensuse package ####

To build an rpm ezkvm package:

- install the cargo-rpm package using cargo: `cargo install cargo-rpm`
- create the package by running: `cargo rpm`

You will find an rpm package in the target/release/rpmbuild/RPM directory.
And a source rpm package in the target/release/rpmbuild/SRPM directory.

## Running ezkvm ##

The ezkvm application will look for config files in /etc/ezkvm, some example files can be
found in the repository, see above in the config section.

The application will also expect some files in /usr/share/ezkvm:

- pve-q35-4.0.cfg
- OVMF_CODE_4M.secboot.fd

These are files that i copied over from proxmox. In the future ezkvm should become
independent on these files, and either use the distro's defaults or a to be developed
custom ezkvm variant for these files.

Note that to allow gpu passthrough, or access to lvm volumes as disk backing, you need to
setup permissions correctly. The ezkvm application can be used in two ways:

1) #### using group permissions in combination with custom udev rules (recommended)

   create the ezkvm group:
   ```
   sudo groupadd ezkvm
   ```

   make the user from which you start ezkvm part of this group:
   ```
   sudo usermod -a -G ezkvm <your_user_name>
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

2) #### setup ezkvm with suid root permissions

   change ownership and permissions of the ezkvm binary:
   ```
   sudo chown root:root ./target/debug/ezkvm
   sudo chmod a+s ./target/debug/ezkvm
   ```
   now the qemu process will be started with root permissions
   so that all needed devices can be claimed.
   Note: If you use the 'gtk' ui option, the qemu process will still be started
   as the normal user, as the ui won't start when executed with root permissions.

3) #### setup networking

   There are many ways to setup networking, but what i use myself mostly, is
   bridge networking. How to set this up differs per distro, and depends also
   on the network tooling that are installed.

   On my laptop with EndeavourOS (arch based) i have NetworkManager
   installed. I've setup the bridge as follows using the nmcli tool:
   ```
   nmcli connection add type bridge ifname vmbr0 stp no
   nmcli connection up bridge-vmbr0
   nmcli connection modify bridge-vmbr0 ipv4.address <ip-address>
   nmcli connection modify bridge-vmbr0 ipv4.method manual
   nmcli connection up bridge-vmbr0
   ```

   The qemu-bridge-helper needs to have suid permissions, otherwise it
   will fail to extend the bridge interface when you start a vm:
   ```
   chmod +s /usr/lib/qemu/qemu-bridge-helper
   ```


4) #### OVMF files ###

   For now, ezkvm depends on OVMF files to reside in /usr/share/ezkvm, but they
   are not currently put there by the packages.
   You can however easily link from there to the files from your distro's ovmf package,
   or copy the ones from proxmox.
   Note that I've made available a custom edk2-ovmf package for arch, since i noticed
   that the recent builds don't support pvscsi, which is still used by several of
   my VM's.
   Beware that this may also be the case for your distro's package, so you might
   want to copy the proxmox files, or build it yourself.
   See my arch-edk2-ovmf repository for the custom arch build.

   ##### ezkvm expects the following files in /usr/share/ezkvm:
    - `OVMF_CODE.fd`: This should point to a legacy 2M OVMF EFI (if present on the system)
    - `OVMF_CODE_4M.fd`: This should point to the normal 4M OVMF EFI
    - `OVMF_CODE_4M.secboot.fd`: This should point to the 4M secure boot enabled OVMF EFI
