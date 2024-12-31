# Network Bridge

## Introduction

On a workstation or server it is easy to share your existing network interface
with your VM's by setting up a network bridge, and attaching the external interface
to it.

VM's can subsequently be configured to connect to that bridge, and thus share the
network interface. If you wish to set things up this way, please refer to documentation
of your distro to see how to setup a network bridge on your ethernet device.

Unfortunately that does not work with most wireless network cards, and so for a
laptop you would have to do it a bit differently.

## Setup a NAT bridge that forwards your VM's traffic

(this is similar to how `virsh net-start default` works with the default libvirtd bridge)
Configuration is slightly different now, typically it involves the following steps:

1) Enable routing
2) Create the bridge
3) Setup iptables with NAT rules
4) Start a DHCP server

Note that to do this, the script below relies on executables form the `net-tools`,
`bridge-utils` and `isc-dhcp-server` packages.

The start_vmbr0.sh script:

```
#!/bin/env bash

IP=192.168.191.1
NET=192.168.191.0/24
MASK=224.0.0.0/24
PORTS=1024-65535

# Allow routing
echo 1 > /proc/sys/net/ipv4/ip_forward

# Setup the bridge
brctl addbr vmbr0
ifconfig vmbr0 ${IP} up

# Setup NAT
iptables -t nat -N EZKVM_PRT
iptables -t nat -A POSTROUTING -j EZKVM_PRT
iptables -t nat -A EZKVM_PRT -s ${NET} -d ${MASK} -j RETURN
iptables -t nat -A EZKVM_PRT -s ${NET} -d 255.255.255.255/32 -j RETURN
iptables -t nat -A EZKVM_PRT -s ${NET} ! -d ${NET} -p tcp -j MASQUERADE --to-ports ${PORTS}
iptables -t nat -A EZKVM_PRT -s ${NET} ! -d ${NET} -p udp -j MASQUERADE --to-ports ${PORTS}
iptables -t nat -A EZKVM_PRT -s ${NET} ! -d ${NET} -j MASQUERADE

touch /var/ezkvm/dhcpd.vmbr0.lease
dhcpd -cf /etc/ezkvm/dhcpd.vmbr0.conf -pf /var/ezkvm/dhcpd.vmbr0.pid -lf /var/ezkvm/dhcpd.vmbr0.lease
```

And the /etc/ezkvm/dhcpd.vmbr0.conf contains:

```
option domain-name "ezkvm.net";
option domain-name-servers 8.8.8.8;

default-lease-time 600;
max-lease-time 7200;

ddns-update-style none;

subnet 192.168.191.0 netmask 255.255.255.0 {
  range 192.168.191.70 192.168.191.99;
  option routers 192.168.191.1;
}
```