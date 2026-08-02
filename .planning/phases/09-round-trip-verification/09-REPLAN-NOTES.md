# Phase 9 Replan Notes (2026-07-29)

**Why this replan happened:** Phase 9 was originally planned (3 plans: 09-01, 09-02, 09-03) before
Phase 8.1 ("USB & SCSI Schema Extension") was inserted and executed. Phase 8.1 has now landed
(verified: 7/7 must-haves, 165/165 tests passing) and changed the ground truth two of Phase 9's
plans were written against. This replan reconciles Phase 9 with that new baseline. No new scope was
added; nothing was added beyond what Phase 8.1 made newly possible to assert.

## What changed in the source code (Phase 8.1, already shipped — not re-implemented here)

1. **USB host identity** is now a typed `UsbHostIdentity` enum (`BusPort{bus,port}` /
   `VendorProduct{vendor_id,product_id}`) in `src/runtime/devices/usb_generic.rs`, parsed once at
   the Proxmox import boundary (`src/config/proxmox/importer.rs`). `src/config/qemu/handlers/usb.rs`'s
   `emit_usb_device` exhaustively matches both variants — no panic path remains for
   vendor:product-ID passthrough syntax (`host=0451:16a0`, as used by real corpus data
   `zbp-server-mh2/301.conf`'s `usb4`/`usb5`).
2. **`scsihw`** (`pvscsi` / `virtio-scsi-pci` / `virtio-scsi-single`) is now fully consumed by
   `ProxmoxImporter`, not parsed-and-discarded. `GenericScsiController{controller_type}` covers the
   shared-bus pvscsi/virtio-scsi-pci case (`src/runtime/devices/pvscsi.rs`); `VirtioScsiSingleDisk`
   (one instance per Proxmox `scsiN` index, no shared bus) covers virtio-scsi-single. The QEMU
   emitter (`src/config/qemu/handlers/pcie.rs`) emits the corresponding `-device pvscsi` /
   `-device virtio-scsi-pci` controller string per corpus file's actual declared `scsihw` value.

Corpus files' actual `scsihw` values (confirmed by reading the `.conf` files directly):

| Corpus | `scsihw` | Emitted controller shape |
|--------|----------|---------------------------|
| `felucia/108.conf` | `pvscsi` | One shared `GenericScsiController` — `-device pvscsi,id=scsihw0,...` |
| `coruscant/501.conf` | `virtio-scsi-pci` | One shared `GenericScsiController` — `-device virtio-scsi-pci,id=scsihw0,...` |
| `zbp-server-mh2/301.conf` | `virtio-scsi-single` | 4 independent `VirtioScsiSingleDisk` instances, each its own `-device virtio-scsi-pci,id=scsihw{N},...` controller (N = Proxmox `scsiN` index) |

## What was changed in this replan

### 1. `09-CONTEXT.md`
Added a "Baseline update: Phase 8.1 landed" addendum under `<decisions>` correcting the two stale
technical assumptions above. D-01 through D-05 (scope, corpus selection, verification method,
stale-reference-file exclusion) are untouched — none of them assumed the now-corrected facts, so
none needed relitigating.

### 2. `09-02-PLAN.md` — DELETED
Its entire purpose was to fix the vendor:product-ID USB passthrough panic in
`src/config/qemu/handlers/usb.rs` via a `resource.contains(':')` string-split hack in the emitter.
That fix already exists in production code, done differently and more thoroughly (a proper typed
`UsbHostIdentity` enum threaded through Runtime, ezkvm YAML schema, the Proxmox importer, and the
QEMU emitter — not a point-fix confined to the emitter). Re-applying 09-02's planned change would
be redundant at best and would regress the typed representation at worst. Deleted rather than
marked SUPERSEDED since it is fully redundant, not partially useful.

### 3. `09-01-PLAN.md` — revised minimally
- Removed the stale "zbp-server-mh2/301's USB issue, fixed in Plan 09-02" phrasing from the
  objective's Purpose line (Plan 09-02 no longer exists; the fix is Phase 8.1's, already landed).
- Added a new must-have truth and a new felucia assertion: the generated cmdline must contain the
  `pvscsi`-specific shared SCSI controller device string (felucia's actual `scsihw: pvscsi` value),
  now that Phase 8.1 makes that controller-type-specific assertion possible and meaningful.
- Updated the stale "148 tests" pre-phase baseline reference to 165 (Phase 8.1's landed count per
  STATE.md).
- No changes to `depends_on`, wave, or the `load_corpus()`/`runtime_for_cmdline()`/`make_ctx()`
  helper contract — Plan 09-03 still reuses these verbatim.

### 4. `09-03-PLAN.md` — rewritten
- `depends_on` changed from `["09-01", "09-02"]` to `["09-01"]` — the dropped 09-02 is no longer a
  dependency.
- Removed Task 2's `<precondition>` gating on "Plan 09-02's emit_usb_device fix" — Phase 8.1's fix
  is already landed and verified before this plan runs; the precondition now instead notes that
  Phase 8.1's typed `UsbHostIdentity`/`VirtioScsiSingleDisk` are already present in `src/`.
- Replaced the must-have truth "Neither corpus test asserts a scsihw-controller-specific device
  string ... since scsihw is parsed but never consumed by the importer" (now FALSE) with a truth
  requiring each corpus test to assert the scsihw-controller-specific device string actually
  emitted for that corpus's declared `scsihw` value.
- Task 1 (coruscant/501): now asserts the exact
  `-device virtio-scsi-pci,id=scsihw0,bus=pci.0,addr=0x5` shared-controller line (previously
  explicitly forbade any `virtio-scsi-pci` substring assertion). Read `src/config/qemu/handlers/pcie.rs`
  to confirm the exact wiring before asserting.
- Task 2 (zbp-server-mh2/301): now asserts 4 per-instance `virtio-scsi-pci`-model controller lines
  (`id=scsihw0`..`id=scsihw3`, one per `VirtioScsiSingleDisk`), replacing the previous
  generic-presence-only assertion. USB vendor:product-ID assertions (usb4/usb5) are unchanged in
  substance (the emitted flags are the same `vendorid=`/`productid=` shape RESEARCH.md's sketch
  anticipated) but reframed as exercising Phase 8.1's already-landed, already-unit-tested code path
  rather than "requiring Plan 09-02's fix."
- Updated the STRIDE threat register: `T-09-03` reframed from "risk of 09-02 not landing first" to
  "low/accept — this path is already shipped and independently verified in Phase 8.1; this plan's
  test is a real-corpus regression guard, not a first exercise of unproven code."
- Updated the stale "148 tests" pre-phase baseline reference to 165, and the "4 net-new tests"
  count to 3 (1 felucia usb.rs unit test from the old 09-02 no longer exists).

### 5. `.planning/ROADMAP.md`
Updated Phase 9's `**Plans**` line (3 plans → 2 plans) and the plan checklist to remove the 09-02
entry and describe the corrected scope of 09-01/09-03.

### 6. `09-RESEARCH.md`
Left as historical record of the original research session (not rewritten), but flagged at the top
with a pointer to this file — two of its findings (Pitfall 2's "usb.rs vendor:product-ID gap must
be fixed in this phase" and Pitfall 3's "scsihw is parsed but never consumed, do not assert a
controller-specific string") are now stale/false as of Phase 8.1 landing. Downstream readers should
treat this REPLAN-NOTES.md file as the authoritative correction for those two points.

## What was NOT changed

- Wave structure: 09-01 remains wave 1 (no deps); 09-03 remains wave 2, now depending only on 09-01.
- Corpus selection (D-03: felucia/108, coruscant/501, zbp-server-mh2/301) — unchanged.
- Verification method (D-04: structural spot-checks, not exact-diff) — unchanged.
- No `src/` production code is modified by this replan — Phase 8.1's implementation is done,
  verified, and out of scope for further edits here.
- No new scope was added beyond making the now-possible scsihw-specific and USB assertions; no
  unrelated features, corpus files, or test cases were introduced.
