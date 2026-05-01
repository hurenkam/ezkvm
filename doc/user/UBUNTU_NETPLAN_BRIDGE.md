# Ubuntu 26.04: Netplan Bridge With DHCP (CLI)

This guide shows how to create a Linux bridge with Netplan on Ubuntu 26.04 and run a DHCP server on that bridge.

Use this setup for local VMs/containers attached to an isolated bridge network.

If you use ezkvm or raw QEMU with `qemu-bridge-helper`, you must also allow the bridge in `/etc/qemu/bridge.conf`. The helper error `access denied by acl file` means the bridge exists but is not whitelisted there.

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

## 2a) Allow the Bridge for qemu-bridge-helper

If your VM networking uses `backend.type: bridge` with `qemu-bridge-helper`, add the bridge name to `/etc/qemu/bridge.conf`:

```bash
sudo install -d -m 0755 /etc/qemu
sudo tee /etc/qemu/bridge.conf >/dev/null <<'CONF'
allow br0
CONF
```

For a Proxmox-style bridge name, use that name instead:

```bash
sudo tee /etc/qemu/bridge.conf >/dev/null <<'CONF'
allow vmbr0
CONF
```

Verify the ACL file:

```bash
sudo cat /etc/qemu/bridge.conf
ls -l /usr/lib/qemu/qemu-bridge-helper
```

If your distro expects the helper to be setuid, verify that as well before starting the VM.

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

4. QEMU fails with `access denied by acl file`:
  - Confirm the requested bridge name is listed in `/etc/qemu/bridge.conf`:

```bash
sudo cat /etc/qemu/bridge.conf
```

  - For ezkvm imports from Proxmox, the requested bridge is often `vmbr0`, so the ACL file must contain `allow vmbr0` unless you intentionally renamed the bridge in the VM config.

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
