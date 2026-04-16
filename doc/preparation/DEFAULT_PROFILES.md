# Default Profiles For Proxmox Imports

## Purpose

This document evaluates whether ezkvm should extract reusable default profiles from the Proxmox validation corpus under `input/`, where:

- `*.conf` files are Proxmox VM configuration files
- `*.qemu.cmd` files are the resulting QEMU command lines

The goal is to identify common patterns that should become reusable ezkvm profiles, and to define how those profiles should be structured so the Proxmox importer can emit compact, maintainable YAML.

## Dataset Used

This analysis is based on the complete current corpus, not a sample.

- 58 Proxmox VM configs
- 58 generated QEMU command lines
- 53 VM pairs from `input/coruscant`
- 4 VM pairs from `input/zbp-server-mh2`
- 1 VM pair from `input/felucia`

Each `.conf` file was examined for Proxmox field usage and each `.qemu.cmd` file was examined for emitted QEMU feature patterns.

## Executive Summary

Reusable profiles should be extracted, but not only as one profile per guest OS.

The strongest result from the corpus is that the commonality is layered:

1. Proxmox baseline behavior
2. Guest OS family behavior
3. Access or display mode behavior
4. Passthrough and tuning behavior

The right profile system is therefore compositional.

The following profile families are justified by the corpus:

- `proxmox-q35-uefi`
- `storage-virtio-scsi-single`
- `storage-virtio-scsi-pci`
- `windows-common`
- `windows-11`
- `windows-10`
- `linux-l26-common`
- `macos-kvm`
- `remote-viewer-spice`
- `headless-vnc`
- `looking-glass`
- `gpu-passthrough`
- `hugepages`

The importer should assign several profiles to one VM rather than synthesizing a single giant OS-specific template.

## What The Corpus Says

### Guest OS distribution

Using the first `ostype` per VM:

| Proxmox `ostype` | VM count | Interpretation |
| --- | ---: | --- |
| `l26` | 42 | Generic Linux guest family |
| `win11` | 9 | Windows 11 family |
| `win10` | 3 | Windows 10 family |
| `other` | 4 | In practice all 4 are macOS-style guests |

Important note: `l26` in Proxmox is not literally an old Linux 2.6-only guest bucket. In this corpus it is the broad generic Linux guest class. A profile should therefore be named after the semantic intent, not the historical Proxmox label.

### Core virtualization baseline

Across the full corpus:

- All VMs are in the Q35 machine family
- 53 of 58 configs use `bios: ovmf`
- 57 of 58 configs define `scsihw`
- 30 VMs use `virtio-scsi-single`
- 26 VMs use `virtio-scsi-pci`
- 31 QEMU command lines contain `vfio-pci`
- 27 QEMU command lines contain SPICE
- 25 QEMU command lines contain VNC
- 33 QEMU command lines contain `-nographic`
- 50 QEMU command lines contain `virtio-scsi`
- 47 QEMU command lines contain `virtio-balloon`
- 53 QEMU command lines contain `usb-tablet`

This is a very strong argument for a reusable Proxmox baseline profile plus reusable storage and display profiles.

### Common Proxmox config keys by VM presence

Observed in the full set:

- `machine`, `memory`, `cores`, `sockets`, `ostype`, `smbios1`, `numa`, and `name` appear in all 58 VMs
- `boot` appears in 57 VMs
- `scsihw` appears in 57 VMs
- `bios` appears in 53 VMs
- `efidisk0` appears in 53 VMs
- `agent` appears in 52 VMs
- `scsi0` appears in 50 VMs
- `net0` appears in 40 VMs
- `hostpci0` appears in 34 VMs
- `serial0` appears in 27 VMs
- `tpmstate0` appears in 18 VMs
- `hugepages` appears in 23 VMs

These are not random one-off features. They describe a stable imported topology.

## Analysis By Requested Profile

## Proxmox

### Verdict

Yes. This is the strongest and safest shared profile to extract.

### Why

The entire corpus is rooted in Proxmox-generated Q35 VMs with consistent assumptions around:

- Q35 machine family
- UEFI/OVMF boot
- ballooning
- guest agent integration
- Proxmox-style machine options such as `hpet=off` in desktop-oriented Windows VMs
- Proxmox-oriented topology shaping already reflected in existing profile experiments under `etc/profiles.d`

### What belongs in a Proxmox baseline

- architecture default
- Q35 machine default
- UEFI boot defaults
- conservative boot menu and strict boot settings
- ballooning defaults
- guest agent default enablement

### What should stay out

- VM-specific UUIDs, TPM paths, MAC addresses, and disk paths
- host-specific Proxmox helper paths unless exact PVE reproduction is explicitly desired
- display mode
- storage controller choice if both SCSI variants are intended to remain supported

### Recommended profile name

- `proxmox-q35-uefi`

## Windows 10

### Verdict

Yes, but only as a thin overlay.

### Why

There are only 3 Windows 10 VMs in the corpus. They do not justify a large standalone profile, but they do justify a small overlay on top of a shared Windows base.

Common signals include:

- UEFI boot
- Windows-style Hyper-V enlightenments in generated QEMU CPU flags
- desktop-oriented behavior in two of the three cases

### Recommended structure

- shared profile: `windows-common`
- thin overlay: `windows-10`

### What belongs in `windows-10`

- at most policy defaults that differ from `windows-11`
- no mandatory TPM
- no mandatory secure boot

## Windows 11

### Verdict

Yes. This is a strong profile candidate.

### Why

The 9 Windows 11 VMs show consistent platform requirements:

- 9 of 9 have an EFI disk
- 9 of 9 have TPM state
- 8 of 9 have VMGenID
- 8 of 9 show Hyper-V CPU enlightenments in generated QEMU command lines
- most are desktop-style VMs with SPICE, VNC, audio, or tablet support

### What belongs in a Windows 11 overlay

- UEFI
- secure boot
- TPM 2.0 emulator defaults
- Windows Hyper-V enlightenments
- localtime RTC

### What should stay out

- exact display transport
- passthrough hardware selection
- Looking Glass settings
- VMGenID concrete value

### Recommended profile names

- `windows-common`
- `windows-11`

## Linux `l26`

### Verdict

Yes, but keep it broad and lightly opinionated.

### Why

The Linux corpus is large but diverse:

- 42 VMs total
- 41 have `scsi0`
- 40 have guest agent enabled
- 37 have EFI disk
- 27 have `serial0`
- 16 have `hugepages`
- 26 have `hostpci0`
- 24 generated QEMU command lines are headless or `-nographic`
- 19 use SPICE
- 18 use VNC
- 23 use VFIO passthrough

That is too mixed to justify a single highly opinionated desktop or server profile. The Linux layer should stay generic and let workload or display layers refine it.

### Naming recommendation

Do not name the ezkvm profile `linux-2.6`.

Use one of:

- `linux-l26-common`
- `linux-generic`

The first preserves a direct link to the Proxmox source signal. The second is more user-friendly.

### What belongs in it

- Q35 + UEFI baseline expectations if not already supplied by `proxmox-q35-uefi`
- host CPU default or other modern Linux-safe CPU default
- guest agent enabled
- no hardcoded display mode

## macOS

### Verdict

Yes. This is a distinct, high-value profile.

### Why

All 4 `ostype: other` VMs are clearly macOS-style guests. They are not generic unknown guests.

Observed shared signatures include:

- `args` containing AppleSMC device injection
- `-smbios type=2`
- Intel vendor spoofing
- Penryn-family or other Intel-focused CPU presentation
- pinned `pc-q35-5.2`
- OVMF
- `vga: none`
- GPU passthrough

This is a strongly differentiated virtualization pattern with guest-specific constraints. It should be a dedicated profile family.

### What belongs in `macos-kvm`

- pinned Q35 version, not floating latest Q35
- UEFI boot
- Intel-compatible CPU model and feature policy
- AppleSMC enablement
- SMBIOS type 2 extra argument
- no emulated VGA by default

### What must remain per-VM

- the OSK secret itself
- host PCI assignments
- optional ROM paths
- disk paths
- MAC addresses

### Recommended profile name

- `macos-kvm`

## Recommended Profile Taxonomy

The importer should use a layered taxonomy.

### Foundation profiles

- `proxmox-q35-uefi`
- `storage-virtio-scsi-single`
- `storage-virtio-scsi-pci`

### OS-family profiles

- `windows-common`
- `windows-11`
- `windows-10`
- `linux-l26-common`
- `macos-kvm`

### Access and display profiles

- `remote-viewer-spice`
- `headless-vnc`
- `headless-serial`
- `looking-glass`
- `audio-desktop`

### Passthrough and tuning profiles

- `gpu-passthrough`
- `usb-passthrough`
- `hugepages`
- `viommu`
- `hidden-hypervisor`

This separation matches the data better than any one-profile-per-OS scheme.

## Why Layered Profiles Are Better

The corpus shows that OS, display, passthrough, and tuning concerns are not tightly coupled.

Examples:

- Linux VMs appear as headless, SPICE, VNC, and GPU passthrough systems
- Windows 11 appears both as remote-viewer desktops and as a Looking Glass gaming VM
- macOS is an OS-specific guest class, but still composes with passthrough and storage decisions

If ezkvm emits only one large per-OS profile, the profile either becomes too broad to be reusable or too specific to be widely applicable.

Layered profiles keep the importer output smaller and make hand-maintained YAML easier to reason about.

## Suggested Profile Shapes

The shapes below are intentionally aligned with the live schema and with the current profiles in `etc/profiles.d`.

## `proxmox-q35-uefi`

```yaml
system:
  architecture: "x86_64"
  machine: "pc-q35-8.1+pve0"
  boot:
    firmware: "uefi"
    menu: true
    strict: true
    reboot_timeout: 1000
  memory:
    ballooning:
      enabled: true
      free_page_reporting: true
      model: "virtio-balloon-pci"
      id: "balloon0"
      bus: "pci.0"
      addr: "0x3"

options:
  guest_agent:
    enabled: true
```

## `storage-virtio-scsi-single`

```yaml
controllers:
  scsi:
    - id: "scsihw0"
      type: "virtio-scsi-single"
      bus: "pci.0"
      addr: "0x5"
```

## `storage-virtio-scsi-pci`

```yaml
controllers:
  scsi:
    - id: "scsihw0"
      type: "virtio-scsi-pci"
      bus: "pci.0"
      addr: "0x5"
```

## `windows-common`

```yaml
system:
  cpu:
    model: "host"
    features:
      - "hv_ipi"
      - "hv_relaxed"
      - "hv_reset"
      - "hv_runtime"
      - "hv_spinlocks=0x1fff"
      - "hv_stimer"
      - "hv_synic"
      - "hv_time"
      - "hv_vapic"
      - "hv_vpindex"
      - "+kvm_pv_eoi"
      - "+kvm_pv_unhalt"

options:
  rtc:
    base: "localtime"
    driftfix: "slew"
```

## `windows-11`

```yaml
system:
  boot:
    secure_boot: true
  tpm:
    version: "2.0"
    backend: "emulator"
    model: "tpm-tis"
```

## `linux-l26-common`

```yaml
system:
  cpu:
    model: "host"

options:
  guest_agent:
    enabled: true
```

## `macos-kvm`

```yaml
system:
  machine: "pc-q35-5.2+pve0"
  boot:
    firmware: "uefi"
  cpu:
    model: "Penryn"
    features:
      - "vendor=GenuineIntel"
      - "+invtsc"

extras:
  - "-device isa-applesmc,osk=${APPLE_OSK}"
  - "-smbios type=2"
```

## `remote-viewer-spice`

```yaml
display:
  type: "remote-viewer"
  usb_tablet: true

spice:
  addr: "127.0.0.1"
  port: 5900
  disable_ticketing: true
```

## `looking-glass`

```yaml
display:
  type: "looking_glass"

system:
  memory:
    ivshmem:
      enabled: true
      size: 128
      vectors: 1
      id: "ivshmem0"
      bus: "pcie.0"
      mem_path: "/dev/kvmfr0"

spice:
  addr: "127.0.0.1"
  port: 5900
  disable_ticketing: true
```

## `gpu-passthrough`

```yaml
host:
  pci: []

controllers:
  xhci:
    - id: "xhci"
      p2: 15
      p3: 15
      bus: "pci.1"
      addr: "0x1b"
```

## `hugepages`

```yaml
system:
  memory:
    hugepages: true
    prealloc: true
```

## Example Profile Stacks

The importer should emit stacks like these.

### Windows 11 desktop over remote-viewer

```yaml
profiles:
  - proxmox-q35-uefi
  - storage-virtio-scsi-single
  - windows-common
  - windows-11
  - remote-viewer-spice
```

### Windows 11 gaming VM with passthrough and Looking Glass

```yaml
profiles:
  - proxmox-q35-uefi
  - storage-virtio-scsi-single
  - windows-common
  - windows-11
  - gpu-passthrough
  - looking-glass
  - hugepages
```

### Generic Linux workstation or dev VM

```yaml
profiles:
  - proxmox-q35-uefi
  - storage-virtio-scsi-pci
  - linux-l26-common
  - remote-viewer-spice
```

### Headless Linux server

```yaml
profiles:
  - proxmox-q35-uefi
  - storage-virtio-scsi-pci
  - linux-l26-common
  - headless-vnc
```

### macOS passthrough VM

```yaml
profiles:
  - proxmox-q35-uefi
  - macos-kvm
  - gpu-passthrough
```

## Importer Assignment Rules

The importer should infer profile names from the source config and the resulting runtime characteristics.

## Foundation assignment

Assign `proxmox-q35-uefi` when all of the following hold:

- machine is in the Q35 family
- firmware is OVMF or an EFI disk is present

Assign storage profile based on `scsihw`:

- `virtio-scsi-single` -> `storage-virtio-scsi-single`
- `virtio-scsi-pci` -> `storage-virtio-scsi-pci`

## OS-family assignment

Assign `windows-common` when `ostype` is `win10` or `win11`.

Assign `windows-11` when `ostype` is `win11`.

Assign `windows-10` when `ostype` is `win10`.

Assign `linux-l26-common` when `ostype` is `l26`.

Assign `macos-kvm` when any of the following are true:

- `ostype` is `other` and raw `args` contains `isa-applesmc`
- raw `args` contains `-smbios type=2` and CPU presentation is Intel-spoofed
- machine is pinned to older Q35 and the VM uses `vga: none` plus AppleSMC extras

The macOS detection should be based on guest signatures, not only `ostype: other`.

## Access and display assignment

Assign `looking-glass` when generated QEMU contains `ivshmem-plain` or the source config clearly maps to the ezkvm ivshmem display flow.

Assign `remote-viewer-spice` when generated QEMU contains SPICE and the VM is clearly a desktop-style VM.

Assign `headless-vnc` when generated QEMU contains VNC and `-nographic` without a richer desktop display path.

Assign `headless-serial` when `serial0` exists and the VM is intentionally no-display.

## Passthrough and tuning assignment

Assign `gpu-passthrough` when:

- `hostpci0` exists and
- the runtime contains `vfio-pci` and
- the VM either uses `vga: none` or otherwise indicates the real display path is passthrough-oriented

Assign `hugepages` when Proxmox config contains `hugepages:`.

Assign `viommu` when machine options contain `viommu=`.

Assign `hidden-hypervisor` when the effective CPU flags include `kvm=off`, `-hypervisor`, or a spoofed vendor strategy intended to hide virtualization details.

## What Should Remain VM-Specific

The importer should keep the following in the VM file and not move them into shared profiles:

- name
- UUID
- VMGenID concrete value
- disk paths
- disk serials
- TPM disk and socket paths
- SPICE and VNC socket or TCP ports
- MAC addresses
- host PCI device IDs
- USB mappings
- ROM paths
- SMBIOS UUID values

These fields vary per machine and would make shared profiles brittle.

## Relationship To Existing Profiles

The current live profiles in `etc/profiles.d` already point in the right direction:

- the old underscore-named profile experiments have been superseded by the new hyphenated layered profile set in `etc/profiles.d`

The main refinement needed is not a conceptual change. The next step is to standardize these into a clearer layered taxonomy and align importer output to emit `profiles:` plus per-VM overrides.

## Current Repo Materialization

The following concrete profile files have now been added under `etc/profiles.d` using the live canonical schema:

- `proxmox-q35-uefi.yaml`
- `storage-virtio-scsi-single.yaml`
- `storage-virtio-scsi-pci.yaml`
- `windows-common.yaml`
- `windows-11.yaml`
- `windows-10.yaml`
- `linux-l26-common.yaml`
- `macos-kvm.yaml`
- `remote-viewer-spice.yaml`
- `looking-glass.yaml`
- `gpu-passthrough.yaml`
- `headless-serial.yaml`

These files are intentionally narrower than the ideal long-term taxonomy. They only encode settings that the current canonical profile schema can represent safely.

Importer status update:

- Proxmox import now emits inferred `profiles` stacks.
- Import output now applies a conservative profile-aware compaction pass that removes redundant VM-local fields already provided by inferred profiles.
- Profile-stack integration tests are in place for representative Windows, Linux, and macOS fixtures.

## Deferred Or Partially Represented Profiles

Some profile ideas from this analysis are not fully materialized yet because the current canonical schema does not model them directly.

### `headless-vnc`

Deferred.

Reason:

- the current canonical profile schema includes `spice` but not a canonical `vnc` section in the profile-oriented VM schema used by these layered profiles

### `hugepages`

Deferred.

Reason:

- the current canonical `system.memory` schema supports `size`, `ballooning`, and `ivshmem`, but not hugepages or preallocation controls in the profile path currently used here

### `macos-kvm` raw AppleSMC and SMBIOS extras

Partially represented.

Reason:

- the current canonical profile schema used here does not expose a raw `extras` or equivalent passthrough argument list for profile layering
- because of that, the concrete `macos-kvm.yaml` file currently captures the machine and CPU baseline but not the full AppleSMC and SMBIOS type 2 raw argument strategy seen in the Proxmox source corpus

Those gaps should be treated as implementation backlog, not as evidence against the profile taxonomy itself.

## Recommended Next Steps

1. Continue narrowing profile ownership boundaries (especially list-shaped sections) so compaction can safely remove more redundant fields over time.
2. Add explicit fixture coverage for remaining deferred profile families (`headless-vnc`, `hugepages`) once schema support lands.
3. Extend profile-stack corpus tests with more edge cases (nested virtualization, multiple display backends, and mixed storage buses).

## Recommended Rollout Order

1. Introduce `proxmox-q35-uefi`, `windows-common`, `windows-11`, `linux-l26-common`, and `macos-kvm`.
2. Split storage controller behavior into `storage-virtio-scsi-single` and `storage-virtio-scsi-pci`.
3. Split display behavior into `remote-viewer-spice`, `headless-vnc`, and `looking-glass`.
4. Add `gpu-passthrough` and `hugepages` as opt-in layers.
5. Teach the importer to map source signals to this layered profile stack.

This order gives immediate compaction wins while keeping the importer logic understandable.