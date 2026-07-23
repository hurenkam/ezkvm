# Phase 2: Runtime Model — Research

**Researched:** 2026-07-22
**Domain:** Rust struct design, QEMU device modelling, Proxmox `.conf` semantics
**Confidence:** HIGH — all findings verified against actual source files and corpus data in this repo.

---

## Summary

Phase 2 adds seven first-class Runtime structs that complete the v1 device vocabulary
(`EfiDisk`, `TpmState`, `HostPci`, `Ivshmem`, `AudioDevice`, `SpiceDisplay`, `RawArgs`).
All real-world field values and QEMU commandline representations are verified against
`input/felucia/108.conf` and `input/felucia/108.ezkvm.qemu.cmd`; multi-host patterns are
verified against the 25-VM `input/coruscant/` corpus.

The central design decision is **trait assignment** — which bus trait each type implements,
and whether it registers as a `RootDevice` (system-level, owned by `RuntimeBuilder`) or a
bus-attached device (owned by `Q35ChipsetBuilder`). The research below resolves each of the
seven types unambiguously.

Two cross-cutting concerns apply to the whole phase:
1. **`device_kind()` extension** — Phase 1 defines the `RootDeviceKind` and `PcieBusDeviceKind`
   enums; Phase 2 must add one variant per new type to the right enum.
2. **No `downcast_ref`** — every new type must be reachable via `device_kind()` so the emitter
   phases never need speculative downcasts.

**Primary recommendation:** Five of the seven types are `RootDevice`-only; `HostPci` and
`Ivshmem` implement `PcieDevice` and register through `Q35ChipsetBuilder`. `RawArgs` is a
newtype-tuple `struct RawArgs(String)`. Keep all struct fields `Option<T>` only where the
Proxmox field is genuinely optional (corpus-verified below).

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RUNT-01 | Runtime supports EFI disk with efitype, pre-enrolled-keys, size | §EfiDisk findings |
| RUNT-02 | Runtime supports TPM state v2.0 with storage volume | §TpmState findings |
| RUNT-03 | Runtime supports PCI passthrough with pcie/x-vga/rombar/romfile | §HostPci findings |
| RUNT-04 | Runtime supports ivshmem with size and name/path | §Ivshmem findings |
| RUNT-05 | Runtime supports audio with device type and driver | §AudioDevice findings |
| RUNT-06 | Runtime supports SPICE display with gl/rendernode/port/clipboard | §SpiceDisplay findings |
| RUNT-07 | Runtime supports raw args preserved verbatim | §RawArgs findings |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| EFI firmware + NVRAM | RootDevice (system-level) | — | pflash units are wired directly to the QEMU machine; not on PCIe/PCI bus |
| TPM (swtpm + tpm-tis) | RootDevice (system-level) | — | `tpm-tis#2` sits directly under machine node in layout; swtpm is a companion process |
| PCI passthrough | PcieDevice (Q35 pcie slot) | — | Goes into `ich9-pcie-port-N` which is on `pcie.0`; needs bus-keyed addressing |
| Ivshmem shared memory | PcieDevice (pcie.0) | — | Machine layout shows `ivshmem-plain#7 @0x8` directly on `pcie.0` |
| HDA audio controller | RootDevice (system-level) | — | Q35 cfg places it at fixed slot `1b.0`/`pci.2,0xc`; no runtime-addressable placement needed |
| SPICE display server | RootDevice (system-level) | — | `-spice` is a QEMU machine-level option, not a device on a bus |
| Raw args passthrough | RootDevice (system-level) | — | Opaque string appended verbatim at end; no bus concept |

---

## Research Findings by Device

---

### RUNT-01: EfiDisk

#### Proxmox field values (from corpus) [VERIFIED: input/felucia/108.conf + coruscant corpus]

```
# From 108.conf active section (most recent snapshot):
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M

# From 108.conf before_lg snapshot (ms-cert absent):
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M

# Corpus variants — with pre-enrolled-keys:
efidisk0: vm1-pool:vm-101-disk-0,efitype=4m,pre-enrolled-keys=1,size=4M

# Corpus variants — without efitype (legacy 2M variant):
efidisk0: vm1-pool:vm-103-disk-1,size=4M
efidisk0: vm1-pool:vm-104-disk-1,size=4M
```

**Fields seen in corpus:**
- `storage_volume`: `<pool>:<name>` (always present, positional first token)
- `efitype`: `"4m"` (optional; absent = legacy 2M format)
- `pre_enrolled_keys`: `1` / absent (boolean flag; absent = false)
- `ms_cert`: `"2023"` (optional; only in 108.conf active section; MS Secure Boot cert year)
- `size`: `"4M"` / `"528K"` / other (logical size as reported by Proxmox)

#### QEMU representation [VERIFIED: input/felucia/108.qemu.cmd.split + coruscant corpus]

The EFI firmware requires **two `-drive if=pflash`** entries:

```
# Unit 0: read-only firmware (host path; ezkvm determines path from efitype + pre_enrolled_keys)
-drive if=pflash,unit=0,format=raw,readonly=on,file=/mnt/trixie/usr/share/pve-edk2-firmware/OVMF_CODE_4M.secboot.fd

# Unit 1: writable NVRAM (the actual storage volume)
-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-108-efidisk,size=540672
```

**Firmware file selection rule** [ASSUMED — based on Proxmox/OVMF conventions]:
- `efitype=4m` + `pre_enrolled_keys=1` → `OVMF_CODE_4M.secboot.fd` (Secure Boot enabled)
- `efitype=4m`, no `pre_enrolled_keys` → `OVMF_CODE_4M.fd`
- No `efitype` (legacy) → `OVMF_CODE.fd` (2M legacy format)

#### ⚠️ Dual-Size Pitfall [VERIFIED: corpus analysis]

The `size=4M` in the Proxmox conf is a **logical/display size** (4 MiB).
The `size=540672` in the QEMU drive is the **physical block device size in bytes** (540672 / 1024 = 528 KiB).
The `size=131072` variant in the corpus (= 128 KiB) corresponds to the legacy OVMF 2M format.

These two sizes are **not convertible from each other** — their relationship is opaque and
storage-pool-dependent. Both must be stored separately.

| QEMU size (bytes) | Logical size | OVMF type |
|-------------------|--------------|-----------|
| 540672 (528 KiB) | "4M" | 4M Secure Boot |
| 131072 (128 KiB) | "4M" | 2M Legacy |

#### Recommended struct

```rust
// src/runtime/efidisk.rs
#[derive(Debug, Clone, Getters, new)]
pub struct EfiDisk {
    storage_volume: String,           // "vm1-pool:vm-108-efidisk" — will resolve to path in Phase 4
    efitype: Option<String>,          // "4m" or None (legacy 2M)
    pre_enrolled_keys: bool,          // true when pre-enrolled-keys=1
    ms_cert: Option<String>,          // "2023" or None
    logical_size: String,             // "4M" — from .conf size= field, keep verbatim
    block_device_size_bytes: Option<u64>, // 540672 — from QEMU cmd size= field; None until resolved
}
```

**Trait assignment:** `RootDevice`

---

### RUNT-02: TpmState

#### Proxmox field values [VERIFIED: input/felucia/108.conf + coruscant corpus]

```
tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
```

**Entire corpus** shows only one variant: `size=4M,version=v2.0`. No v1.2 or other size observed.

Fields:
- `storage_volume`: `<pool>:<name>` (always present)
- `size`: `"4M"` (always "4M" in corpus — likely invariant for swtpm v2.0)
- `version`: `"v2.0"` (always present)

#### swtpm launch command [VERIFIED: input/felucia/108.swtpm.cmd + 108.ezkvm.swtpm.cmd]

```bash
# Proxmox original:
swtpm socket \
  --tpmstate backend-uri=file:///dev/vm1/vm-108-tpmstate,mode=0600 \
  --ctrl type=unixio,path=/var/run/qemu-server/108.swtpm,mode=0600 \
  --pid file=/var/run/qemu-server/108.swtpm.pid \
  --terminate --daemon --log file=/run/qemu-server/108-swtpm.log,level=1,prefix=[id=...] \
  --tpm2

# ezkvm version:
/usr/bin/swtpm socket \
  --tpm2 \
  --tpmstate backend-uri=file:///dev/vm1/vm-108-tpmstate \
  --ctrl type=unixio,path=/var/run/ezkvm/wakiza.swtpm,mode=0600 \
  --pid file=/var/run/ezkvm/wakiza.swtpm.pid \
  --terminate \
  --log file=/var/log/ezkvm/wakiza-swtpm.log,level=1 \
  --daemon
```

**swtpm lifecycle note:** swtpm must be launched **before** QEMU because QEMU connects to the
Unix socket immediately on startup. The socket path is derived from the VM name:
`/var/run/ezkvm/<vm_name>.swtpm`. This is a Phase 8 concern; Phase 2 only needs the struct.

#### QEMU representation [VERIFIED: input/felucia/108.qemu.cmd.split]

```
-chardev socket,id=tpmchar,path=/var/run/qemu-server/108.swtpm
-tpmdev emulator,id=tpmdev,chardev=tpmchar
-device tpm-tis,tpmdev=tpmdev
```

Machine layout confirms: `tpm-tis#2 (tpm-tis)` sits directly under the machine node (not
inside any bus in Q35). This confirms `RootDevice` is the correct trait.

#### Recommended struct

```rust
// src/runtime/tpmstate.rs
#[derive(Debug, Clone, Getters, new)]
pub struct TpmState {
    storage_volume: String,  // "vm1-pool:vm-108-tpmstate" — resolved to path in Phase 4
    version: String,         // "v2.0" — keep verbatim; drives --tpm2 flag
}
```

**Trait assignment:** `RootDevice`

---

### RUNT-03: HostPci

#### Proxmox field values [VERIFIED: input/felucia/108.conf + coruscant corpus]

```
# 108.conf active section:
hostpci0: 0000:03:00,pcie=1,x-vga=1

# Corpus patterns:
hostpci0: 0000:05:00.0,pcie=1
hostpci0: 0000:07:00,pcie=1,rombar=0
hostpci0: 0000:41:00,pcie=1,romfile=vgabios-radeon-rx570.bin
hostpci0: 0000:41:00,pcie=1,x-vga=1
hostpci1: 0000:12:00,pcie=1,x-vga=1,romfile=vgabios-amd-firepro-w7100.bin
hostpci1: 0000:41:00,pcie=1,rombar=1,romfile=vgabios-radeon-rx570.bin
```

**BDF format observation:**
- `0000:03:00` — **no function suffix** → this is a multi-function device; Proxmox auto-enumerates functions .0, .1
- `0000:05:00.0` — **explicit function** → only function 0 is passed through

**Sub-options seen in corpus:**
- `pcie=1` — almost always present; enables PCIe mode (vs PCI compatibility mode)
- `x-vga=1` — VGA compatibility flag (GPU primary display)
- `rombar=0` / `rombar=1` — hide/show ROM BAR; default is 1 (show)
- `romfile=<filename>` — custom VBIOS ROM file

#### QEMU representation [VERIFIED: input/felucia/108.qemu.cmd.split + 108.ezkvm.qemu.cmd]

**Proxmox output (multi-function):**
```
-device vfio-pci,host=0000:03:00.0,id=hostpci0.0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on
-device vfio-pci,host=0000:03:00.1,id=hostpci0.1,bus=ich9-pcie-port-1,addr=0x0.1
```

**ezkvm output (multi-function):**
```
-device vfio-pci,host=0000:03:00.0,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on
-device vfio-pci,host=0000:03:00.1,id=hostpci1,bus=ich9-pcie-port-1,addr=0x0.1
```

**Key differences Proxmox vs ezkvm:**
- Proxmox id: `hostpci0.0` / `hostpci0.1` (slot.function notation)
- ezkvm id: `hostpci0` / `hostpci1` (sequential integers — simpler)

**Single-function example (no rombar flag):**
```
-device vfio-pci,host=0000:05:00.0,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0
```

**rombar=0 example:**
```
-device vfio-pci,host=0000:05:00.0,id=hostpci0,bus=ich9-pcie-port-1,addr=0x0,rombar=0
```

#### ⚠️ Multi-function Pitfall [VERIFIED: corpus + machine-layout]

When BDF has no function suffix (`0000:03:00`), Proxmox auto-discovers both GPU (`.0`) and
audio (`.1`). The ezkvm design should store the **base BDF** plus a **Vec of function
numbers** so the QEMU emitter can expand them.

The `functions` field is populated:
- If BDF ends in `.N` → `vec![N]` (single function)
- If BDF has no function → `vec![0, 1]` (two-function default for GPU+audio; Phase 4 concern)

Phase 2 note: Phase 4 (Proxmox→Runtime) determines the actual function vector by probing
`/sys/bus/pci/devices/<bdf>/` or defaulting to [0, 1] for no-function BDFs. Phase 2 just
needs the field to exist.

#### Bus placement

Machine layout: `ich9-pcie-port-1 (pcie-root-port @1c.0)` → hostpci devices are children of
the pcie root ports. The root ports are on `pcie.0`. Each `hostpciN` occupies one root port
(`ich9-pcie-port-<N+1>`). The PcieAddress in Q35's pcie_bus should use `device=<slot_index>,
function=0` to map hostpciN to a stable slot.

#### Recommended struct

```rust
// src/runtime/devices/hostpci.rs
#[derive(Debug, Clone, Getters, new)]
pub struct HostPci {
    base_bdf: String,           // "0000:03:00" (no function) or "0000:05:00.0" (with function)
    functions: Vec<u8>,         // [0, 1] for multi-function GPU; [0] for single-function
    pcie: bool,                 // true when pcie=1
    x_vga: bool,                // true when x-vga=1
    rombar: Option<bool>,       // None = default (show), Some(false) = rombar=0
    romfile: Option<String>,    // "vgabios-radeon-rx570.bin" or None
}
```

**Trait assignment:** `PcieDevice` — registered in `Q35ChipsetBuilder.pcie_bus` keyed by
`PcieAddress { device: slot_idx, function: 0 }`.

---

### RUNT-04: Ivshmem

#### Proxmox / args field values [VERIFIED: input/felucia/108.conf + coruscant corpus]

Ivshmem is **not a standalone Proxmox key** — it appears inside the opaque `args:` value.
The ezkvm format promotes it to a first-class device.

```
# 108.conf args: contains:
-device ivshmem-plain,memdev=ivshmem0,bus=pcie.0
-object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M

# Corpus variants:
-device ivshmem-plain,memdev=ivshmem,bus=pcie.0 -object memory-backend-file,id=ivshmem,share=on,mem-path=/dev/shm/looking-glass,size=128M
-device ivshmem-plain,memdev=ivshmem12,bus=pcie.0 -object memory-backend-file,id=ivshmem12,share=on,mem-path=/dev/shm/lg12,size=32M
-device ivshmem-plain,memdev=ivshmem44,bus=pcie.0 -object memory-backend-file,id=ivshmem44,share=on,mem-path=/dev/shm/lg44,size=32M
```

**Multi-ivshmem example (single VM with two Looking Glass outputs):**
```
-device ivshmem-plain,memdev=ivshmem0,bus=pcie.0 -object memory-backend-file,id=ivshmem0,...,size=128M
-device ivshmem-plain,memdev=ivshmem1,bus=pcie.0 -object memory-backend-file,id=ivshmem1,...,size=128M
```

**Fields observed:**
- `id` / `memdev`: `ivshmem0`, `ivshmem12`, `ivshmem`, etc. — the correlating name between device and object
- `mem_path`: `/dev/kvmfr0` (KVMFR kernel module device) or `/dev/shm/<name>` (POSIX shm)
- `size`: `"128M"` or `"32M"` (string with unit suffix; keep verbatim)

#### QEMU representation [VERIFIED: input/felucia/108.ezkvm.qemu.cmd]

```
-device ivshmem-plain,memdev=ivshmem0,bus=pcie.0,addr=0x8
-object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
```

Machine layout confirms: `ivshmem-plain#7 (ivshmem-plain @0x8)` is directly on `pcie.0`.
The `-object` argument is a QEMU memory backend object (not a `-device`); it must precede
or accompany the `-device ivshmem-plain` line (both reference the same `id`).

#### Recommended struct

```rust
// src/runtime/devices/ivshmem.rs
#[derive(Debug, Clone, Getters, new)]
pub struct Ivshmem {
    id: String,        // "ivshmem0" — used as memdev= and memory-backend id=
    mem_path: String,  // "/dev/kvmfr0" or "/dev/shm/looking-glass"
    size: String,      // "128M" — keep verbatim; drives -object size= and Looking Glass config
}
```

**Trait assignment:** `PcieDevice` — registered in `Q35ChipsetBuilder.pcie_bus` keyed by
`PcieAddress { device: <ivshmem_slot_idx>, function: 0 }`.

---

### RUNT-05: AudioDevice

#### Proxmox field values [VERIFIED: input/felucia/108.conf + coruscant corpus]

```
audio0: device=ich9-intel-hda,driver=spice
```

Entire corpus shows exactly this one variant. No `pa` (PulseAudio) or `none` variants
observed in corpus, but the REQUIREMENTS.md and Proxmox docs list them as valid.

**Fields:**
- `device`: `"ich9-intel-hda"` (always in corpus; other valid values: `"intel-hda"`, `"ac97"`)
- `driver`: `"spice"` (in corpus; other valid: `"pa"`, `"pipewire"`, `"none"`)

#### QEMU representation [VERIFIED: input/felucia/108.qemu.cmd.split + 108.ezkvm.qemu.cmd]

The audio device generates **three related QEMU arguments**:

```
# 1. Audio backend (goes before -device):
-audiodev spice,id=spice-backend0

# 2. HDA controller:
-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc

# 3. Codec devices (always two: micro + duplex):
-device hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0
-device hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0
```

Machine layout: `audio0 (ich9-intel-hda @1b.0 readconfig)` is on `pcie.0`, then appears
as `audiodev0 (ich9-intel-hda @0xc)` under `pci.2` (the Q35 secondary PCI bridge). The
`readconfig` note means the Q35 config file defines the initial HDA slot at `1b.0`; ezkvm
actually places it at `pci.2,addr=0xc` via the ezkvm-q35.cfg equivalent.

The bus placement is **fixed by Q35 conventions**; the Runtime struct only needs to carry
`device_type` and `driver`. No runtime bus address needed.

#### Recommended struct

```rust
// src/runtime/audio.rs
#[derive(Debug, Clone, Getters, new)]
pub struct AudioDevice {
    device_type: String,  // "ich9-intel-hda" — the QEMU device name
    driver: String,       // "spice" | "pa" | "pipewire" | "none"
}
```

**Trait assignment:** `RootDevice` — bus placement is implicit in the Q35 configuration;
treating it as a system-level device avoids spurious slot management in Phase 2.

---

### RUNT-06: SpiceDisplay

#### Proxmox field values [VERIFIED: input/felucia/108.conf]

In `108.conf`, SPICE is configured via the `args:` key:

```
args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on ...
```

There is no standalone `spice:` key in `108.conf`. The Proxmox `.conf` format does support a
`spice:` config key (with options `gl`, `rendernode`, `port`, `clipboard`), but it is not
present in the felucia corpus. The REQUIREMENTS.md (RUNT-06) includes all four attributes.

**QEMU representation [VERIFIED: input/felucia/108.qemu.cmd.split + 108.ezkvm.qemu.cmd]:**

```
-spice port=5903,addr=0.0.0.0,disable-ticketing=on
```

The ezkvm.qemu.cmd generates this as a **first-class structured argument**, not via raw args
passthrough — confirming that the Phase 4 importer should extract SpiceDisplay from `args:`.

**Other corpus variants (from args: lines):**
```
-spice port=5904,addr=0.0.0.0,disable-ticketing=on
-spice port=5905,addr=0.0.0.0,disable-ticketing=on
-spice port=5909,addr=0.0.0.0,disable-ticketing=on
```

All corpus instances use `disable-ticketing=on`. `gl` and `rendernode` are [ASSUMED] based on
Proxmox documentation; no GL-enabled examples appear in this corpus.

The existing ezkvm schema `SpiceSchema` (in `src/config/ezkvm/schema/display.rs`) already has
`gl_enabled`, `port`, `listen`, `disable_ticketing` — confirming this field set.

#### Recommended struct

```rust
// src/runtime/spice.rs
#[derive(Debug, Clone, Getters, new)]
pub struct SpiceDisplay {
    port: Option<u16>,             // 5903 — None if not set (Unix socket mode)
    addr: Option<String>,          // "0.0.0.0" or None
    disable_ticketing: bool,       // true in all corpus examples
    gl: bool,                      // false in corpus; true when gl=on
    rendernode: Option<String>,    // "/dev/dri/renderD128" or None
    clipboard: bool,               // false by default
}
```

**Trait assignment:** `RootDevice` — SPICE is a machine-level display option, not a bus device.

---

### RUNT-07: RawArgs

#### Proxmox field values [VERIFIED: input/felucia/108.conf + coruscant corpus]

```
# 108.conf active section:
args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on -device virtio-serial-pci \
      -chardev spicevmc,id=vdagent,name=vdagent \
      -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 \
      -device virtio-mouse -device virtio-keyboard \
      -device ivshmem-plain,memdev=ivshmem0,bus=pcie.0 \
      -object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
```

The `args:` value is a single long string of QEMU arguments. In Phase 4, the Proxmox importer
will extract `SpiceDisplay` and `Ivshmem` from this string; the remainder that cannot be
structured (virtio-serial-pci, chardev/virtserialport for SPICE vdagent, virtio-mouse,
virtio-keyboard, CPU overrides, smbios overrides) will remain in `RawArgs`.

**Ordering constraint (QEMU-03):** Raw args must be appended **last** in the QEMU commandline.
This is because the `args:` content in Proxmox may contain cross-references like
`-chardev spicevmc,id=vdagent -device virtserialport,chardev=vdagent` that are internally
ordered. Reordering fragments within the string breaks these references.

**One instance per Runtime:** `args:` is a single field in Proxmox conf; at most one
`RawArgs` exists per Runtime.

#### Recommended struct

```rust
// src/runtime/rawargs.rs
#[derive(Debug, Clone)]
pub struct RawArgs(pub String);
```

**No `Getters` / `new` needed** — the single inner string is accessed via `.0` directly.

**Trait assignment:** `RootDevice`

---

## Recommended Struct Summary

| Type | File | Trait(s) | Registration |
|------|------|----------|--------------|
| `EfiDisk` | `src/runtime/efidisk.rs` | `RootDevice` | `RuntimeBuilder::with_efidisk()` |
| `TpmState` | `src/runtime/tpmstate.rs` | `RootDevice` | `RuntimeBuilder::with_tpmstate()` |
| `HostPci` | `src/runtime/devices/hostpci.rs` | `PcieDevice` | `Q35ChipsetBuilder::with_host_pci(idx, device)` |
| `Ivshmem` | `src/runtime/devices/ivshmem.rs` | `PcieDevice` | `Q35ChipsetBuilder::with_ivshmem(idx, device)` |
| `AudioDevice` | `src/runtime/audio.rs` | `RootDevice` | `RuntimeBuilder::with_audio_device()` |
| `SpiceDisplay` | `src/runtime/spice.rs` | `RootDevice` | `RuntimeBuilder::with_spice_display()` |
| `RawArgs` | `src/runtime/rawargs.rs` | `RootDevice` | `RuntimeBuilder::with_raw_args()` |

---

## Standard Stack

All dependencies already in `Cargo.toml`. No new crates needed for Phase 2.

[VERIFIED: Cargo.toml]

| Crate | Version | Purpose |
|-------|---------|---------|
| `derive-getters` | 0.5.0 | Field accessors via `#[derive(Getters)]` — matches all existing devices |
| `derive-new` | 0.7.0 | Constructor generation via `#[derive(new)]` — matches all existing devices |
| `thiserror` | 2 | Error types — added by Phase 1 |

---

## Architecture Patterns

### File Module Convention (from `src/runtime/README.md`) [VERIFIED]

```
src/runtime.rs           — declares `mod X; pub use X::...;` for root-level modules
src/runtime/             — one file per module
src/runtime/devices.rs   — declares `mod Y; pub use Y::*;` for device submodules
src/runtime/devices/     — concrete device implementations
```

**Rule:** Parent module declares children with `mod name;`; selectively re-exports with
`pub use`. Do not use `mod.rs` files.

### New RootDevice types → `src/runtime/<name>.rs`

Pattern (from `src/runtime/memory.rs`) [VERIFIED]:

```rust
use derive_getters::Getters;
use derive_new::new;
use crate::runtime::RootDevice;

#[derive(Debug, Clone, Getters, new)]
pub struct TpmState {
    storage_volume: String,
    version: String,
}

impl RootDevice for TpmState {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "tpmstate" }
    // device_kind() added by Phase 1 to RootDevice trait — must implement here
}
```

### New PcieDevice types → `src/runtime/devices/<name>.rs`

Pattern (from `src/runtime/devices/pcie_net.rs`) [VERIFIED]:

```rust
use derive_getters::Getters;
use derive_new::new;
use crate::runtime::PcieDevice;

#[derive(Debug, Clone, Getters, new)]
pub struct HostPci {
    base_bdf: String,
    functions: Vec<u8>,
    pcie: bool,
    x_vga: bool,
    rombar: Option<bool>,
    romfile: Option<String>,
}

impl PcieDevice for HostPci {
    fn as_any(&self) -> &dyn std::any::Any { self }
    // device_kind() added by Phase 1 to PcieDevice trait — must implement here
}
```

### `device_kind()` Integration

Phase 1 defines these enums (from `01-foundation/PLAN.md`) [VERIFIED]:
- `RootDeviceKind` in `src/runtime.rs`
- `PcieBusDeviceKind` in `src/runtime/pcie.rs`

Phase 2 must **add variants** to each:

```rust
// Add to RootDeviceKind (src/runtime.rs):
pub enum RootDeviceKind {
    Memory,
    Chipset,
    // --- Phase 2 additions:
    EfiDisk,
    TpmState,
    AudioDevice,
    SpiceDisplay,
    RawArgs,
}

// Add to PcieBusDeviceKind (src/runtime/pcie.rs):
pub enum PcieBusDeviceKind {
    PvScsi,
    VirtioNet,
    // --- Phase 2 additions:
    HostPci,
    Ivshmem,
}
```

### Q35ChipsetBuilder extension

Add two new methods to `Q35ChipsetBuilder` in `src/runtime/q35.rs`:

```rust
pub fn with_host_pci(mut self, idx: u8, device: Arc<dyn PcieDevice>) -> Self {
    // PcieAddress { device: idx, function: 0 } as the slot key
    self.pcie_bus.insert(PcieAddress::new(idx, 0), device);
    self
}

pub fn with_ivshmem(mut self, idx: u8, device: Arc<dyn PcieDevice>) -> Self {
    // Use high device indices to avoid collision with hostpci slots
    // e.g., hostpci uses 0..=15, ivshmem uses 16..=31
    self.pcie_bus.insert(PcieAddress::new(16 + idx, 0), device);
    self
}
```

**Planner note:** Exact address allocation strategy is **at the planner's discretion** — the
research just establishes that both HostPci and Ivshmem share Q35's `pcie_bus` HashMap and
need non-colliding slot keys.

### RuntimeBuilder extension (Phase 1 removes Mutex)

```rust
// Pattern for new RootDevice types (consistent with Phase 1 changes):
pub fn with_efidisk(mut self, efidisk: EfiDisk) -> Self {
    self.root_devices.push(Arc::new(efidisk));
    self
}
// ... repeat for TpmState, AudioDevice, SpiceDisplay, RawArgs
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Size string parsing ("4M" → bytes) | Custom parser | Keep as `String`; parsing deferred to Phase 4/7 |
| BDF function enumeration | Custom /sys prober | Store `Vec<u8>` from caller; Phase 4 resolves from Proxmox or host |
| SPICE arg parsing | Custom tokenizer | Phase 4 uses proper arg-splitting (Pitfall 3 from roadmap) |
| EFI firmware path lookup | Custom table | Phase 7 emitter holds host-specific path config |

---

## Common Pitfalls

### Pitfall 1: EFI Dual-Size — DON'T convert between `logical_size` and `block_device_size_bytes`

**What goes wrong:** Converting "4M" → 4194304 bytes and using that as the `size=` in the
pflash drive produces the wrong result. The QEMU `size=540672` (528 KiB) is NOT 4 MiB.

**Why it happens:** Proxmox's `size=4M` is the allocated pool size, not the pflash unit size.
The actual OVMF NVRAM images are 528 KiB (4M type) or 128 KiB (2M type).

**How to avoid:** Store `logical_size: String` verbatim from `.conf`; store
`block_device_size_bytes: Option<u64>` only when it's known from QEMU command output.
Phase 7 determines the `size=` value from the OVMF image size on the host, not from conversion.

### Pitfall 2: HostPci Multi-Function — Store base BDF + functions Vec, not expanded BDFs

**What goes wrong:** Storing `"0000:03:00.0"` and `"0000:03:00.1"` as two separate `HostPci`
structs prevents Phase 6 (YAML) from round-tripping to the Proxmox one-liner format.

**Why it happens:** The temptation is to expand multi-function early.

**How to avoid:** One `HostPci` per Proxmox `hostpciN:` line, with `base_bdf` = the raw BDF
string (may or may not include `.N`) and `functions: Vec<u8>` populated by Phase 4.

### Pitfall 3: RawArgs — Do NOT tokenize or restructure the string

**What goes wrong:** Splitting `args:` into individual flags and re-joining reorders
cross-referencing chardev/device pairs (e.g., `-chardev spicevmc,id=vdagent` must precede
`-device virtserialport,chardev=vdagent`). Reordering breaks the chardev reference.

**How to avoid:** `RawArgs(String)` stores the entire value verbatim. Phase 4 may extract
`SpiceDisplay` and `Ivshmem` before storing the remainder, but that extraction must preserve
relative ordering of remaining tokens.

### Pitfall 4: AudioDevice — Don't model as PciDevice for Phase 2

**What goes wrong:** Adding `AudioDevice` to Q35's `pci_bus` HashMap requires a `PciAddress`
and introduces slot management that isn't needed until Phase 7.

**Why it happens:** `ich9-intel-hda` is physically a PCI device.

**How to avoid:** Model as `RootDevice` for Phase 2. The Q35 ezkvm config file (`ezkvm-q35.cfg`)
already places the HDA controller at a fixed slot. Phase 7 just emits the fixed QEMU args.
If future phases need explicit slot management, `AudioDevice` can gain `PciDevice` then.

### Pitfall 5: `device_kind()` Enum Collisions with Phase 1

**What goes wrong:** Phase 1 names its pcie-bus enum `PcieBusDeviceKind` (not `PcieDeviceKind`)
to avoid collision with the existing `PciDeviceKind` in `pci_generic.rs`. Phase 2 must add to
the **same `PcieBusDeviceKind`** enum, not create a new one.

**How to avoid:** Read Phase 1's PLAN.md key_links section before adding any enum variants.
The canonical enum names are: `RootDeviceKind`, `PcieBusDeviceKind`, `PciBusDeviceKind`,
`UsbBusDeviceKind`, `IsaBusDeviceKind`.

### Pitfall 6: SpiceDisplay is NOT the SPICE vdagent virtserialport chain

**What goes wrong:** Conflating `SpiceDisplay` (the `-spice` machine option) with the
virtio-serial/chardev/virtserialport chain used for SPICE clipboard/vdagent (which lives in
`args:`).

**How to avoid:** `SpiceDisplay` covers only the `-spice port=...,addr=...,disable-ticketing=...`
flags. The vdagent chain stays in `RawArgs` until a future phase models it explicitly.

---

## Module Declaration Checklist

When Phase 2 adds `src/runtime/efidisk.rs`, `tpmstate.rs`, `audio.rs`, `spice.rs`,
`rawargs.rs`, it must also:

1. Add `mod efidisk;` (etc.) in `src/runtime.rs`
2. Add `pub use efidisk::EfiDisk;` (etc.) in `src/runtime.rs`
3. Add `mod hostpci; mod ivshmem;` in `src/runtime/devices.rs`
4. Add `pub use hostpci::*; pub use ivshmem::*;` in `src/runtime/devices.rs`
5. Add `HostPci`, `Ivshmem` to the `pub use devices::{...}` in `src/runtime.rs`

---

## QEMU Commandline Reference (per device)

| Device | QEMU Fragment |
|--------|---------------|
| EfiDisk (unit 0, firmware) | `-drive if=pflash,unit=0,format=raw,readonly=on,file=<OVMF_CODE_4M.secboot.fd>` |
| EfiDisk (unit 1, NVRAM) | `-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=<block_dev>,size=<bytes>` |
| TpmState (chardev) | `-chardev socket,id=tpmchar,path=<vm>.swtpm` |
| TpmState (tpmdev) | `-tpmdev emulator,id=tpmdev,chardev=tpmchar` |
| TpmState (device) | `-device tpm-tis,tpmdev=tpmdev` |
| HostPci (function .0) | `-device vfio-pci,host=<bdf>.0,id=hostpciN,bus=ich9-pcie-port-<M>,addr=0x0.0[,multifunction=on][,rombar=0][,romfile=<f>]` |
| HostPci (function .1) | `-device vfio-pci,host=<bdf>.1,id=hostpci<N+1>,bus=ich9-pcie-port-<M>,addr=0x0.1` |
| Ivshmem (backend) | `-object memory-backend-file,id=<id>,share=on,mem-path=<path>,size=<size>` |
| Ivshmem (device) | `-device ivshmem-plain,memdev=<id>,bus=pcie.0[,addr=<hex>]` |
| AudioDevice (backend) | `-audiodev spice,id=spice-backend0` (or `-audiodev pa,...` etc.) |
| AudioDevice (controller) | `-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc` |
| AudioDevice (codecs) | `-device hda-micro,...,audiodev=spice-backend0` + `-device hda-duplex,...` |
| SpiceDisplay | `-spice port=<port>,addr=<addr>,disable-ticketing=on[,gl=on][,rendernode=<path>]` |
| RawArgs | `<verbatim string appended at end>` |

---

## Validation Architecture

The phase goal is "structs compile and can be instantiated via builder API."
No test framework config changes are needed; the existing `cargo test` infrastructure
(Rust's built-in `#[test]`) is sufficient.

### Test Map

| Req ID | Behavior | Test Type | Command |
|--------|----------|-----------|---------|
| RUNT-01 | EfiDisk constructs with all fields | unit | `cargo test efidisk` |
| RUNT-02 | TpmState constructs and registers as RootDevice | unit | `cargo test tpmstate` |
| RUNT-03 | HostPci constructs multi-func (functions: [0,1]) | unit | `cargo test hostpci` |
| RUNT-04 | Ivshmem constructs and device_kind() returns Ivshmem | unit | `cargo test ivshmem` |
| RUNT-05 | AudioDevice constructs | unit | `cargo test audio` |
| RUNT-06 | SpiceDisplay constructs | unit | `cargo test spice` |
| RUNT-07 | RawArgs(String) preserves value verbatim | unit | `cargo test rawargs` |
| All | Runtime builder constructs all 7 types via API | integration | `cargo test runtime_all_devices` |

### Wave 0 Gaps

- [ ] Test stubs in `src/runtime/efidisk.rs` `#[cfg(test)]` block — covers RUNT-01
- [ ] Test stubs in `src/runtime/tpmstate.rs` — covers RUNT-02
- [ ] Test stubs in `src/runtime/devices/hostpci.rs` — covers RUNT-03
- [ ] Test stubs in `src/runtime/devices/ivshmem.rs` — covers RUNT-04
- [ ] Integration test in `src/runtime.rs` or dedicated test file — covers all-7 builder

---

## Recommended Task Breakdown for Planner

| Task | Deliverables | Dependencies |
|------|-------------|--------------|
| **02-01** | `EfiDisk` + `TpmState` structs; RootDevice impls; `device_kind()` variants added to `RootDeviceKind`; RuntimeBuilder `with_efidisk`, `with_tpmstate`; unit tests | Phase 1 complete |
| **02-02** | `HostPci` + `Ivshmem` structs; PcieDevice impls; `device_kind()` variants added to `PcieBusDeviceKind`; Q35ChipsetBuilder `with_host_pci`, `with_ivshmem`; unit tests | Phase 1 complete |
| **02-03** | `AudioDevice` + `SpiceDisplay` + `RawArgs` structs; RootDevice impls; `device_kind()` variants; RuntimeBuilder `with_audio_device`, `with_spice_display`, `with_raw_args`; unit tests | Phase 1 complete |
| **02-04** | Integration test: build a `Runtime` containing all 7 device types; `cargo test` passes; re-export all new types from `src/runtime.rs` | 02-01 + 02-02 + 02-03 |

Tasks 02-01, 02-02, 02-03 can execute **in parallel** (no inter-dependencies). Task 02-04
is the phase gate and requires all three.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `efitype=4m` without `pre_enrolled_keys` → `OVMF_CODE_4M.fd` (not secboot) | EfiDisk | Wrong firmware file selected; VM won't boot Secure Boot |
| A2 | Legacy no-efitype variant → `OVMF_CODE.fd` (2M) | EfiDisk | Wrong pflash unit size |
| A3 | `gl`, `rendernode`, `clipboard` SPICE fields are valid Proxmox `.conf` keys (not in felucia corpus) | SpiceDisplay | Planner includes fields that may never be populated from Proxmox import |
| A4 | Phase 1 enum names are `RootDeviceKind`, `PcieBusDeviceKind` | device_kind() section | Name collisions require rework |

---

## Open Questions

1. **HostPci slot allocation strategy in Q35 pcie_bus**
   - What we know: Q35 has ich9-pcie-port-1 through -N for passthrough devices
   - What's unclear: How many total pcie-port slots are pre-configured in ezkvm-q35.cfg, and how to map hostpci index to port index
   - Recommendation: For Phase 2, use `PcieAddress { device: idx, function: 0 }` as the map key and defer actual ich9-port assignment to Phase 7

2. **Phase 1 actual `device_kind()` signatures**
   - What we know: Phase 1 PLAN.md specifies `PcieBusDeviceKind`, `RootDeviceKind`
   - What's unclear: Whether Phase 1 is fully executed before Phase 2 starts, or if Phase 2 must handle the case where Phase 1 is in-progress
   - Recommendation: Gate Phase 2 on Phase 1 `cargo check` passing

3. **`ms_cert` semantics**
   - What we know: `ms-cert=2023` appears in one snapshot; 2023 refers to a Microsoft cert batch year
   - What's unclear: What runtime behavior it changes (firmware path? NVRAM content?)
   - Recommendation: Store as `Option<String>` verbatim; Phase 4 can interpret if needed

---

## Sources

### Primary (HIGH confidence)
- `input/felucia/108.conf` — canonical test case; all field values verified directly
- `input/felucia/108.qemu.cmd.split` — Proxmox QEMU cmd; all QEMU flag formats verified
- `input/felucia/108.ezkvm.qemu.cmd` — ezkvm QEMU cmd; target format verified
- `input/felucia/108.ezkvm.swtpm.cmd` + `108.swtpm.cmd` — swtpm cmd verified
- `input/felucia/108.machine-layout` — physical device topology verified
- `input/coruscant/*.conf` (25 VMs) — corpus patterns verified
- `input/coruscant/*.qemu.cmd.split` — QEMU args corpus verified
- `src/runtime/README.md` — design patterns and conventions verified
- `src/runtime/*.rs` + `src/runtime/devices/*.rs` — existing trait/struct patterns verified
- `src/config/ezkvm/schema/` (audio.rs, display.rs, tpm.rs, resources.rs) — existing ezkvm schema verified
- `.planning/phases/01-foundation/PLAN.md` — Phase 1 enum naming constraints verified
- `.planning/REQUIREMENTS.md` — v1 requirements text verified

### Tertiary (LOW confidence)
- Proxmox OVMF firmware file naming convention (A1, A2) — [ASSUMED] from training knowledge
- SPICE `gl`/`rendernode`/`clipboard` attribute list (A3) — [ASSUMED] from training knowledge; not in felucia corpus

---

## Metadata

**Confidence breakdown:**
- Struct fields: HIGH — all verified against actual corpus files
- Trait assignments: HIGH — verified against machine layout and existing pattern analysis
- QEMU fragment syntax: HIGH — verified against both 108.qemu.cmd.split and 108.ezkvm.qemu.cmd
- device_kind() enum names: HIGH — verified against Phase 1 PLAN.md
- OVMF firmware paths: LOW — host-specific, [ASSUMED] from Proxmox conventions

**Research date:** 2026-07-22
**Valid until:** Stable (all findings are against committed corpus files, not external sources)
