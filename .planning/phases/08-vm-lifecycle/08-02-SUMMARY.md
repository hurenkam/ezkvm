---
phase: 08-vm-lifecycle
plan: 02
subsystem: start-ui-tpm
status: complete
completed: 2026-07-28
files_modified:
  - src/lifecycle/readiness.rs
  - src/lifecycle/start.rs
  - src/lifecycle/ui_client.rs
  - src/bin/fake_swtpm.rs
  - src/bin/fake_ui_client.rs
  - tests/vm_lifecycle.rs
  - tests/ui_client_mapping.rs
verification:
  - cargo build
  - cargo test --test vm_lifecycle -- --test-threads=1
  - cargo test --test ui_client_mapping
commits_created: 0
---

# Plan 08-02 Summary

Implemented the remaining `start` lifecycle behavior for TPM and UI-client flows.

## What changed

- Added `src/lifecycle/readiness.rs` with bounded Unix-socket readiness polling (`50ms` interval, timeout error).
- Extended `src/lifecycle/start.rs` to:
  - reject double-starts for live VMs,
  - start `swtpm` first for TPM-configured VMs,
  - poll the TPM socket for up to 8 seconds before launching qemu,
  - pass the TPM socket path into `QemuContext`,
  - kill orphaned `swtpm` and remove partial state on qemu spawn failure,
  - launch mapped UI clients after a fixed ~2 second delay,
  - warn (without failing) when the UI client cannot be launched,
  - persist `swtpm_pid` and `ui_client_pid` in `VmHandle`.
- Added `src/lifecycle/ui_client.rs` to map:
  - `Spice` -> `remote-viewer spice://host:port`
  - `Vnc` -> `remote-viewer vnc://host:port`
  - `LookingGlass` -> `looking-glass-client -f <ivshmem mem_path>`
  - headless/unsupported displays -> no client
- Added stub binaries:
  - `src/bin/fake_swtpm.rs`
  - `src/bin/fake_ui_client.rs`
- Expanded lifecycle integration coverage in `tests/vm_lifecycle.rs` and added `tests/ui_client_mapping.rs`.

## Verification

- `cargo build` ✅
- `cargo test --test vm_lifecycle -- --test-threads=1` ✅
- `cargo test --test ui_client_mapping` ✅

## Policy / scope notes

- No `git add` or `git commit` commands were run.
- `STATE.md` and `ROADMAP.md` were not modified.
- Work was limited to the plan's listed files plus this summary file.
