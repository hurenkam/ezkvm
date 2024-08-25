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
on how to install rust on your OS. More info can be found on the [Arch Linux wiki](https://wiki.archlinux.org/title/rust).

Once the toolchain is in place, simply run `cargo build` from the root of the
repo, and you should find the ezkvm executable in the `target/debug` directory after
a successful build.

Note: Although it is my intention for ezkvm to eventually work on any system that can run
qemu, my initial focus will be on Linux, and more specifically EndeavourOS.
Any diversity needed to run on other distro's or non-Linux platforms will be added once
that is mature. 
That said, I am of course willing to review, and if found ok, to merge pull requests that
incorporate changes to make it run on other distro's or platforms.


## Todo ##
 - Look into QCFG, things may become a lot easier if i use that as an (intermediate) format:
https://wiki.qemu.org/Features/QCFG

 - Drop priviledges where appropriate; Currently some use cases require qemu to run with
   root priviledges (e.g. pci passthrough), some other use cases refuse to run with root
   priviledges (Gtk ui). Also swtpm & lg client don't need to run with root priviledges.



