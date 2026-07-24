# Roadmap: ezkvm

## Overview

ezkvm converts Proxmox `.conf` and `storage.cfg` files into a typed in-memory Runtime model,
serializes that model to ezkvm YAML, and generates valid QEMU commandlines — with full
round-trip fidelity for real-world configurations. The v1 journey flows in nine phases of
strict dependency order: first a stable foundation of typed errors and device dispatch
(Phase 1), then the missing Runtime device types (Phase 2), then the Proxmox file parser
(Phase 3) and Proxmox→Runtime conversion (Phase 4), then the YAML schema extension (Phase 5)
and YAML↔Runtime wiring (Phase 6), then the QEMU commandline emitter (Phase 7), then
VM lifecycle management — start, stop, reset, and UI client launch (Phase 8) — and finally
an integration test that boots `input/felucia/108.conf` as a real VM (Phase 9). Each phase
delivers a coherent, testable capability whose output is the next phase's input.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Typed thiserror error enums, device_kind() dispatch, and RuntimeBuilder cleanup — unblocks all conversion phases (historically verified 2026-07-24)
- [x] **Phase 2: Runtime Model** - Implement EfiDisk, TpmState, HostPci, Ivshmem, AudioDevice, SpiceDisplay, and RawArgs as first-class Runtime types (historically verified 2026-07-24)
- [x] **Phase 3: Proxmox Parser** - Snapshot-aware .conf parser + storage.cfg parser with correct sub-option tokenization for colons and URL-encoded comments (completed 2026-07-23)
- [x] **Phase 4: Proxmox→Runtime** - TryFrom/ProxmoxImporter conversion: ProxmoxVmConf + ProxmoxStorageConf → fully populated Runtime (completed 2026-07-23)
- [x] **Phase 5: YAML Schema** - Extend ezkvm YAML schema to cover all seven v1 Runtime device types via saphyr (completed 2026-07-23)
- [x] **Phase 6: YAML↔Runtime** - TryFrom/Into impls for lossless Runtime ↔ ezkvm YAML round-trip (completed 2026-07-24)
- [ ] **Phase 7: QEMU Cmdline** - Segment-based QEMU commandline emitter with drive-before-device ordering and verbatim raw args
- [ ] **Phase 8: VM Lifecycle** - Start/stop/reset VM processes (qemu + swtpm), launch UI clients (Looking Glass / remote-viewer), QEMU monitor control
- [ ] **Phase 9: Round-Trip Verification** - Integration tests: felucia/108.conf → Runtime → ezkvm YAML → Runtime → QEMU cmdline → bootable VM

## Phase Details

### Phase 1: Foundation

**Goal**: Establish typed error enums and device_kind() dispatch across the codebase so every downstream conversion phase compiles against a stable, exhaustiveness-checked trait surface.
**Depends on**: Nothing (first phase)
**Requirements**: RUNT-08, RUNT-09
**Success Criteria** (what must be TRUE):

  1. All TryFrom impls in proxmox.rs and qemu.rs use typed `thiserror` error enums; zero `type Error = ()` remain anywhere in the codebase
  2. Every device trait (RootDevice, PcieDevice, ScsiDevice, SataDevice, UsbDevice, StorageDevice) exposes a `device_kind()` method returning a typed enum; all existing implementors compile with the new method
  3. No new `downcast_ref()` call sites are introduced by this phase or any later phase
  4. `RuntimeBuilder` builds with a plain `Vec<Arc<dyn RootDevice>>` (no Mutex); all existing builder-pattern tests pass

**Risks**:

- **Pitfall 19** (Unit error types — Pitfall 19): proxmox.rs and qemu.rs currently use `type Error = ()`; every new conversion impl added before fixing this inherits the pattern — fix all stubs in this phase before any new TryFrom code is written
- **Pitfall 18** (downcast_ref proliferation — Pitfall 18): 54 existing downcast sites will need updating as new device types are added; establishing device_kind() now prevents all future sprawl
- **Pitfall 21** (Mutex in RuntimeBuilder — Pitfall 21): Mutex-wrapped Vec is panic-prone when a prior builder call panics and poisons the lock; remove Mutex entirely since builders are single-threaded by design

**Plans**: TBD

Plans:

- [ ] 01-01: Add `thiserror = "2.0"` explicitly to Cargo.toml; replace `type Error = ()` in proxmox.rs and qemu.rs with named thiserror error enums (`ProxmoxConversionError`, `QemuConversionError`)
- [ ] 01-02: Add `fn device_kind(&self) -> DeviceKind` to RootDevice, PcieDevice, ScsiDevice, SataDevice, UsbDevice, and StorageDevice traits; implement for all existing concrete types (Memory, Q35Chipset, PvScsi, VirtioNet, Ssd, Hdd, Cdrom, GenericPciDevice, GenericUsbDevice)
- [ ] 01-03: Remove `Mutex` from `RuntimeBuilder`; replace with `Vec<Arc<dyn RootDevice>>`; verify all builder tests compile and pass

---

### Phase 2: Runtime Model

**Goal**: All seven v1 device types are first-class Runtime structs with correct trait implementations, giving every downstream conversion phase a complete, stable target API to write against.
**Depends on**: Phase 1
**Requirements**: RUNT-01, RUNT-02, RUNT-03, RUNT-04, RUNT-05, RUNT-06, RUNT-07
**Success Criteria** (what must be TRUE):

  1. `EfiDisk`, `TpmState`, `HostPci`, `Ivshmem`, `AudioDevice`, `SpiceDisplay`, and `RawArgs` exist as concrete types in `src/runtime/`
  2. Each new type implements the appropriate device trait(s) and returns a correct `device_kind()` variant
  3. A `Runtime` containing all seven new device types can be constructed programmatically via the builder API
  4. `cargo test` passes with no new `downcast_ref()` usages; no unsafe code added

**Risks**:

- **Pitfall 4** (Multi-function PCI): `HostPci` must store a **base BDF** (domain:bus:slot) plus a **functions: Vec<u8>** field — not a single resolved function — to support multi-function GPU expansion in PROX-06; design this in the struct from the start
- **Pitfall 7** (args passthrough): `RawArgs` must wrap an opaque `String`; do not attempt to tokenize or parse individual QEMU flags from the value — the internal cross-references (chardev ↔ virtserialport, object ↔ ivshmem-plain) cannot be safely reordered
- **Pitfall 9** (EFI size units): `EfiDisk` needs two size fields: `logical_size` (from .conf, human-readable string such as "4M") and `block_device_size_bytes` (u64, from QEMU cmd); do not convert between them — their relationship is opaque

**Plans**: TBD

Plans:

- [ ] 02-01: Implement `EfiDisk` (logical_size: String, block_device_size_bytes: Option<u64>, efitype: String, pre_enrolled_keys: bool, storage_volume: String) and `TpmState` (version: String, storage_volume: String) in `src/runtime/`
- [ ] 02-02: Implement `HostPci` (base_bdf: String, functions: Vec<u8>, pcie: bool, x_vga: bool, rombar: bool, romfile: Option<String>) and `Ivshmem` (size_mb: u64, name: String) in `src/runtime/`
- [ ] 02-03: Implement `AudioDevice` (device_type: String, driver: String), `SpiceDisplay` (gl: bool, rendernode: Option<String>, port: Option<u16>, clipboard: bool), and `RawArgs(String)` in `src/runtime/`
- [ ] 02-04: Register all seven types with appropriate device traits; add `device_kind()` variants; wire into Q35ChipsetBuilder (or as RootDevice as appropriate); write construction tests for each type

---

### Phase 3: Proxmox Parser

**Goal**: Raw Proxmox `.conf` and `storage.cfg` files parse into typed `ProxmoxVmConf` and `ProxmoxStorageConf` structs, correctly handling every known format quirk present in the corpus.
**Depends on**: Phase 1
**Requirements**: PROX-01, PROX-02, PROX-03, PROX-04
**Success Criteria** (what must be TRUE):

  1. `input/felucia/108.conf` parses to a `ProxmoxVmConf` where `cpu == "host"` and zero snapshot fields (e.g., `"x86-64-v2-AES"`, `snaptime`) leak into the active section
  2. `##args%3A` and `##hostpci1%3A` comment lines from corpus files produce zero runtime config entries in the parsed struct
  3. `net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1` parses with MAC address preserved verbatim as `"BC:24:11:3A:21:B7"`
  4. `input/felucia/storage.cfg` parses to a `ProxmoxStorageConf` containing a `vm1-pool` lvmthin entry with correct vgname

**Risks**:

- **Pitfall 1** (Snapshot contamination — FATAL): The parser MUST use a two-phase state machine: accumulate lines before the first `[...]` header as the active section; accumulate subsequent lines under their snapshot name; 108.conf has four snapshot sections each repeating all key names — a naive line-by-line parser produces garbage Runtime
- **Pitfall 2** (URL-encoded comments): Comment stripping MUST happen on raw bytes before any URL-decoding; `##key%3Avalue` lines decode to valid-looking `key: value` pairs that silently override live config
- **Pitfall 3** (Colon tokenization): A generic `split(',').split('=')` fails on MAC addresses, PCI BDFs (`0000:03:00`), and storage pool:volume references (`vm1-pool:vm-108-boot`); each device type requires its own per-grammar tokenizer
- **Pitfall 10** (storage.cfg required): Parser must fail with a descriptive error when storage.cfg is absent and the .conf contains pool:volume storage references

**Plans**: TBD

Plans:

- [x] 03-01: Implement `split_sections(input: &str)` two-phase state machine; produce active key-value pairs and a `BTreeMap<String, Vec<(String, String)>>` of snapshot sections; write test asserting 108.conf active section has `cpu: host` and no `snaptime`
- [ ] 03-02: Define `ProxmoxVmConf` struct (BTreeMap-indexed device fields: scsi, sata, ide, virtio, net, hostpci, usb, audio; single-entry Option for efidisk/tpmstate; scalar fields: memory, machine, bios, cpu, args, vga, etc.); define sub-structs (ProxmoxDiskConf, ProxmoxNetConf, ProxmoxHostPciConf, ProxmoxUsbConf, ProxmoxAudioConf, ProxmoxTpmConf, ProxmoxEfiDiskConf); expand into `src/config/proxmox/` subdirectory matching ezkvm layer layout
- [ ] 03-03: Implement per-device-type sub-option tokenizer using `splitn(2, ',')` for positional separation; write per-type unit tests covering: net0 (MAC colon), scsi0 (pool:volume colon), hostpci0 (PCI BDF colon), efidisk0 (pool:volume + option bag), usb0 (bus-port and VID:PID formats)
- [ ] 03-04: Implement `FromStr for ProxmoxStorageConf` (stanza format: type-id header, then indented key: value lines); write test parsing felucia/storage.cfg and asserting vm1-pool vgname

---

### Phase 4: Proxmox→Runtime

**Goal**: A `ProxmoxVmConf` + `ProxmoxStorageConf` converts to a fully populated `Runtime`, with all seven v1 device types represented, storage volumes resolved to host paths, and descriptive errors on failure.
**Depends on**: Phase 2, Phase 3
**Requirements**: PROX-05, PROX-06
**Success Criteria** (what must be TRUE):

  1. `input/felucia/108.conf` + `storage.cfg` imports to a `Runtime` containing `EfiDisk`, `TpmState`, `HostPci`, `AudioDevice`, and `RawArgs` with correct field values
  2. `hostpci0: 0000:03:00,pcie=1,x-vga=1` produces a `HostPci` with `base_bdf = "0000:03:00"` and `functions = [0, 1]` (two-function expansion for GPU + audio)
  3. Volume reference `vm1-pool:vm-108-boot` resolves to host path `/dev/vm1/vm-108-boot` via `ProxmoxStorageConf`
  4. Import returns a `ProxmoxImportError` with device context (not `()`) when a required field is absent or storage.cfg cannot resolve a volume reference

**Risks**:

- **Pitfall 4** (Multi-function PCI — PROX-06): `hostpci0: 0000:03:00` has an implied `.0`; the importer must expand to `functions: [0, 1]` by default (both GPU and audio functions) — treating it as single-function silently loses GPU HDMI audio
- **Pitfall 20** (Tuple TryFrom): Use a named `ProxmoxImporter { vm_conf, storage_conf }` struct with an `into_runtime(self) -> Result<Runtime, ProxmoxImportError>` method instead of `TryFrom<(ProxmoxVmConf, ProxmoxStorageConf)>`; named struct is easier to extend and composes cleanly with `?`
- **Pitfall 10** (storage.cfg resolver): Implement a `StorageResolver` that maps pool:volume to paths using storage type rules: `lvmthin` → `/dev/<vgname>/<volume>`, `dir` → `<path>/images/<vmid>/<volume>`

**Plans**: TBD

Plans:

- [x] 04-01: Define `ProxmoxImporter` struct and `ProxmoxImportError` thiserror enum; implement base conversion for Memory, Chipset type, and existing storage devices
- [ ] 04-02: Implement conversion for EfiDisk (both size fields), TpmState, HostPci (with multi-function expansion to functions: [0,1] for slots with companions), and AudioDevice from ProxmoxVmConf fields
- [ ] 04-03: Implement `StorageResolver`; resolve all pool:volume references in scsi/sata/ide/efidisk/tpmstate to host paths using ProxmoxStorageConf rules; propagate descriptive errors for unresolvable references
- [ ] 04-04: Write integration test: import felucia/108.conf + storage.cfg → assert all seven v1 device types present, HostPci has functions = [0, 1], and storage paths are resolved

---

### Phase 5: YAML Schema

**Goal**: The ezkvm YAML schema has schema types for all seven v1 Runtime device types, verified as correctly serializable and deserializable through the saphyr pipeline.
**Depends on**: Phase 2
**Requirements**: YAML-03
**Success Criteria** (what must be TRUE):

  1. Schema types for EfiDisk, TpmState, HostPci, Ivshmem, AudioDevice, SpiceDisplay, and RawArgs exist in `src/config/ezkvm/schema/`
  2. Each schema type serializes to YAML and deserializes back without data loss via saphyr (using `to_styled_compact_yaml()` pattern)
  3. `cargo test` passes all schema round-trip tests for the seven new types, including `HostPci` with a multi-value `functions` field

**Risks**:

- Saphyr custom path: The ezkvm project uses bespoke saphyr-based serde (not standard `serde_yaml`); new schema types must follow the existing compact_yaml pattern — do not introduce `serde_yaml` as a dependency
- YAML-03 scope: `HostPciSchema` must preserve `functions: Vec<u8>` to survive PROX-06 multi-function data through the YAML round-trip
- RawArgs whitespace: `RawArgsSchema` must store the verbatim string including internal whitespace and quoting; do not normalize or trim

**Plans**: TBD

Plans:

- [x] 05-01: Add `EfiDiskSchema` (logical_size, efitype, pre_enrolled_keys, storage_volume), `TpmStateSchema` (version, storage_volume), and `HostPciSchema` (base_bdf, functions: Vec<u8>, pcie, x_vga, rombar, romfile) to `src/config/ezkvm/schema/`; write YAML round-trip tests for each
- [ ] 05-02: Add `IvshmemSchema` (size_mb, name), `AudioDeviceSchema` (device_type, driver), `SpiceDisplaySchema` (gl, rendernode, port, clipboard), and `RawArgsSchema(String)` to schema; write YAML round-trip tests for each
- [ ] 05-03: Extend `VirtualMachineSchema` and `HostSchema` to include new device type fields; update existing ConfigSchema round-trip tests to cover the new fields

---

### Phase 6: YAML↔Runtime

**Goal**: The full Runtime ↔ ezkvm YAML ↔ Runtime pipeline is wired end-to-end; all seven v1 device types survive the round-trip with field-level fidelity.
**Depends on**: Phase 4, Phase 5
**Requirements**: YAML-01, YAML-02
**Success Criteria** (what must be TRUE):

  1. A `Runtime` imported from felucia/108.conf serializes to an ezkvm YAML file without errors
  2. The YAML file deserializes back to a `Runtime` with identical device count, device types, and all field values
  3. Spot-checked fields survive round-trip unchanged: `EfiDisk.logical_size`, `TpmState.version`, `HostPci.base_bdf`, `HostPci.functions`, and `RawArgs.0` (verbatim string)

**Risks**:

- YAML-02 lossless constraint: `RawArgs` must be stored as a verbatim YAML string scalar; any YAML emitter normalization (e.g., escaping the inner `-device` flags) must be confirmed to round-trip identically
- Type coverage gap: Both `EzkvmConfigSchema::try_from(Runtime)` and `Runtime::try_from(ConfigSchema)` must handle all seven new types; a missing arm in either direction causes silent data loss with no compile-time warning

**Plans**: TBD

Plans:

- [ ] 06-01: Implement `TryFrom<Runtime> for ConfigSchema` extensions (Runtime → YAML direction) for all seven new device types; follow existing handler pattern in `src/config/ezkvm/runtime/builder.rs`
- [ ] 06-02: Implement `TryFrom<ConfigSchema> for Runtime` extensions (YAML → Runtime direction) for all seven new device types; follow existing handler pattern in `src/config/ezkvm/runtime/parser.rs`
- [ ] 06-03: Write round-trip integration test: felucia/108.conf → ProxmoxImporter → Runtime → ConfigSchema → YAML string → ConfigSchema → Runtime; assert field-level equality for all seven v1 device types

---

### Phase 7: QEMU Cmdline

**Goal**: The QEMU commandline emitter produces a correctly ordered, valid argument list for all v1 Runtime device types, with raw args appended verbatim and all drive/netdev references preceding their dependent `-device` arguments.
**Depends on**: Phase 2
**Requirements**: QEMU-01, QEMU-02, QEMU-03
**Success Criteria** (what must be TRUE):

  1. `QemuCommandLine::try_from((runtime, context))` produces a commandline containing all seven v1 device types
  2. Every `-drive if=none,id=<X>,...` argument appears before its corresponding `-device ...,drive=<X>,...` argument in the output
  3. Every `-netdev ...,id=<N>,...` argument appears before its corresponding `-device virtio-net-pci,...,netdev=<N>,...` argument
  4. The `RawArgs` blob is appended verbatim after all structured arguments, with its internal token order preserved

**Risks**:

- **Pitfall 5** (Drive/device ordering — FATAL): Single-pass inline emission produces wrong ordering; the `QemuCommandLine` segment struct MUST collect all drives into `drives: Vec<String>` and all device args into `devices: Vec<String>` and emit drives section first — QEMU boots silently with wrong hardware if ordering is violated
- **Pitfall 6** (netdev ordering): Same constraint; netdev segment must precede devices segment in the fixed emission order
- **Pitfall 11** (bootindex): Derive `bootindex` values from the Runtime boot order list starting at 100; devices absent from the boot order omit `bootindex` entirely

**Plans**: TBD

Plans:

- [ ] 07-01: Implement `QemuCommandLine` segmented struct (`machine`, `firmware`, `drives`, `netdevs`, `chardevs`, `tpm`, `objects`, `devices`, `misc` fields, each `Vec<String>`); implement `Display` emitting segments in that fixed order; define `QemuContext` (vm_name, socket_paths, storage_paths)
- [ ] 07-02: Implement `TryFrom<(Runtime, QemuContext)> for QemuCommandLine`; emit EfiDisk (pflash drive pair), TpmState (chardev + tpmdev pair), HostPci (one vfio-pci device per function, multifunction=on for .0 when companion exists), AudioDevice, and network devices
- [ ] 07-03: Emit Ivshmem object+device pair; append `RawArgs` blob verbatim to `misc` segment (or as a final append after all segments); derive `bootindex` from Runtime boot order; handle `vga: none` as active `-vga none -nographic` flags
- [ ] 07-04: Write ordering validation test: tokenize emitted cmdline string; assert every `drive=<id>` reference position follows a `-drive ...,id=<id>,...` position; assert every `netdev=<id>` reference follows a `-netdev ...,id=<id>,...` position

---

### Phase 8: VM Lifecycle

**Goal**: ezkvm can start, stop, and reset a running VM by managing `qemu-system-x86_64` and `swtpm` child processes, communicate with QEMU via its monitor socket, and launch the configured UI client after VM start.
**Depends on**: Phase 7
**Requirements**: VMGR-01, VMGR-02, VMGR-03, VMGR-04, VMGR-05
**Success Criteria** (what must be TRUE):

  1. `ezkvm start <vm.yaml>` launches `qemu-system-x86_64` (with generated cmdline) and `swtpm` (when TPM is configured); both processes start without error
  2. After VM start, the configured UI client (Looking Glass client or `remote-viewer`) is launched automatically
  3. `ezkvm stop <vm>` sends `system_powerdown` via QEMU monitor and waits for process exit; `ezkvm kill <vm>` sends `quit` (with SIGTERM fallback)
  4. `ezkvm reset <vm>` sends `system_reset` via QEMU monitor
  5. QEMU monitor communication works over Unix socket

**Risks**:

- swtpm must be started before qemu and ready before qemu connects to its socket — use a readiness poll, not a fixed sleep
- QEMU monitor socket path must be stable and included in the generated cmdline (`-qmp unix:<path>,server,nowait`)
- UI client launch must be non-blocking (detached child process); failure to launch client must not kill the VM

**Plans**: TBD

Plans:

- [ ] 08-01: Define `VmHandle` struct tracking qemu PID, swtpm PID, monitor socket path, and UI client PID; implement `start()` launching swtpm (if needed) then qemu with generated cmdline
- [ ] 08-02: Implement QEMU monitor client (QMP JSON over Unix socket): `system_powerdown`, `quit`, `system_reset` commands; implement `stop()`, `kill()`, `reset()` on `VmHandle`
- [ ] 08-03: Implement UI client launch (Looking Glass / remote-viewer) as detached child; read client binary and args from ezkvm YAML `display` config
- [ ] 08-04: Write integration tests: start VM, verify qemu process running, send monitor command, verify process exits

---

### Phase 9: Round-Trip Verification

**Goal**: The complete felucia/108.conf pipeline — `.conf` + `storage.cfg` → Runtime → ezkvm YAML → Runtime → QEMU cmdline — produces a QEMU commandline that starts a working VM, verified by integration tests across multiple corpus files.
**Depends on**: Phase 6, Phase 7, Phase 8
**Requirements**: QEMU-04
**Success Criteria** (what must be TRUE):

  1. `input/felucia/108.conf` + `storage.cfg` imports, serializes to YAML, deserializes back, and generates a QEMU cmdline — all without errors
  2. The generated QEMU cmdline matches `108.ezkvm.qemu.cmd` reference output; any divergences are documented with rationale
  3. A VM launched with the generated cmdline reaches the QEMU monitor prompt without fatal startup errors
  4. Integration tests pass for at least two additional corpus files beyond felucia/108.conf (e.g., one coruscant and one zbp-server-mh2 config)

**Risks**:

- **Pitfall 7** (args cross-references — Pitfall 7): The `args` blob in 108.conf contains internally cross-referencing QEMU flags (`chardev=vdagent` ↔ `virtserialport`, `-object memory-backend-file,id=ivshmem0` ↔ `-device ivshmem-plain,memdev=ivshmem0`); emit the blob verbatim — any reordering of individual tokens within it breaks the VM
- Multi-corpus testing: Testing only 108.conf misses SCSI controller variant differences (pvscsi vs virtio-scsi-single) present in zbp-server-mh2 configs and NUMA configs on coruscant; these expose different Runtime paths

**Plans**: TBD

Plans:

- [ ] 08-01: Write end-to-end pipeline test for felucia/108.conf: import → Runtime → YAML → Runtime → QEMU cmdline; diff output against `108.ezkvm.qemu.cmd`; document each acceptable divergence in a `VERIFICATION.md` or test comment
- [ ] 08-02: Add end-to-end tests for two additional corpus configs; confirm `cargo test` passes across all integration tests
- [ ] 08-03: Fix any regressions discovered during multi-corpus testing; tag the passing test suite state as the v1 verification baseline

---

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9

Note: Phase 3 (Proxmox Parser) depends only on Phase 1 and may begin in parallel with Phase 2 (Runtime Model). Phase 5 (YAML Schema) depends only on Phase 2 and may begin in parallel with Phase 3. Phase 7 (QEMU Cmdline) depends only on Phase 2 and may begin in parallel with Phases 3–6.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/3 | Not started | - |
| 2. Runtime Model | 0/4 | Not started | - |
| 3. Proxmox Parser | 1/1 | Complete    | 2026-07-23 |
| 4. Proxmox→Runtime | 1/1 | Complete    | 2026-07-23 |
| 5. YAML Schema | 1/1 | Complete    | 2026-07-23 |
| 6. YAML↔Runtime | 0/3 | Not started | - |
| 7. QEMU Cmdline | 0/4 | Not started | - |
| 8. VM Lifecycle | 0/4 | Not started | - |
| 9. Round-Trip Verification | 0/3 | Not started | - |
