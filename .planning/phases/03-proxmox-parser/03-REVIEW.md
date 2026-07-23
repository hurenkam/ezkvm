---
phase: 03-proxmox-parser
reviewed: 2026-07-23T20:58:12Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src/config/proxmox.rs
  - src/config/proxmox/error.rs
  - src/config/proxmox/parser.rs
  - src/config/proxmox/conf.rs
  - src/config/proxmox/storage.rs
findings:
  critical: 1
  warning: 2
  info: 0
  total: 3
status: needs-fix
---

# Phase 03: Code Review Report — Proxmox Parser

**Reviewed:** 2026-07-23T20:58:12Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** needs-fix

## Post-review update

CR-01, WR-03, and WR-04 were resolved in `2760cce`
(`fix(proxmox-parser/03-02): preserve parse errors`): recognized malformed
fields now return `ProxmoxParseError`, boolean handling follows the plan, and
the missing parser assertions are covered. The remaining CR-02, WR-01, and
WR-02 findings concern `StorageResolver`, which is outside this parser phase's
planned API and should be addressed with the Proxmox-to-Runtime importer work.

## Summary

`cargo test -p ezkvm proxmox` passes (18/18 unit + 7/7 integration tests green), and the
headline PROX-01..04 success criteria (snapshot isolation, `##` comment skipping, colon-safe
MAC/BDF tokenization, LvmThin `vgname` resolution) all hold for the `input/felucia` fixture.
`split_sections()` and `parse_sub_options()` are correctly implemented per PLAN.md and match the
documented two-state machine and colon-safety design.

However, two high-confidence defects were found that are **not caught by the current test
suite** because the tests were written to match the code's (incorrect) behavior rather than
real-world Proxmox semantics or the documented error contract:

1. `ProxmoxVmConf::from_str` declares `type Err = ProxmoxParseError` but the implementation
   never returns `Err` — every parse failure (malformed index suffix, malformed sub-option,
   unparsable integer) is silently swallowed via `.ok()` / `if let Ok(...)`, causing devices or
   scalar fields to vanish from the result with zero diagnostic. `cargo build` independently
   confirms 3 of 4 `ProxmoxParseError` variants are dead code — proof no code path ever
   constructs them.
2. `StorageResolver::resolve()` for `StorageType::Dir` always builds
   `{path}/images/{vmid}/{volume}`, ignoring the storage entry's declared `content` type. This
   is provably wrong for the corpus: `input/felucia/storage.cfg`'s `local` entry declares
   `content iso,vztmpl` (no `images` support at all), yet every ISO-backed `ide2`/`ide0` disk in
   the corpus (`108.conf`, `105.conf`, `114.conf`, `193.conf`, `2000.conf`, `535.conf`) resolves
   through this exact code path and receives a path that does not correspond to where Proxmox
   actually stores ISO files (`{path}/template/iso/{filename}`, not `{path}/images/{vmid}/...`).

Both defects directly threaten the project's stated core value ("full round-trip fidelity for
real-world configs") and are corpus-verifiable today with `input/felucia` now in place.

## Critical Issues

### CR-01: `ProxmoxVmConf::from_str` never returns `Err` — all parse failures are silently discarded

**File:** `src/config/proxmox/conf.rs:97-207` (dispatch loop), cross-referenced with
`src/config/proxmox/error.rs:1-14`

**Issue:**
The `FromStr` impl for `ProxmoxVmConf` declares `type Err = ProxmoxParseError;`, but tracing
every branch of the dispatch loop shows no branch ever produces `Err`:

- Scalar numeric fields silently drop invalid data instead of propagating an error:
  ```rust
  } else if key == "memory" {
      conf.memory = value.parse::<u64>().ok();      // conf.rs:110
  } else if key == "cores" {
      conf.cores = value.parse::<u8>().ok();          // conf.rs:112
  } else if key == "sockets" {
      conf.sockets = value.parse::<u8>().ok();        // conf.rs:114
  ```
  A malformed `memory: 16GB-typo` line does not error — it silently produces
  `conf.memory == None`, which downstream (Phase 4 `ProxmoxImporter::into_runtime`) surfaces as
  `MissingMemory` — a misleading diagnostic that hides the real cause (a malformed value, not an
  absent field).

- All per-device index parsing and per-device sub-option parsing swallow errors identically,
  e.g.:
  ```rust
  } else if let Some(rest) = key.strip_prefix("scsi") {     // conf.rs:155
      if let Ok(idx) = rest.parse::<u8>() {
          if let Ok(disk) = parse_disk_raw(&value) {
              conf.scsi.insert(idx, disk);
          }
      }
  }
  ```
  The same double `if let Ok(...)` swallow pattern repeats for `sata`, `ide`, `virtio`, `net`,
  `hostpci`, `usb`, `serial`, and `tpmstate0` (conf.rs:149-202). If `parse_net_raw()` fails
  because no recognized NIC model token is present (a real, documented failure mode — see
  `parser.rs:129-133`, `InvalidSubOption { field: "net", .. }`), the entire `net0` device is
  dropped from `ProxmoxVmConf` with **no error, no log, no indication** that a NIC was present
  in the source file but failed to import.

- Independent proof this is dead functionality, not just theoretical: running
  `cargo build --lib --tests` emits:
  ```
  warning: variants `InvalidLine`, `InvalidSection`, and `MissingRequiredField` are never constructed
    --> src/config/proxmox/error.rs:4,7,13
  ```
  Three of the four documented `ProxmoxParseError` variants are unreachable in the entire
  codebase. The fourth (`InvalidSubOption`) *is* constructed by `parser.rs`'s per-device helpers,
  but every one of its call sites in `conf.rs` discards the `Result` with `if let Ok(...)`, so
  even that variant can never actually surface to a caller.

**Impact:** This directly contradicts the phase's own documented threat model
(`03-RESEARCH.md` §Common Pitfalls, `T-03-05`) and the project's core value of "full round-trip
fidelity" (`.planning/STATE.md`). A future Proxmox version, a hand-edited `.conf` file, or subtly
corrupted config will silently produce a `Runtime` **missing hardware the source file declared**
(a dropped GPU passthrough, a dropped disk, a dropped NIC) with the tool reporting success. This
is the same class of failure the project explicitly called out as unacceptable for snapshot
contamination ("Bleeding snapshot fields into the active config produces silently wrong VMs") —
here it is silent *device* loss instead of snapshot bleed, but the risk category (silently wrong
VM, no error surfaced) is identical.

**Fix:** Propagate parse errors instead of swallowing them. At minimum, numeric scalar fields and
per-device parses should use `?` instead of `.ok()`/`if let Ok`:
```rust
} else if key == "memory" {
    conf.memory = Some(value.parse::<u64>().map_err(|_| {
        ProxmoxParseError::InvalidSubOption { field: "memory".into(), raw: value.clone() }
    })?);
} else if let Some(rest) = key.strip_prefix("scsi") {
    let idx = rest.parse::<u8>().map_err(|_| {
        ProxmoxParseError::InvalidSubOption { field: key.clone(), raw: key.clone() }
    })?;
    conf.scsi.insert(idx, parse_disk_raw(&value)?);
}
```
If silent-skip-on-unknown-key is intentionally kept for forward compatibility (reasonable), that
tolerance should be scoped to *unrecognized keys only* — not to recognized keys with malformed
values. Add a regression test asserting `ProxmoxVmConf::from_str("net0: bridge=vmbr0\n")` (no
model token) returns `Err`, not `Ok(ProxmoxVmConf { net: {}, .. })`.

---

### CR-02: `StorageResolver::resolve()` produces incorrect filesystem paths for non-image content on `dir` storage

**File:** `src/config/proxmox/storage.rs:118-127`

**Issue:**
```rust
StorageType::Dir => {
    let path = entry.properties.get("path").ok_or_else(|| { ... })?;
    Ok(format!("{}/images/{}/{}", path, self.vmid, volume))   // line 125
}
```
This unconditionally assumes every `dir`-storage volume reference is a VM disk image living
under `{path}/images/{vmid}/{filename}`, regardless of the storage entry's declared `content`
types. It never inspects `entry.properties.get("content")`.

Corpus-verified failure case: `input/felucia/storage.cfg`:
```
dir: local
	path /var/lib/vz
	content iso,vztmpl
```
The `local` entry declares support for **only** `iso` and `vztmpl` content — it does not even
list `images` as a supported content type. Yet `input/felucia/108.conf:52` contains:
```
ide2: local:iso/virtio-win-0.1.248.iso,media=cdrom,size=715188K
```
Resolving `"local:iso/virtio-win-0.1.248.iso"` through `StorageResolver::resolve()` yields:
```
/var/lib/vz/images/108/iso/virtio-win-0.1.248.iso
```
The real file, per Proxmox's storage layout for `iso` content on `dir` storage, lives at:
```
/var/lib/vz/template/iso/virtio-win-0.1.248.iso
```
This is not an isolated case — every ISO-backed `ideN` device across the corpus
(`felucia/108.conf`, `coruscant/105.conf`, `coruscant/114.conf`, `coruscant/193.conf`,
`coruscant/2000.conf`, `coruscant/535.conf`) hits this same code path and receives a
non-existent path.

**Impact:** Any imported VM with a CD-ROM/ISO attached to `dir` storage will generate a QEMU
`-drive`/`-device` referencing a file that does not exist on the target host, causing boot
media or driver ISOs to silently fail to attach — directly undermining "round-trip fidelity for
real-world configs," the project's stated core value. This is corpus-verifiable today since
`input/felucia` now exists, and the bug reproduces on the very fixture this phase's own tests
were written against.

**Fix:** Branch on `entry.properties.get("content")` (or, more robustly, encode content-type in
`StorageType`/a separate field) and only apply the `images/{vmid}/` convention when the
referenced content is an actual VM disk image. ISO and template content should resolve to
`{path}/template/iso/{volume_suffix}` / `{path}/template/cache/{volume_suffix}` respectively
(stripping the redundant `iso/`/`vztmpl/` prefix already present in `volume` if the Proxmox
convention embeds it, or keeping it if the on-disk layout expects it — either way, `images/` and
`{vmid}/` must not be blindly prepended for non-image content). At minimum, add a corpus-derived
regression test:
```rust
#[test]
fn test_storage_resolver_dir_iso_content() {
    // uses input/felucia/storage.cfg + input/felucia/108.conf's ide2 volume
    let result = resolver.resolve("local:iso/virtio-win-0.1.248.iso").unwrap();
    assert_eq!(result, "/var/lib/vz/template/iso/virtio-win-0.1.248.iso");
}
```

## Warnings

### WR-01: Existing test masks CR-02 instead of catching it

**File:** `src/config/proxmox/storage.rs:178-181`

**Issue:**
```rust
#[test]
fn test_storage_resolver_dir_includes_vmid() {
    ...
    let result = resolver.resolve("local:iso/virtio-win.iso").unwrap();
    assert!(result.contains("/images/108/"), "path should include vmid");
}
```
This test asserts the resolver's actual (incorrect, per CR-02) output rather than the real
Proxmox on-disk convention for ISO content. It gives false confidence that `dir`-storage
resolution is correct, since the assertion was written to match the implementation rather than
to independently verify expected real-world behavior.

**Fix:** Replace with an assertion against the true expected path (see CR-02 fix), or explicitly
split into two tests — one for `images` content (VM disk) and one for `iso`/`vztmpl` content —
once CR-02 is fixed.

---

### WR-02: `mod tests` in storage.rs missing `#[cfg(test)]`, unlike every other test module in the reviewed files

**File:** `src/config/proxmox/storage.rs:136`

**Issue:** `conf.rs` and `parser.rs` both correctly gate their test modules with
`#[cfg(test)]`. `storage.rs` does not:
```rust
mod tests {          // storage.rs:136 — no #[cfg(test)] above this line
    use super::*;
    ...
}
```
Confirmed via `grep -n "cfg(test)\|mod tests" src/config/proxmox/storage.rs` — only one match
(`mod tests` itself), no `#[cfg(test)]` attribute present.

**Impact:** This test module (including its 5 `#[test]`-annotated functions, file-reading via
`CARGO_MANIFEST_DIR`, and `use super::*`) compiles into every non-test build (`cargo build`,
release binaries), unlike the sibling test modules in `conf.rs`/`parser.rs`. It is inert dead
code in production builds (nothing calls `#[test]` functions outside a test harness), but it
is inconsistent with the established convention in this same phase's other files and needlessly
grows the compiled artifact.

**Fix:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    ...
}
```

---

### WR-03: `numa`/`tablet` boolean parsing diverges from PLAN.md's specified rule

**File:** `src/config/proxmox/conf.rs:115-117`

**Issue:** PLAN.md Task 03-02-B explicitly specifies:
```
"numa"     → conf.numa = Some(&raw_val != "0")
"tablet"   → conf.tablet = Some(&raw_val != "0")
```
(truthy unless exactly `"0"`), but the implementation uses the inverse comparison:
```rust
} else if key == "numa" {
    conf.numa = Some(value == "1");        // conf.rs:115
} else if key == "tablet" {
    conf.tablet = Some(value == "1");      // conf.rs:117
```
(falsy unless exactly `"1"`). For the current corpus (values are always strictly `"0"` or
`"1"`), the two rules agree, so no test fails today. But the two rules diverge for any other
value (e.g. a hypothetical malformed or future Proxmox emission of `numa: true` or a stray
whitespace variant) — the PLAN's rule would treat it as `true`, the implementation treats it as
`false`. This is a silent behavioral deviation from the reviewed spec, not merely a style
preference.

**Fix:** Either align the implementation with PLAN.md (`value != "0"`) or update PLAN.md/decision
log to record the intentional deviation and rationale. Add a test with a non-`"0"`/`"1"` value
to lock in whichever behavior is chosen.

---

### WR-04: Documented PLAN.md test assertions for `ProxmoxVmConf::from_str` are not present in the actual test

**File:** `src/config/proxmox/conf.rs:210-233` (test `test_proxmox_vm_conf_from_108_conf`)

**Issue:** PLAN.md Task 03-02-B's Behavior/test section explicitly lists these required
assertions after parsing `108.conf`:
```
conf.memory == Some(16384)
conf.scsi.values().all(|d| !d.volume.contains("x86-64-v2-AES"))
```
Neither assertion appears in the actual `test_proxmox_vm_conf_from_108_conf` test. Given CR-01
(numeric parse failures are silently swallowed to `None`), the `conf.memory == Some(16384)`
assertion in particular is exactly the kind of check that would have caught a regression in the
`memory` numeric-parsing path — its absence is a real coverage gap, not a nitpick.

**Fix:** Add the two missing assertions from PLAN.md's documented Behavior section to close the
coverage gap:
```rust
assert_eq!(conf.memory, Some(16384), "memory should be 16384");
assert!(
    conf.scsi.values().all(|d| !d.volume.contains("x86-64-v2-AES")),
    "snapshot values must not leak into active scsi volumes"
);
```

## Info

### IN-01: `ProxmoxVmConf::from_str`'s `Result` return type is misleading given current behavior

**File:** `src/config/proxmox/conf.rs:99-101`

**Issue:** `fn from_str(s: &str) -> Result<Self, Self::Err>` currently has exactly one `return`
path (`Ok(conf)` at the end) — the function is, as written, infallible. Contrast with
`storage.rs`'s `FromStr for ProxmoxStorageConf`, which honestly declares
`type Err = std::convert::Infallible` for the same "never actually errors" situation. This
inconsistency within the same phase's files is a readability/API-honesty issue: a caller reading
`ProxmoxVmConf::from_str(..)?` reasonably expects malformed input to be rejected, when in the
current implementation it never is (see CR-01).

**Fix:** Once CR-01 is addressed (errors are actually propagated), this signature becomes
accurate and this note is moot. If CR-01 is deliberately deferred, consider using
`std::convert::Infallible` in the interim to make the current (lack of) error behavior explicit
rather than implied.

---

_Reviewed: 2026-07-23T20:58:12Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
