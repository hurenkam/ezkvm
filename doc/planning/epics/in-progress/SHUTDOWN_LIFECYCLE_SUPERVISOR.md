# Shutdown Lifecycle Supervisor

Status: in_progress

## Problem

ezkvm currently treats a live QEMU PID as "Running" even when the guest has
already issued a shutdown request and the runtime is still waiting for QEMU to
exit. That makes Windows shutdown look complete from the guest side while the
host still reports a healthy VM.

## Goal

Promote the existing QMP shutdown monitor into the VM-scoped lifecycle owner
for shutdown detection, teardown tracking, and status reconciliation.

## Scope

- Keep one shutdown observer per running VM.
- Record when the guest has already entered shutdown teardown.
- Teach `ezkvm status` to distinguish "guest shutdown in progress" from simple
  process liveness.
- Clean up the marker state when QEMU exits or the VM is stopped/force-killed.
- Add regression coverage for shutdown event handling and delayed exit paths.

## Acceptance Criteria

- A shutdown marker exists only while QEMU is still alive after guest shutdown
  detection.
- `ezkvm status` reports a stopping/shutdown-in-progress state when that marker
  is present.
- The marker is cleared on normal exit, stop, kill, and next start.
- Tests cover the marker lifecycle and the new lifecycle transition.

## Related Backlog Items

- E-04 Promote shutdown monitor into a VM-scoped lifecycle supervisor
- E-05 Reconcile guest shutdown state with QMP and PID state
- E-06 Add shutdown lifecycle regression tests and traces