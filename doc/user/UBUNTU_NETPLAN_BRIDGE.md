# Ubuntu 26.04: Netplan Bridge With DHCP (CLI)

This guide shows how to create a Linux bridge with Netplan on Ubuntu 26.04 and run a DHCP server on that bridge.

Use this setup for local VMs/containers attached to an isolated bridge network.

## Goal

- Create bridge `br0`
- Assign bridge gateway IP `10.50.0.1/24`
- Serve DHCP leases on `br0` with `dnsmasq`
- Optionally enable outbound internet access with NAT

## 1) Install Required Packages

```bash
sudo apt update
sudo apt install -y netplan.io dnsmasq
```

## 2) Create the Bridge With Netplan

Create `/etc/netplan/99-br0.yaml`:

```bash
sudo tee /etc/netplan/99-br0.yaml >/dev/null <<'YAML'
network:
  version: 2
  renderer: networkd
  ethernets:
    eno1:
      dhcp4: true
  bridges:
    br0:
      interfaces: []
      dhcp4: false
      addresses:
        - 10.50.0.1/24
      parameters:
        stp: false
        forward-delay: 0
YAML
```

Apply Netplan:

```bash
sudo netplan generate
sudo netplan try
sudo netplan apply
```

Verify bridge IP:

```bash
ip -br a show br0
```

Expected: `br0` has `10.50.0.1/24`.

## 3) Run DHCP Server on the Bridge

Create `dnsmasq` bridge config:

```bash
sudo tee /etc/dnsmasq.d/br0.conf >/dev/null <<'CONF'
interface=br0
bind-interfaces

# DHCP lease range on bridge subnet
dhcp-range=10.50.0.100,10.50.0.200,255.255.255.0,12h

# Default gateway offered to clients
dhcp-option=option:router,10.50.0.1

# DNS servers offered to clients
dhcp-option=option:dns-server,1.1.1.1,8.8.8.8
CONF
```

Enable and start:

```bash
sudo systemctl enable --now dnsmasq
sudo systemctl restart dnsmasq
```

Verify DHCP is listening:

```bash
sudo systemctl status dnsmasq --no-pager
sudo ss -lunp | grep ':67'
```

## 4) Optional: NAT for Internet Access From Bridge Clients

If bridge clients should access the internet through the host, enable IP forwarding and NAT.

Enable forwarding:

```bash
echo 'net.ipv4.ip_forward=1' | sudo tee /etc/sysctl.d/99-br0-forward.conf
sudo sysctl --system
```

Find uplink interface (example output might show `eno1`):

```bash
ip route | awk '/default/ {print $5; exit}'
```

Add NAT (replace `eno1` if needed):

```bash
sudo nft add table ip nat
sudo nft 'add chain ip nat postrouting { type nat hook postrouting priority 100; }'
sudo nft add rule ip nat postrouting oifname "eno1" ip saddr 10.50.0.0/24 masquerade
```

Verify NAT rule:

```bash
sudo nft list ruleset | sed -n '/table ip nat/,$p'
```

## Important Safety Notes

- Do not run this DHCP config on a bridge that is directly connected to a LAN that already has DHCP, unless you fully intend to serve DHCP there.
- For isolated VM networking, keep `interfaces: []` so `br0` is host-local and not bridged into your physical LAN.
- If you need true L2 bridging into a physical network, attach a physical interface to `br0` and ensure there is only one authoritative DHCP domain.

## Troubleshooting

1. `dnsmasq` fails to start:
   - Check for another DHCP/DNS service bound to UDP 67/53:

```bash
sudo ss -lunp | grep -E ':(53|67)'
```

2. Bridge not present after apply:
   - Validate YAML indentation and syntax:

```bash
sudo netplan --debug generate
```

3. Clients get lease but no internet:
   - Confirm forwarding is enabled:

```bash
sysctl net.ipv4.ip_forward
```

   - Confirm NAT rule uses the correct uplink interface.

## Rollback

Remove bridge and DHCP config:

```bash
sudo rm -f /etc/netplan/99-br0.yaml
sudo rm -f /etc/dnsmasq.d/br0.conf
sudo netplan apply
sudo systemctl restart dnsmasq
```

If you enabled forwarding and want to disable it:

```bash
sudo rm -f /etc/sysctl.d/99-br0-forward.conf
sudo sysctl --system
```
