# Feature Landscape: ezkvm Proxmox-to-QEMU Config Converter

**Domain:** VM configuration conversion tool (Rust library + CLI)
**Researched:** 2026-07-22
**Corpus:** `input/` directory — 60+ real Proxmox VM configs across 5 hosts
**Canonical test case:** `input/felucia/108.conf` — Windows 11 gaming VM with GPU passthrough, Looking Glass, SPICE, TPM v2.0, OVMF, USB passthrough, virtio-net, pvscsi

---

## Table Stakes

Features the tool must support to achieve basic VM conversion fidelity. Missing any of these = tool cannot represent real-world Proxmox VMs including the canonical test case.

| Feature | Why Required | Complexity | Current State | Notes |
|---------|-------------|------------|---------------|-------|
| **Proxmox `.conf` parser** | Entry point for all import workflows; key=value + option=val,opt=val format | Med | Scaffolding only (`ProxmoxVmSchema` empty) | Every field needs its own parser |
| **QEMU commandline generator** | Exit point for VM launch; must emit valid QEMU `-device`, `-object`, `-blockdev` etc. | High | Scaffolding only (`QemuSchema` empty) | Most complex output path |
| **ezkvm YAML round-trip** | Save/load; file schema + runtime schema well-defined but conversions unimplemented | Med | Schema defined; `TryFrom` stubs exist | saphyr serialization in place |
| **CPU config** | `cpu: host/EPYC/x86-64-v2-AES`, `sockets`, `cores`, optional `flags` | Low | `CpuSchema` exists in schema | Runtime representation incomplete |
| **Memory** | `memory: N` (MiB), used everywhere | Low | `MemorySchema` exists, basic runtime | Complete |
| **Machine type** | `machine: q35/pc-q35-8.1` — determines bus topology | Low | `MachineSchema` exists | Used in 100% of corpus |
| **Boot order** | `boot: order=scsiX;ideY;netZ` | Low | `BootSchema` exists | Simple semicolon-delimited list |
| **OS type hint** | `ostype: win11/win10/l26/other` — affects QEMU CPU/timer defaults | Low | **Missing from schema** | Needed for correct commandline |
| **SCSI controller** | `scsihw: pvscsi/virtio-scsi-pci/virtio-scsi-single` — determines SCSI bus device | Low | `PvScsi` in runtime; others missing | Three variants in corpus |
| **Storage: SCSI disks** | `scsiN: pool:disk,discard=on,size=XG,ssd=1,cache=writeback,iothread=1` | Med | Basic `Ssd`/`Hdd` exist; options incomplete | Most common storage type |
| **Storage: IDE (CDROM)** | `ide2: none,media=cdrom` or `local:iso/foo.iso,media=cdrom` | Low | `Cdrom` via `IdeDevice` exists | Needed for install media |
| **Storage: EFI disk** | `efidisk0: pool:disk,efitype=4m,pre-enrolled-keys=1,ms-cert=2023,size=4M` | Med | Schema partial; no Runtime type | Required for OVMF boot; 95%+ of corpus |
| **BIOS: OVMF** | `bios: ovmf` triggers pflash firmware setup in QEMU cmd | Med | `bios` field in schema; no commandline impl | Only bios type in entire corpus |
| **TPM v2.0** | `tpmstate0: pool:disk,size=4M,version=v2.0` → swtpm process + QEMU chardev | Med | `TpmSchema` (swtpm/hwtpm) in schema | Schema exists; commandline gen missing |
| **Network interfaces** | `netN: virtio=MAC,bridge=vmbr0,firewall=1` or `e1000e=MAC,...` | Med | `VirtioNet` in PcieDeviceTypeSchema | e1000e variant missing; multiple `net0..netN` |
| **PCI passthrough** | `hostpciN: 0000:XX:YY,pcie=1,x-vga=1,rombar=0,romfile=...` → `vfio-pci` | High | `Passthrough` in PcieDeviceTypeSchema | Multi-function passthrough (e.g., GPU+audio at same SBDF) needs special handling |
| **USB passthrough** | `usbN: host=BUS-PORT` or `host=VENDORID:DEVICEID` → `usb-host` device | Low | `UsbDeviceSchema` + `UsbDeviceResourceSchema` | Two addressing modes (bus:port vs VID:PID) |
| **VGA / display type** | `vga: none/virtio/virtio-gl/qxl/qxl2/vmware,memory=N` | Med | `DisplaySchema` in schema; vga=none for passthrough not modeled | `vga: none` = suppress emulated GPU; common with passthrough |
| **SMBIOS UUID** | `smbios1: uuid=...` → `-smbios type=1,uuid=...` | Low | `smbios_uuid` field in `VirtualMachineSchema` | Required for Windows activation stability |
| **vmgenid** | `vmgenid: UUID` → `-device vmgenid,guid=...` | Low | `vmgenid` field in `VirtualMachineSchema` | Needed for Windows VM generation ID |
| **QEMU guest agent** | `agent: 1` → virtserialport `org.qemu.guest_agent.0` | Low | `GuestAgent` in schema | Simple boolean in Proxmox |
| **SPICE display** | `args: -spice port=N,...` + virtio-serial + vdagent chardev | Med | `SpiceSchema` in display schema | Comes via `args` in Proxmox, not a first-class key |
| **ivshmem (Looking Glass)** | `args: -device ivshmem-plain,memdev=X -object memory-backend-file,...` | Med | `IvshmemPlain` in PcieDeviceTypeSchema | Comes via `args` in Proxmox; multiple instances per VM possible |
| **Audio** | `audio0: device=ich9-intel-hda,driver=spice` → `Ich9IntelHda` PCIe device + SPICE backend | Low | `AudioSchema` + `Ich9IntelHda` in schema | Only ich9-intel-hda+spice in corpus |
| **Raw `args` passthrough** | `args: <qemu flags>` — arbitrary QEMU flags Proxmox doesn't natively model | High | **Missing from schema and Runtime** | SPICE and ivshmem come through here in Proxmox; must parse AND preserve |
| **Snapshot sections** | `[snapshot_name]` sections in `.conf` files with full config per snapshot | Med | **Not implemented** | Every conf in corpus has snapshots; must parse `parent`, `snaptime`, comments |

---

## Differentiators

Features present in the real corpus but not strictly required by the canonical `felucia/108.conf` test case. Supporting them broadens coverage and prevents round-trip loss.

| Feature | Value Proposition | Complexity | Present In Corpus | Notes |
|---------|-------------------|------------|-------------------|-------|
| **NUMA topology** | `numa: 0/1`, `numa0: cpus=0-7,hostnodes=0,memory=N,policy=preferred` | High | `coruscant/501.conf`, `530.conf` | **Missing from schema**; critical for hugepages VMs; generates `-numa` + memory backend |
| **Hugepages** | `hugepages: 1024` → `-object memory-backend-file,...,mem-path=/run/hugepages/kvm/...` | High | `coruscant/501.conf`, `409.conf` | **Missing from schema**; changes memory QEMU model significantly |
| **Memory balloon** | `balloon: 0` (disable) or `balloon: N` (max balloon size) | Low | `zbp-server-mh2/201.conf` | **Missing from schema**; simple field |
| **Serial port** | `serial0: socket` → ISA serial chardev + socket | Low | `coruscant/501.conf`, many others | **Missing from schema**; straightforward |
| **USB tablet device** | `tablet: 0/1` → `usb-tablet,bus=ehci.0` | Low | Widespread in corpus | **Missing from schema**; affects mouse pointer behavior |
| **SPICE enhancements** | `spice_enhancements: foldersharing=1,videostreaming=all` | Low | `coruscant/501.conf` | **Missing from schema**; SPICE quality-of-life options |
| **VirtioNet advanced options** | Queue sizes, vhost, MTU, firewall per interface | Med | In corpus flags | Schema has `rx_queue_size`/`tx_queue_size`; firewall flag missing |
| **e1000e network** | `net0: e1000e=MAC,bridge=...` | Low | `coruscant/` VMs | Schema only has VirtioNet; needs e1000/e1000e variant |
| **Multiple PCI passthrough** | `hostpci0..hostpciN` up to 12 seen in corpus; multi-function devices | Med | `coruscant/501.conf` (hostpci0-2) | Schema supports it; Runtime device indexing needed |
| **Multi-function PCI** | Single `hostpciN` address covers `XX:YY.0` + `XX:YY.1` (e.g. GPU + audio) | High | Implicit in corpus | QEMU: separate `vfio-pci` per function; Proxmox: single `hostpciN: SBDF` |
| **Multiple USB passthrough** | `usb0..usb5` in corpus | Low | `coruscant/` VMs | Schema supports it; indexing in Proxmox import needed |
| **VID:PID USB passthrough** | `host=0451:16a0` (vendor:device ID) vs bus:port | Low | `coruscant/` VMs | `UsbDeviceResourceSchema` has variant; wiring needed |
| **Virtio disk** | `virtio0: pool:disk,...` → `virtio-blk-pci` | Med | Rare in corpus | Schema has virtio bus; no virtio disk type |
| **SATA disk** | `sata0: pool:disk,...` | Low | `zbp-server-mh2/` VMs | Schema has `SataDeviceSchema`; runtime wiring |
| **Storage: iothread** | `scsiN: ...,iothread=1` → per-disk I/O thread | Low | `coruscant/530.conf`, `zbp-*` | Affects QEMU commandline blockdev setup |
| **Storage: cache mode** | `cache=writeback/none/directsync` | Low | Widespread | Affects `detect-zeroes` + `discard` in blockdev JSON |
| **Storage: backup flag** | `backup=0` suppresses backup | Low | `coruscant/100.conf` | Proxmox-only metadata; no QEMU impact |
| **EFI ms-cert** | `efidisk0: ...,ms-cert=2023` — Windows Secure Boot cert year | Low | `felucia/108.conf` main section | Single option difference; determines OVMF firmware variant |
| **CPU with flags** | `cpu: host,flags=+ibpb;+virt-ssbd;+pdpe1gb;+aes` | Low | `coruscant/100.conf` snapshot | Semicolon-separated flag list |
| **CPU hidden** | `cpu: host,hidden=1` — hide KVM from guest | Low | Corpus | Important for anti-cheat/DRM in gaming VMs |
| **Unused disks** | `unusedN: pool:disk` — detached but tracked disks | Low | `coruscant/501.conf` | Proxmox metadata only; no QEMU impact; preserve for round-trip |
| **hookscript** | `hookscript: local:snippets/foo.sh` — Proxmox lifecycle hook | Low | Rare in corpus | Proxmox-only; no QEMU impact; preserve for round-trip |
| **localtime** | `localtime: 1` — RTC clock source | Low | Rare in corpus | Affects QEMU `-rtc base=localtime` |
| **template** | `template: 1` — marks VM as template | Low | Rare | Proxmox-only metadata |
| **meta field** | `meta: creation-qemu=8.1.5,ctime=1713817056` — creation metadata | Low | All corpus VMs | `MetaSchema` likely exists; verify round-trip |
| **vmstate** | `vmstate: pool:disk` — snapshot VM RAM disk | Low | Snapshots only | In snapshot sections; no QEMU impact |
| **bootdisk** | `bootdisk: scsi0` — explicit boot disk override | Low | Rare | Affects `-boot` commandline flag |
| **runningcpu/runningmachine** | `runningcpu: x86-64-v2-AES`, `runningmachine: pc-q35-8.1+pve0` | Low | Rare | Runtime state; ignore during import |
| **Multiple ivshmem** | Multiple ivshmem-plain instances in single `args` string (multi-VM Looking Glass host) | Med | `coruscant/501.conf` (3 ivshmem) | `args` parser must handle repeated device patterns |
| **QXL display** | `vga: qxl/qxl2,memory=N` | Low | `coruscant/` VMs | Variant of vga emulation schema |
| **virtio-gl display** | `vga: virtio-gl,memory=N` | Low | `coruscant/` VMs | GPU-accelerated virtio display |
| **QEMU `.cmd` import** | Parse `.qemu.cmd` (raw QEMU commandline) → Runtime | High | Input corpus has `.qemu.cmd` files | Reverse-engineering QEMU args; complex but enables auditing Proxmox's actual behavior |

---

## Anti-Features

Features to explicitly NOT build. These keep the scope tight on config conversion.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **VM lifecycle management** (start/stop/pause) | Not a VM manager; scope creep | Out of scope per PROJECT.md |
| **GUI or web interface** | CLI + library API is the target surface | Expose clean Rust API; let others build UI |
| **Live migration** | Proxmox cluster feature; massively complex | Only import/export static configs |
| **Network provisioning** | Host setup, not VM config | Represent bridge name as opaque string |
| **Storage provisioning** | Host setup, not VM config | Represent pool:volume as opaque resource reference |
| **Address conflict detection** | VM configs are trusted inputs from running Proxmox | Warn but don't reject; fidelity over validation |
| **OVMF binary distribution** | License/size concerns | Reference firmware path as configurable string |
| **swtpm process management** | Runtime concern, not config | Emit correct QEMU chardev args; let caller manage swtpm |
| **Proxmox API integration** | No live cluster access needed | Work with exported `.conf` files only |
| **Backup/restore management** | Out of scope per PROJECT.md | `backup=0` flag is preserved as metadata |
| **SeaBIOS VMs** | Zero examples in entire corpus; OVMF is universal | Don't model SeaBIOS boot; if needed later, add bios enum |
| **KVM device validation** | Host-specific; converter has no host context | Emit config as-is; validation is the caller's job |
| **Proxmox cluster features** (HA, pools, permissions) | Config converter scope | Preserve `hookscript`, `template` as opaque metadata fields |

---

## Feature Dependencies

```
Proxmox .conf parser
  └─ All other import features depend on this

Machine type
  └─ SCSI controller type (pvscsi/virtio-scsi-pci/virtio-scsi-single)
      └─ Storage: SCSI disks (bus topology)

BIOS: OVMF
  └─ EFI disk (pflash0 + pflash1 in QEMU cmd)

TPM
  └─ Storage: TPM state disk (tpmstate0 resource reference)

SPICE display
  └─ Audio (ich9-intel-hda requires SPICE driver in Proxmox)
  └─ SPICE enhancements (foldersharing, videostreaming)

PCI passthrough (GPU)
  └─ VGA: none (suppress emulated display)
  └─ ivshmem (Looking Glass shared memory — GPU passthrough use case)

NUMA
  └─ Hugepages (NUMA nodes with memory backend files)

args passthrough
  └─ SPICE (Proxmox emits SPICE via args in .conf)
  └─ ivshmem (Proxmox emits ivshmem via args in .conf)
  └─ Custom CPU flags override (e.g., -cpu host,-hypervisor)

Snapshot sections
  └─ parent field (snapshot DAG)
  └─ snaptime field (snapshot timestamp)
  └─ vmstate field (snapshot RAM disk)
```

---

## Feature Gaps: `args` Parsing vs Structured Representation

The Proxmox `args` field is the most tricky feature in the entire corpus. Proxmox uses it as an escape hatch for features it doesn't model natively. In `felucia/108.conf`, both SPICE and ivshmem come through `args`. This creates a design tension:

**Option A — Parse `args` into structured types:**
Detect known device patterns (`-spice`, `-device ivshmem-plain`, `-device virtio-serial-pci`, etc.) and lift them into the Runtime model as first-class structured types. Cleanest round-trip; enables ezkvm YAML to be human-readable. Complexity: high. Risk: unknown/exotic args patterns lost.

**Option B — Preserve `args` as opaque string:**
Store `args` verbatim in Runtime and re-emit exactly. Zero fidelity loss; trivial to implement. Complexity: low. Risk: ezkvm YAML contains opaque QEMU flags; cannot reason about devices in `args`.

**Option C — Hybrid (recommended):** Parse known patterns into structured types; preserve remainder as opaque `extra_args`. This is what the existing `PcieDeviceTypeSchema` suggests (ivshmem and Ich9IntelHda are modeled). Implement SPICE parsing from args, ivshmem parsing from args. Store truly unknown args as a `Vec<String>` passthrough field.

---

## MVP Recommendation

For the milestone targeting `felucia/108.conf` as minimum bar, prioritize in this order:

**Phase 1 — Runtime model completeness:**
1. CPU config (type + sockets + cores + optional flags/hidden)
2. OS type hint
3. BIOS enum + EFI disk type
4. TPM state disk (link tpmstate to resource)
5. VGA type (none/virtio/qxl/vmware + memory)
6. `args` parsing (SPICE → SpiceSchema, ivshmem → IvshmemPlain, preserve remainder)
7. Tablet device
8. Memory balloon

**Phase 2 — Proxmox .conf import:**
9. Key=value parser with option bag (`key: val,opt=val,opt=val`)
10. Indexed keys (`scsiN`, `hostpciN`, `usbN`, `netN`)
11. Snapshot section parser (`[name]` + body + parent/snaptime)
12. Storage resource references (pool:volume format)

**Phase 3 — QEMU commandline generation:**
13. Machine + pflash (EFI)
14. SMP + CPU
15. Memory
16. SCSI controller + blockdev JSON for each disk
17. Network (virtio-net-pci / e1000e)
18. PCI passthrough (vfio-pci, multi-function)
19. USB passthrough (usb-host)
20. TPM (swtpm chardev)
21. SPICE + vdagent + virtio-serial
22. ivshmem + memory-backend-file
23. vmgenid, smbios, vga

**Defer (differentiators, not in 108.conf):**
- NUMA + hugepages (in corpus, not in 108.conf; high complexity, significant QEMU model change)
- QEMU `.cmd` import (useful for audit but not needed for conversion)
- Serial port, virtio disk, SATA disk (present in other corpus VMs)

---

## Sources

- Corpus analysis: `input/felucia/108.conf`, `input/coruscant/501.conf`, `input/coruscant/100.conf`, `input/coruscant/530.conf`, `input/zbp-server-mh2/201.conf` (direct file read, HIGH confidence)
- Existing schema: `src/config/ezkvm/schema/` directory — `virtual_machine.rs`, `host.rs`, `pcie.rs`, `tpm.rs`, `audio.rs`, `display.rs` (direct file read, HIGH confidence)
- Existing runtime: `src/runtime/devices/` — pvscsi, pcie_net, pci_generic, usb_generic (direct file read, HIGH confidence)
- Proxmox config format: derived from corpus + QEMU `.cmd` files in `input/coruscant/` (empirical, HIGH confidence)
- QEMU commandline format: `input/coruscant/501.qemu.cmd` (direct file read, HIGH confidence)
