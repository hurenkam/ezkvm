# Phase 9: Round-Trip Verification - Research

> **⚠ PARTIALLY STALE (replanned 2026-07-29):** Phase 8.1 ("USB & SCSI Schema Extension") landed
> after this research was written and fixes two findings below at the schema level, not the
> emitter-level workarounds this document recommends:
> - **Pitfall 2** ("usb.rs vendor:product-ID gap ... must be resolved as part of this phase's
>   scope") — **STALE.** Phase 8.1 already fixed this via a typed `UsbHostIdentity` enum threaded
>   through Runtime/schema/importer/emitter. The `09-02-PLAN.md` this research recommended has been
>   dropped as redundant.
> - **Pitfall 3** ("scsihw is parsed but never consumed... do not assert a scsihw-specific device
>   string") — **STALE.** Phase 8.1 fully wired `scsihw` through the pipeline; Phase 9's plans now
>   DO assert controller-type-specific device strings per corpus file.
>
> See `09-REPLAN-NOTES.md` for the authoritative correction and full rationale. The remainder of
> this document is left unmodified as a historical record of the original research session.

**Researched:** 2026-07-29
**Domain:** Rust integration testing of an existing `TryFrom` conversion chain (Proxmox `.conf` → Runtime → ezkvm YAML → Runtime → QEMU cmdline); no new external dependencies.
**Confidence:** HIGH (all findings verified by reading this repo's actual source and corpus files, or by direct execution — no external library research was needed for this phase)

## Summary

Phase 9 does not need a new library, framework, or architecture — the full four-stage conversion chain (`ProxmoxImporter` → `Runtime` → `EzkvmConfigSchema` (YAML) → `Runtime` → `QemuCommandLine`) already exists and is independently tested per-stage in Phases 3-7. This phase's job is to chain all four stages together, once per corpus file, for three real-world `.conf`/`storage.cfg` pairs (felucia/108, coruscant/501, zbp-server-mh2/301), and assert structural invariants on the final `QemuCommandLine` output per `09-CONTEXT.md` D-04 (device-count/type presence, drive-before-device ordering, raw `args` verbatim) — never an exact-diff against the stale `108.ezkvm.qemu.cmd` reference (D-05).

Reading the actual corpus files surfaced two findings that materially change what this phase must plan for. First, **all three corpus configs contain an `efidisk0` entry, and `ProxmoxImporter` never resolves `EfiDisk.block_device_size_bytes`** — a pre-existing, already-documented gap (see Phase 7's `qemu_cmdline.rs`/`root.rs` test comments) that makes `emit_root_device`'s `EfiDisk` arm `.expect()`-panic if the full imported `Runtime` is fed directly into `QemuCommandLine::try_from`. Phase 7's own tests route around this by re-registering every root device except `EfiDisk` into a fresh `Runtime` before calling the QEMU emitter; Phase 9's Task 1 must reuse that exact workaround for all three corpus files, not just felucia, or every one of the three round-trip tests will panic.

Second, and more significant: **`zbp-server-mh2/301.conf`'s `usb4` (`host=0451:16a0`) and `usb5` (`host=0403:6001`) entries use Proxmox's vendor:product-ID USB passthrough syntax, not the bus-port syntax** (`host=1-4`, etc.) that `usb.rs`'s `emit_usb_device` handler assumes. The handler does `resource.split_once('-').expect(...)` unconditionally, and `"0451:16a0".split_once('-')` returns `None` (confirmed by direct execution in this research session) — so today's code **will panic** converting zbp-server-mh2/301's Runtime to a `QemuCommandLine`, unconditionally, before this phase adds a single line of code. This is a real gap in the emitter (not a test-writing problem) and must be resolved as part of this phase's scope — QEMU-04's success criteria explicitly require all three corpus configs to round-trip "without errors." The planner must decide how: add vendor:product-ID support to `usb.rs` (recommended — see Code Examples), or explicitly descope those two USB entries with user sign-off. Given `09-CONTEXT.md` grants no discretion to silently drop real corpus data, the recommended path is the former.

Beyond those two blocking findings, both storage.cfg files fully cover their `.conf`'s referenced storage pools (no missing-pool import errors), and the `numa`/`hugepages`/`balloon`/`unused0-3`/`spice_enhancements` fields present in coruscant/501 and zbp-server-mh2/301 are silently ignored by the parser (confirmed: "unknown keys are silently ignored" in `conf.rs`) — consistent with REQUIREMENTS.md's explicit v1 out-of-scope note for full NUMA support, so their presence causes no errors, just untested fields (acceptable per D-04's "structural spot-check" scope). The SCSI controller type (`scsihw: pvscsi` / `virtio-scsi-pci` / `virtio-scsi-single`) is parsed into `ProxmoxVmConf.scsihw` but never read by `ProxmoxImporter` — all SCSI disks import as `PvScsi` regardless of the source's declared controller, so the planner should not write assertions expecting a `virtio-scsi`-specific device string for coruscant/zbp; assert scsi drive/device presence generically instead.

**Primary recommendation:** Add a new `tests/round_trip_verification.rs` integration test file (mirroring `tests/qemu_cmdline.rs`'s existing style/helpers), with one test function per corpus file (3 total, or 3 `#[test]`s driven by a small shared per-corpus helper struct — either is idiomatic in this codebase's existing style). Before writing corpus-specific assertions, first fix the `usb.rs` vendor:product-ID gap (a small, scoped code change) — without it, the zbp-server-mh2/301 round trip cannot complete "without errors" as D-04 and QEMU-04 require.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `.conf`/`storage.cfg` parsing | Backend (importer) | — | `ProxmoxImporter`/`conf.rs`/`parser.rs` already own this; Phase 9 only consumes it |
| Runtime model construction | Backend (Runtime) | — | `RuntimeBuilder`/`Q35ChipsetBuilder` already own this; unchanged by Phase 9 |
| ezkvm YAML serialize/deserialize | Backend (config schema) | — | `EzkvmConfigSchema`/saphyr already own this; unchanged by Phase 9 |
| QEMU cmdline emission | Backend (emitter) | — | `QemuCommandLine::try_from`/`handlers/*.rs` already own this; Phase 9's usb.rs fix lands here |
| Structural spot-check assertions | Test / Integration | — | New `tests/round_trip_verification.rs`; no production code path, pure `#[test]` assertions |
| USB vendor:product-ID passthrough emission | Backend (emitter) | — | `handlers/usb.rs`'s `emit_usb_device` — genuine gap this phase must close (see Summary) |

## Package Legitimacy Audit

Not applicable — this phase introduces no new external dependencies (no new crates in `Cargo.toml`). All work is new test code plus a small fix to existing `handlers/usb.rs` logic, using only crates already present (`thiserror`, `derive-getters`, `derive-new`, `saphyr`, std lib).

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust `#[test]` / `cargo test` (integration tests) | (toolchain-provided) | Round-trip integration test harness | Already the sole test framework used across `tests/*.rs` in this repo; no justification to introduce anything else |

### Supporting
None — no new supporting libraries needed. All conversion types (`ProxmoxImporter`, `EzkvmConfigSchema`, `QemuCommandLine`) already exist in `src/config/`.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Plain `#[test]` functions per corpus | `rstest`/parameterized-test crate | Would reduce boilerplate for the 3 near-identical corpus tests, but introduces a new dev-dependency for marginal benefit in a 3-case suite; `09-CONTEXT.md`'s the agent's Discretion note defers this stylistic choice to whatever Phase 7's existing test style already uses (plain `#[test]`, no macro-driven parameterization) |

**Installation:** None required — no `Cargo.toml` changes.

**Version verification:** N/A — no new package versions to verify.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QEMU-04 | Generated commandline for felucia/108.conf produces a VM that starts in QEMU (reinterpreted per `09-CONTEXT.md` D-01 as: cmdline is well-formed and passes structural spot-checks; real boot deferred to Phase 10) | Confirmed existing `QemuCommandLine::try_from` and Phase 7's ordering/verbatim-args guarantees are sufficient to build these assertions for felucia; confirmed the two blocking gaps (EfiDisk workaround reuse, usb.rs vendor:product-ID fix) that must be addressed before coruscant/501 and zbp-server-mh2/301 can pass the same test shape |

## User Constraints

<user_constraints>

### Locked Decisions (from `09-CONTEXT.md`)

- **D-01:** Phase 9 does NOT spawn a real `qemu-system-x86_64` process. ROADMAP's success criterion #3 ("reaches the QEMU monitor prompt") is reinterpreted as: the generated cmdline is well-formed and passes structural assertions (D-04) — actual boot-readiness is deferred entirely to Phase 10.
- **D-02:** No new stub/fake binaries needed — Phase 8's `fake_qemu`/`fake_swtpm`/`fake_ui_client` stay untouched and unused; Phase 9 tests operate purely on the generated `Vec<String>`/cmdline model, never spawning any process.
- **D-03:** The two additional integration-test targets (beyond felucia/108.conf) are **coruscant/501.conf** and **zbp-server-mh2/301.conf** — chosen because both ship companion `lspci`/`lspci_t`/`lspci_v` (and, for zbp-server-mh2/301, `lsusb`) output for hardware cross-check, and coruscant exercises different SCSI-controller/NUMA paths.
- **D-04:** None of the three configs are verified via exact-string diff against a hand-authored reference `.qemu.cmd`. Instead: correct device count, correct device types present (cross-referenced against the `.conf`'s device keys and, where available, `lspci`/`lsusb` output), key flags/ordering rules holding (drive-before-device, raw `args` appended verbatim), and zero errors/panics through the full round-trip.
- **D-05:** `input/felucia/108.ezkvm.qemu.cmd` is **stale** (generated for an older ezkvm version) and MUST NOT be used as a diff target or correctness oracle. Do not regenerate or "fix" it. This supersedes ROADMAP's original success criterion #2 for all three configs uniformly, felucia included.

### the agent's Discretion

- Exact assertion granularity within "structural spot-checks" (e.g., exact `-device` argument counts vs. presence/absence per device-kind) — follow whatever rigor level Phase 7's `qemu.rs` unit tests already established, applied per-corpus-file.
- Whether coruscant/501 and zbp-server-mh2/301's `lspci`/`lsusb` files are parsed programmatically for assertions or used as human-reference documentation while writing tests — either acceptable as long as resulting assertions are correct.

### Deferred Ideas (OUT OF SCOPE)

- **Real-QEMU boot verification** (spawning `qemu-system-x86_64`, confirming monitor-prompt reachability) — belongs entirely to Phase 10 (Debian 13/Ubuntu 26.04 real-host tooling).
- **Regenerating a fresh, current `108.ezkvm.qemu.cmd` reference file** — user did not request this; noted only as a "could do later" idea, not adopted.

</user_constraints>

## Corpus Inventory (device types per file — for structural spot-check design)

### felucia/108.conf (existing corpus, already used by Phase 6/7 tests)

| Key | Value (active section) | Runtime device type |
|-----|------------------------|----------------------|
| `agent` | 1 | (not modeled — ignored) |
| `args` | full SPICE/ivshmem/vmouse/vkbd blob | `RawArgs` — must be verbatim, single occurrence |
| `audio0` | `device=ich9-intel-hda,driver=spice` | `AudioDevice` |
| `bios` | `ovmf` | consumed via `QemuContext.ovmf_code_path` (not a Runtime device) |
| `boot` | `order=scsi0;ide2;net0` | `Runtime.boot_order` (bootindex 100/101/102 per Phase 7-04) |
| `efidisk0` | `vm1-pool:vm-108-efidisk,...` | `EfiDisk` — **triggers the block_device_size_bytes gap, see Common Pitfalls** |
| `hostpci0` | `0000:03:00,pcie=1,x-vga=1` | `HostPci`, `functions: [0,1]` (x-vga expands to 2 functions) |
| `ide2` | `none,media=cdrom` | `Cdrom("none")` at `IdeAddress(1,0)` |
| `net0` | `virtio=...,bridge=vmbr0` | `VirtioNetPcie` |
| `scsi0`, `scsi1` | boot + tmp disks | `Ssd` via `PvScsi` (scsihw=pvscsi honored coincidentally — importer always emits PvScsi) |
| `tpmstate0` | `...,version=v2.0` | `TpmState` |
| `usb0` | `host=1-2.2` | `GenericUsbDevice` bus-port form — supported today |
| `vga` | `none` | consumed elsewhere / ignored for cmdline (no VGA device emitted when `none`) |
| `vmgenid` | uuid | not modeled (no Runtime field found) |

**Already tested:** `tests/yaml_round_trip.rs::felucia_108_runtime_round_trips_yaml`, `tests/qemu_cmdline.rs::test_qemu_cmdline_felucia_108_*` exercise import → YAML round trip and import → cmdline stages separately, but **no existing test chains all four stages (`.conf` → Runtime → YAML → Runtime → cmdline) in one pass** — this is genuinely new work for Phase 9's Task 1, even for felucia.

### coruscant/501.conf (second corpus target, D-03)

| Key | Value | Runtime device type | Note |
|-----|-------|----------------------|------|
| `args` | 3x `ivshmem` device/object pairs | `RawArgs` — verbatim, contains 3 distinct `ivshmem12`/`ivshmem43`/`ivshmem44` cross-references |
| `balloon` | 0 | not parsed — silently ignored (v1 out of scope) |
| `cpu` | `EPYC` | `CpuTopology` (cpu type string) |
| `efidisk0` | `vm1-pool:vm-501-disk-0,...` | `EfiDisk` — **same block_device_size_bytes gap as felucia** |
| `hostpci0` | `0000:07:00,pcie=1,rombar=0` | `HostPci`, `functions: [0]` (no x-vga) — real device is a USB controller (Fresco Logic FL1100, single function per lspci `01:00.0`) |
| `hostpci1` | `0000:41:00,pcie=1` | `HostPci`, `functions: [0]` — real hardware has `.0` VGA + `.1` Audio per lspci (`02:00.0`/`02:00.1`), but `.conf` omits `x-vga`, so only function 0 imports; **this matches existing Runtime semantics (x-vga is the only multi-function trigger), not a bug** |
| `hostpci2` | `0000:0e:11.0,pcie=1` | `HostPci`, base_bdf correctly stripped to `0000:0e:11` (BDF already includes `.0` suffix — importer's `rfind('.')` strip logic handles this correctly) — real device is `03:00.0 Ethernet controller: Intel X550 Virtual Function` |
| `hugepages` | 1024 | not parsed — silently ignored (v1 out of scope) |
| `numa` | 1 | `Runtime.numa` boolean parsed; `numa0:` sub-key (`cpus=0-7,hostnodes=0,...`) is NOT parsed — silently ignored, matches REQUIREMENTS.md RUNT-V2-01 deferral |
| `scsi0`,`scsi1`,`scsi2` | 3 disks across `vm1-pool`, `ws0`, `ws0-pool` | all import as `Ssd`/`Hdd` via `PvScsi` regardless of `scsihw: virtio-scsi-pci` |
| `scsihw` | `virtio-scsi-pci` | parsed into `ProxmoxVmConf.scsihw` but **never read by the importer** — does not affect emitted device type (always PvScsi-shaped) |
| `serial0` | `socket` | parsed into `ProxmoxVmConf.serial` (confirmed parsed) but not confirmed consumed into a Runtime root device — treat as ignored for cmdline purposes (RUNT-V2-02 deferred to v2) |
| `spice_enhancements` | `foldersharing=1,...` | not parsed — silently ignored |
| `unused0`,`unused1`,`unused2` | stale storage refs | not parsed — silently ignored (no error even though these reference volumes/snapshots that may not fully resolve) |
| `vmgenid` | uuid | not modeled |

**Storage cross-check:** `storage.cfg` defines `local`, `vm0`, `vm1-pool`, `vm2-pool`, `vm1`, `vm2`, `ws0`, `ws0-pool` — every pool referenced by `501.conf`'s active section (`vm1-pool`, `ws0`, `ws0-pool`) is present. **No missing-pool import errors expected.**

### zbp-server-mh2/301.conf (third corpus target, D-03)

| Key | Value | Runtime device type | Note |
|-----|-------|----------------------|------|
| `cores`/`sockets`/`cpu` | 12/1/host | `CpuTopology` |
| `efidisk0` | `vm0:vm-301-efidisk,...` | `EfiDisk` — **same block_device_size_bytes gap** |
| `hostpci0` | `0000:6e:00.0,pcie=1` | `HostPci`, base_bdf stripped to `0000:6e:00`, `functions: [0]` (no x-vga) |
| `hugepages` | 1024 | not parsed — ignored |
| `net0` | `virtio=...,bridge=vmbr0` | `VirtioNetPcie` |
| `numa` | 1 | boolean only, no `numaN:` sub-keys present in this file — nothing to ignore here |
| `scsi0`-`scsi3` | 4 disks across `vm0`/`ws0` | all import as `Ssd`/`Hdd` via `PvScsi` regardless of `scsihw: virtio-scsi-single` |
| `scsihw` | `virtio-scsi-single` | same non-effect as coruscant — parsed but unused |
| `serial0` | `socket` | parsed, not consumed for cmdline (RUNT-V2-02 deferred) |
| `unused1`,`unused2`,`unused3` | stale refs | not parsed — ignored |
| `usb0` | `host=1-4` | `GenericUsbDevice` bus-port form — supported |
| `usb1` | `host=1-5` | bus-port form — supported |
| `usb2` | `host=1-7.6` | bus-port form — supported |
| `usb3` | `host=1-7.5.1` | bus-port form — supported |
| `usb4` | `host=0451:16a0` | **vendor:product-ID form — NOT supported by `handlers/usb.rs` today; will panic (see Common Pitfalls / Summary)** |
| `usb5` | `host=0403:6001` | **vendor:product-ID form — same panic** |
| `vga` | `virtio-gl,memory=64` | consumed elsewhere / not modeled as a distinct Runtime device beyond what already exists |

**Storage cross-check:** `storage.cfg` defines `local`, `boot`, `ws0`, `vm0`, `bak`. `301.conf`'s active-section storage references are `vm0` (efidisk0, scsi0, scsi2, scsi3) and `ws0` (scsi1, unused1/2/3) — both present. **No missing-pool import errors expected.**

**lsusb cross-check:** `301.lsusb` confirms `0403:6001` (FTDI FT232 Serial) and `0451:16a0` (TI SmartRF05EB) are real, present USB devices on the source hardware — these are not typos or malformed corpus entries; the vendor:product-ID passthrough form is the correct real-world Proxmox syntax for "pass through this specific USB device by ID rather than by host bus/port," and must be supported.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| `.conf`/`storage.cfg` loading in the new test file | A new ad-hoc parser or fixture-loading helper | Copy the exact `load_felucia_108()`-shaped helper pattern already used in `tests/yaml_round_trip.rs` and `tests/qemu_cmdline.rs` (read file, `ProxmoxVmConf::from_str`, `ProxmoxStorageConf::from_str`), parameterized per corpus directory/vmid | These integration test files cannot share code across binaries in this crate's current structure (confirmed: `tests/qemu_cmdline.rs`'s own comment states "Rust integration test binaries cannot import each other's code without a shared test-support module, which is out of scope"); duplicating the small helper per test file is the established, intentional pattern here — do not attempt to introduce a `tests/common/mod.rs` shared-helpers module unless the planner deliberately decides to invest in that refactor (out of this phase's scope per CONTEXT.md) |
| EfiDisk-triggered panic workaround | A production-code fix to populate `block_device_size_bytes` during import | Reuse the exact `felucia_chipset_and_boot_order()`/`felucia_runtime_for_cmdline()` pattern (rebuild a fresh `Runtime` re-registering every root device except `EfiDisk`) already used in `tests/qemu_cmdline.rs` and `handlers/root.rs`'s own test module | This is a pre-existing, already-documented, out-of-scope gap per Phase 7's own comments; Phase 9 is an integration-test phase, not a bug-fix phase for this specific gap — the existing workaround is proven and reusable across all three corpus files |
| USB vendor:product-ID emission | A bespoke ad-hoc string-matching hack inside the new test file to "skip" or "patch around" usb4/usb5 | A proper `UsbDeviceKind` variant (or extended `HostPassthrough` handling) in `src/runtime` + a corresponding `handlers/usb.rs` match arm emitting `-device usb-host,bus=xhci.0,port=N,vendorid=0x<VID>,productid=0x<PID>,id=usbN` | This is a genuine, small, well-scoped production-code gap; hacking around it in test code would violate D-04's "zero errors/panics through the full round-trip" requirement while leaving the real emitter broken for any future real-world config with ID-based USB passthrough |

**Key insight:** Almost nothing new needs to be built for this phase except (1) the integration test file itself, chaining stages already proven correct in isolation, and (2) one small, well-understood fix to `handlers/usb.rs` to support the vendor:product-ID USB passthrough syntax that real corpus data (zbp-server-mh2/301) actually uses. Everything else is composition of existing, already-tested conversion code.

## Common Pitfalls

### Pitfall 1: EfiDisk `.expect()`-panic on `block_device_size_bytes` (affects ALL THREE corpus files)
**What goes wrong:** Feeding a fully-imported `Runtime` (with its `EfiDisk` root device intact) directly into `QemuCommandLine::try_from((runtime, ctx))` panics inside `emit_root_device`'s `EfiDisk` arm at `.block_device_size_bytes().expect("EfiDisk.block_device_size_bytes required for pflash size= (D-06)")`.
**Why it happens:** `ProxmoxImporter` never populates `EfiDisk.block_device_size_bytes` for any Proxmox-imported config (confirmed by `importer.rs`'s own `test_04_02_efidisk_logical_size_from_options`, which asserts `block_device_size_bytes().is_none()`); this is a pre-existing gap from Phase 4, not introduced by Phase 7 or Phase 9.
**How to avoid:** Reuse Phase 7's documented workaround — build a fresh `Runtime`, re-registering every root device from the fully-imported `Runtime` except `EfiDisk`, before calling `QemuCommandLine::try_from`. Apply this identically for felucia, coruscant, and zbp-server-mh2. If the planner wants the round-trip test to also prove `EfiDisk` round-trips correctly through YAML (it can — the YAML stage does not hit this gap, only the final QEMU-emission stage does), keep `EfiDisk` in the Runtime through the YAML round-trip stage and only strip it immediately before the final `QemuCommandLine::try_from` call.
**Warning signs:** A test that panics with a message containing `"D-06"` or `"block_device_size_bytes"` — this is this exact known gap, not a new bug.

### Pitfall 2: USB vendor:product-ID passthrough is unsupported by `handlers/usb.rs` (blocks zbp-server-mh2/301 specifically)
**What goes wrong:** `emit_usb_device`'s `HostPassthrough` arm does `resource.split_once('-').expect("UsbDeviceKind::HostPassthrough.resource must be '<bus>-<port>' ...")`. For `usb4: host=0451:16a0` and `usb5: host=0403:6001`, `resource` is `"0451:16a0"`/`"0403:6001"` — neither contains a `-`, so `split_once` returns `None` and the `.expect()` panics. **Confirmed by direct execution in this research session** (not merely inferred from reading the code).
**Why it happens:** The Proxmox `usbN: host=...` field accepts two distinct real-world syntaxes — `<bus>-<port>` (e.g. `1-2.2`) for passthrough-by-topology, and `<vendorid>:<productid>` (e.g. `0451:16a0`) for passthrough-by-device-identity. The parser (`parse_usb_raw`) already stores either form transparently as a raw string (confirmed by its own test `test_parse_usb_raw_vid_pid`), and the importer wraps either form identically into `UsbDeviceKind::HostPassthrough { resource }` without distinguishing them — the distinction is only made (incompletely) at emission time in `handlers/usb.rs`.
**How to avoid:** Detect the two forms in `emit_usb_device` (e.g., `resource.contains(':')` → vendor:product-ID form → emit `-device usb-host,bus=xhci.0,port={port},vendorid=0x{vid},productid=0x{pid},id=usb{port}`; else → bus-port form, existing behavior). Real QEMU syntax supports `vendorid=`/`productid=` as hex (with `0x` prefix) on `-device usb-host`. This is a small, scoped, low-risk fix — verify against QEMU's own `usb-host` device documentation before finalizing the exact flag names/format.
**Warning signs:** A test that panics with a message containing `"must be '<bus>-<port>'"` when processing zbp-server-mh2/301 — this is this exact gap.

### Pitfall 3: `scsihw` does not influence the emitted SCSI controller type
**What goes wrong:** Writing an assertion that expects a `virtio-scsi-pci`-specific or `virtio-scsi-single`-specific device string in the coruscant/zbp cmdline output (because their `.conf` declares `scsihw: virtio-scsi-pci`/`virtio-scsi-single`) will fail, because `ProxmoxImporter` always constructs a `PvScsi` root device for SCSI disks regardless of the `scsihw` field's value.
**Why it happens:** `ProxmoxVmConf.scsihw` is parsed (stored) but never read anywhere in `importer.rs` — the SCSI-import loop unconditionally uses `PvScsiBuilder`.
**How to avoid:** Assert scsi drive/device *presence* and *count* generically (e.g., "N `-drive` entries with `id=drive-scsiN`, N corresponding entries referencing that drive"), not a specific controller-model string tied to the source `.conf`'s declared `scsihw` value. This is consistent with `09-CONTEXT.md`'s D-04 "structural spot-check" framing (device count/types present), not an exact re-derivation of the source hardware's controller model.
**Warning signs:** Hard failures asserting on literal `virtio-scsi-pci`/`virtio-scsi-single` substrings in generated output for coruscant/zbp.

### Pitfall 4: `RawArgs` re-tokenization (carried over from Phase 7 research, still applies)
**What goes wrong:** Treating the `args` blob (present in felucia and coruscant, absent in zbp-server-mh2/301) as anything other than an opaque, single, verbatim string — e.g. splitting on whitespace, reordering embedded `-device`/`-object`/`-chardev` tokens — breaks internal cross-references (`chardev=vdagent` ↔ `virtserialport`; `-object memory-backend-file,id=ivshmem0` ↔ `-device ivshmem-plain,memdev=ivshmem0`) that real QEMU depends on positionally/by-reference, not by token order guarantees from ezkvm.
**Why it happens:** Generic device-emission code paths are tempting to reuse for "just one more string," but `RawArgs` is Proxmox user-authored opaque passthrough, never parsed by ezkvm (per project decision "args field is opaque" in `STATE.md`).
**How to avoid:** Assert the exact captured `args:` string from the parsed `.conf` appears verbatim, exactly once, as a contiguous substring in the final cmdline output — reuse the exact assertion pattern from `tests/qemu_cmdline.rs::test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end` (search for the raw blob within `output`, and separately confirm it is positioned after the last real `-device` token, being careful that the blob itself contains embedded `-device` substrings that a naive `rfind` would false-positive on).
**Warning signs:** An assertion using `output.rfind("-device")` directly against the full rendered string instead of against `cmdline.devices().last()` — this is the exact false-positive trap Phase 7's own test comments call out.

## Code Examples

### Existing four-stage helper pattern to extend (from `tests/qemu_cmdline.rs`)
```rust
// Source: tests/qemu_cmdline.rs (existing, verified in this repo)
fn load_felucia_108() -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let conf_str = std::fs::read_to_string("input/felucia/108.conf")
        .expect("input/felucia/108.conf not found");
    let storage_str = std::fs::read_to_string("input/felucia/storage.cfg")
        .expect("input/felucia/storage.cfg not found");
    let vm_conf = ProxmoxVmConf::from_str(&conf_str).expect("parse 108.conf");
    let storage_conf = ProxmoxStorageConf::from_str(&storage_str).expect("parse storage.cfg");
    (vm_conf, storage_conf)
}

// Recommended generalization for Phase 9 (per-corpus parameterized form):
fn load_corpus(dir: &str, conf_file: &str) -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let conf_str = std::fs::read_to_string(format!("input/{dir}/{conf_file}"))
        .unwrap_or_else(|_| panic!("input/{dir}/{conf_file} not found"));
    let storage_str = std::fs::read_to_string(format!("input/{dir}/storage.cfg"))
        .unwrap_or_else(|_| panic!("input/{dir}/storage.cfg not found"));
    let vm_conf = ProxmoxVmConf::from_str(&conf_str).expect("parse .conf");
    let storage_conf = ProxmoxStorageConf::from_str(&storage_str).expect("parse storage.cfg");
    (vm_conf, storage_conf)
}
```

### EfiDisk-stripping workaround to reuse (from `tests/qemu_cmdline.rs`)
```rust
// Source: tests/qemu_cmdline.rs::felucia_runtime_for_cmdline (existing, verified in this repo)
fn runtime_for_cmdline(full_runtime: Runtime) -> Runtime {
    let mut runtime = Runtime::new();
    for device in full_runtime.root_devices() {
        if device.device_kind() != RootDeviceKind::EfiDisk {
            runtime.register_root_device(device.clone());
        }
    }
    runtime
}
```

### Recommended `handlers/usb.rs` fix sketch (vendor:product-ID support)
```rust
// Sketch only — planner/executor to verify exact QEMU usb-host flag names/hex formatting
// against QEMU's own -device usb-host documentation before implementing.
UsbDeviceKind::HostPassthrough { resource } => {
    if let Some((vid, pid)) = resource.split_once(':') {
        builder.push_device(format!(
            "-device usb-host,bus=xhci.0,port={},vendorid=0x{},productid=0x{},id=usb{}",
            address.port(), vid, pid, address.port()
        ));
    } else {
        let (hostbus, hostport) = resource.split_once('-').expect(
            "UsbDeviceKind::HostPassthrough.resource must be '<bus>-<port>' or '<vid>:<pid>' (D-06)",
        );
        builder.push_device(format!(
            "-device usb-host,bus=xhci.0,port={},hostbus={},hostport={},id=usb{}",
            address.port(), hostbus, hostport, address.port()
        ));
    }
}
```

## State of the Art

Not applicable — this phase does not involve external library version drift. All code involved is this repo's own, current implementation.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The recommended `-device usb-host,...,vendorid=0x...,productid=0x...` flag syntax is the correct modern QEMU idiom for vendor:product-ID USB host passthrough | Code Examples / Pitfall 2 | Low-medium — this is standard, long-stable QEMU `usb-host` syntax (`[ASSUMED]` from training knowledge, not verified against a fetched QEMU doc in this research session since this is a code/test phase with no external doc lookup budget spent); if wrong, the fix would need one flag-name correction, not an architecture change. Planner should have the executor cross-check against `qemu-system-x86_64 -device usb-host,help` or QEMU docs at implementation time. |
| A2 | `serial0: socket` entries in coruscant/zbp are parsed but not consumed into any Runtime root device for cmdline emission | Corpus Inventory tables | Low — confirmed `serial` field exists in `ProxmoxVmConf` and is parsed, but this research did not exhaustively trace every consumer of `self.vm_conf.serial` in `importer.rs`; if it is silently consumed somewhere, structural spot-checks for coruscant/zbp may need an extra serial-device assertion. Recommend the executor grep `vm_conf.serial` in `importer.rs` before finalizing per-corpus assertions. |

## Open Questions

1. **Should the `usb.rs` vendor:product-ID fix be a task within Phase 9, or should zbp-server-mh2/301's usb4/usb5 be explicitly descoped with user sign-off?**
   - What we know: Real corpus data uses this syntax; the current emitter panics on it; QEMU-04 and D-04 both require "zero errors/panics through the full round-trip" for all three chosen corpus files.
   - What's unclear: Whether `09-CONTEXT.md`'s the agent's Discretion section intended to grant scope for small production-code fixes discovered during planning, versus being strictly test-writing only.
   - Recommendation: Add it as an explicit, small Task 1 (or early sub-task) in the plan — fixing this is a prerequisite for zbp-server-mh2/301's round-trip to complete at all, not an optional nicety; flag it prominently to the user during `/gsd-plan-phase` review given it is a production code change discovered mid-research, not merely test code.

2. **Should structural spot-checks parse `lspci`/`lspci_t`/`lspci_v`/`lsusb` programmatically, or reference them only as human documentation while hand-writing assertions?**
   - What we know: `09-CONTEXT.md`'s the agent's Discretion explicitly permits either.
   - What's unclear: Whether programmatic parsing would add disproportionate complexity for a one-time structural check.
   - Recommendation: Use them as human-reference documentation (as done in this research) to hand-derive assertions like "hostpci0/1/2 all present, 3 total hostpci devices" and "usb0-usb5, 6 total usb devices" — writing a throwaway `lspci` parser for a 3-file test suite is disproportionate effort; this matches the "either is acceptable" guidance.

## Environment Availability

Skipped — this phase has no external tool/service/runtime dependencies beyond the Rust toolchain already in use throughout this project (confirmed via `cargo test` executing successfully in this research session). No QEMU/swtpm/UI-client binaries are spawned (D-01, D-02).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` via `cargo test` (integration tests under `tests/`) |
| Config file | None — no `pytest.ini`/`jest.config.*` equivalent; Cargo's own test harness |
| Quick run command | `cargo test --test round_trip_verification` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| QEMU-04 | felucia/108 full 4-stage round trip completes without error, cmdline passes structural spot-checks | integration | `cargo test --test round_trip_verification felucia -- --nocapture` | ❌ Wave 0 — new file |
| QEMU-04 | coruscant/501 full 4-stage round trip completes without error, cmdline passes structural spot-checks (incl. hostpci/scsi/numa paths) | integration | `cargo test --test round_trip_verification coruscant -- --nocapture` | ❌ Wave 0 — new file |
| QEMU-04 | zbp-server-mh2/301 full 4-stage round trip completes without error, cmdline passes structural spot-checks (incl. usb vendor:product-ID path) | integration | `cargo test --test round_trip_verification zbp -- --nocapture` | ❌ Wave 0 — new file |

### Sampling Rate
- **Per task commit:** `cargo test --test round_trip_verification`
- **Per wave merge:** `cargo test` (full suite — currently 148 tests per STATE.md, expect low-teens net-new)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `tests/round_trip_verification.rs` — new file covering QEMU-04 across all three corpus configs
- [ ] `src/config/qemu/handlers/usb.rs` — needs the vendor:product-ID fix (Pitfall 2) before zbp-server-mh2/301's test can pass; this is production code, not test infrastructure, but is a hard prerequisite for Wave 0 to be closeable
- [ ] No new shared fixtures/conftest-equivalent needed — per-file `load_corpus()`-style helpers duplicated per existing established pattern (see Don't Hand-Roll)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | No | Phase touches no auth surface |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | Yes (marginal) | Corpus `.conf` files are trusted, checked-in fixtures, not untrusted runtime input — the existing `ProxmoxVmConf::from_str`/`ProxmoxStorageConf::from_str` parsers (already covering malformed-input handling from Phase 3) are exercised as-is; this phase adds no new input-parsing surface, only new call sites into already-validated parsers |
| V6 Cryptography | No | N/A — no secrets, keys, or crypto touched by this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Panic-as-DoS on malformed/unexpected sub-option values (the exact `usb.rs` `.expect()` panic found in this research) | Denial of Service | Convert `.expect()`-style panics on data-shape assumptions into typed `thiserror` error variants where the input in question could plausibly come from untrusted/real-world Proxmox exports (as it does here) rather than purely programmer-supplied test data — the project's own `RUNT-09`/"thiserror from day one" decision already establishes this pattern; the `usb.rs` fix in this phase should ideally follow it (handle the unrecognized-form case as a typed error rather than a second unconditional `.expect()`), though `09-CONTEXT.md` scopes this phase to structural spot-checks, so the minimum bar is "supports both known forms without panicking," with graceful-error-on-unknown-form as a nice-to-have, not a hard requirement |

## Sources

### Primary (HIGH confidence — verified by direct code inspection/execution in this session)
- `src/config/proxmox/importer.rs` — EfiDisk gap, hostpci base_bdf/function logic, SCSI-controller-type non-effect, unknown-key silent-ignore behavior
- `src/config/proxmox/conf.rs` — field parsing coverage (scsihw, numa, serial, unused, hugepages, balloon, spice_enhancements — all confirmed present/absent in the parser)
- `src/config/proxmox/parser.rs` — `parse_usb_raw` accepting both bus-port and vendor:product-ID forms (confirmed via its own existing test `test_parse_usb_raw_vid_pid`)
- `src/config/qemu/handlers/usb.rs` — the `.expect()`-panic on vendor:product-ID resource strings, confirmed by direct Rust execution (`"0451:16a0".split_once('-')` → `None`)
- `src/config/qemu.rs`, `src/config/qemu/handlers/root.rs` — EfiDisk `.expect()`-panic mechanism and its documented pre-existing-gap status
- `tests/qemu_cmdline.rs`, `tests/yaml_round_trip.rs`, `tests/proxmox_import.rs` — existing test conventions, helper patterns, and the RawArgs/ordering assertion style to reuse
- `input/felucia/108.conf`, `input/coruscant/501.conf`, `input/zbp-server-mh2/301.conf` and their `storage.cfg` files — full device-key inventory and storage-pool cross-check
- `input/coruscant/501.lspci.txt`, `501.lspci_t.txt`, `501.lspci_v.txt` — hostpci real-hardware cross-reference (Fresco Logic USB controller, AMD Radeon RX 570 dual-function GPU, Intel X550 VF Ethernet)
- `input/zbp-server-mh2/301.lspci`, `301.lspci_t.txt`, `301.lsusb` — hostpci/usb real-hardware cross-reference (confirms `0403:6001`/`0451:16a0` are real connected USB devices, not corpus typos)
- Direct execution: `rustc`/run of a standalone snippet reproducing `usb.rs`'s `split_once('-')` call against `"0451:16a0"`, confirming the panic condition
- `cargo test --test qemu_cmdline` — confirmed existing 5 tests pass, confirming this repo's test harness/build is currently green

### Secondary (MEDIUM confidence)
- `.planning/phases/07-qemu-cmdline/07-RESEARCH.md` (Pitfall 4, HostPci multifunction) — read to confirm x-vga-driven multi-function behavior is intentional design, not a gap, for coruscant's single-function hostpci1/2 entries

### Tertiary (LOW confidence)
- QEMU `-device usb-host,vendorid=,productid=` exact flag syntax (Code Examples / Assumptions Log A1) — based on training knowledge of QEMU's long-stable `usb-host` device model, not verified against a fetched QEMU doc in this session (no external doc-lookup tooling was invoked for this phase, consistent with `.planning/config.json`'s all-external-search-providers-disabled configuration); flagged in Assumptions Log for executor verification at implementation time.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new libraries; existing `cargo test` harness confirmed working
- Architecture: HIGH — full four-stage chain already exists and independently tested per stage; this phase only composes it
- Pitfalls: HIGH for Pitfall 1 (EfiDisk) and Pitfall 3 (scsihw) — directly confirmed by reading existing, already-passing test code and importer source; HIGH for Pitfall 2 (usb vendor:product-ID) — confirmed by direct code execution reproducing the exact panic condition
- Security: MEDIUM — no auth/crypto surface touched; the one relevant finding (panic-as-DoS via `.expect()`) is a code-quality observation, not a verified exploit path, since these are trusted checked-in fixture files, not attacker-controlled input in production

**Research date:** 2026-07-29
**Valid until:** Effectively indefinite for the architectural/gap findings (tied to this repo's current source, not external library drift) — re-verify only if `src/config/qemu/handlers/usb.rs` or `src/config/proxmox/importer.rs` change materially before Phase 9 planning is acted upon.

## RESEARCH COMPLETE
