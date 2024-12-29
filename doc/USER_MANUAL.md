# How does it work ##

The idea is to use a small and simple config file, which will be expanded by the ezkvm
app into a call to qemu with the appropriate arguments. It is assumed that things like
network, firewall and dhcp daemons are already setup, and so ezkvm only needs to start
the qemu process.
By default it will setup a monitor pipe, so that you can connect to the qemu monitor to
do some advanced administration, either manually, or through a helper tool (I may extend
the tool in the future to make some of those administration tasks more user friendly).

# Config file syntax ##

The syntax is still much in flux, and meanwhile I would like to use [StrictYAML](https://hitchdev.com/strictyaml/)
however there does not seem to be a good parser crate for rust yet. So for now I'm using
the serde-yaml parser, but restrict the config file to not use advanced yaml features.

Some examples can be found in etc directory:

- ### windows-11-desktop.yaml ###
  This is VM based on my company desktop vm, it runs fine and responsive using
  virtio-vga-gl, and remote-viewer, but does seem to use a bit more cpu cycles than my
  current ubuntu oriole desktop (see below).
- ### windows-11-gaming.yaml ###
  This is a config file that i derived from my proxmox windows 11 gaming VM, it is
  configured with GPU passthrough and looking-glass as viewer, and with
  the current ezkvm in combination with this config file, i can run that VM just fine
  from my EndeavourOS Gemini installation.
- ### ubuntu-oriole-desktop.yaml ###
  This works fine with the same settings as my windows 11 desktop settings, but for
  the sake of showing some diversity, it is configured with the gtk client here.
  I have also run opensuse tumbleweed with the same settings just fine.
- ### macos-sequoia-desktop.yaml ###
  A macos example, but as of yet untested. I have yet to try this on my laptop, but
  the basics should be in place for this to work.

To start the qemu VM, it should suffice to run `ezkvm --start <configfile>`.
