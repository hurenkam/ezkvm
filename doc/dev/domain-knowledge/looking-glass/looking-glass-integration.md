# Looking Glass Integration Notes

Date: 2026-05-28  
Scope: Operational integration guidance for low-latency guest display transport using Looking Glass-style workflows  
Audience: Operators integrating passthrough graphics with host-side interactive display

## Purpose

Provide implementation-agnostic integration checkpoints for combining a passthrough GPU guest with a host-side display client pipeline.

## Cross-References

- Requirements baseline: [Product Requirements](../../requirements/product-requirements.md)
  - FR-001 predictable runtime workflow
  - FR-006 validation before execution
  - FR-007 dry-run preview
  - NFR-003 actionable diagnostics
  - NFR-005 observability
- GPU readiness: [GPU Passthrough Host Readiness](../gpu/gpu-passthrough-host-readiness.md)
- Machine behavior: [QEMU Chipset Behavior](../qemu/qemu-chipset-behavior.md)
- Chipset context: [Q35 Chipset Domain](../q35/q35-chipset-domain.md)

## Must-Have Prerequisites

- Guest GPU passthrough path is already stable without Looking Glass.
- Shared memory transport path available with permissions suitable for both producer and consumer sides.
- Guest-side capture component and host-side client are version-compatible.
- Host input focus and display routing plan is defined (single-seat vs multi-seat expectations).

## Optional But Recommended

- Hugepage or memory-layout tuning for lower transport jitter.
- Explicit frame pacing settings for mixed-refresh environments.
- Dedicated startup ordering/check script for client launch after guest capture availability.
- Separate profile for fallback remote access when client transport is unavailable.

## Validation Checks (Before Runtime)

1. Confirm passthrough-only baseline is stable first.
2. Confirm shared-memory region can be created, opened, and cleaned up across restarts.
3. Confirm capture and client versions/protocol expectations align.
4. Verify dry-run/effective-command output includes required transport device/object arguments.
5. Verify host permissions/groups avoid running as privileged workaround.

## Validation Checks (Runtime)

1. Host client receives frames within expected startup window.
2. Input (keyboard/mouse) capture/release behavior is predictable.
3. Audio path (if used in adjacent stack) remains synchronized enough for target use.
4. Reconnect behavior works after guest reboot without manual cleanup every cycle.
5. Frame pacing remains stable under sustained workload, not only idle desktop.

## Common Failure Signatures

- Client opens but no frames arrive:
  - Common causes: shared-memory permission mismatch, capture component not initialized, startup ordering issue.
- Frames appear with heavy stutter/latency spikes:
  - Common causes: CPU contention, memory pressure, scheduling interference, mismatched pacing settings.
- Input capture behaves erratically:
  - Common causes: focus conflicts, host compositor shortcuts, multiple input hooks.
- Works once, fails on next launch:
  - Common causes: stale shared-memory objects, cleanup/race condition, guest component not auto-starting reliably.

## Troubleshooting Cues

1. Prove passthrough stability independently before debugging client transport.
2. Validate shared-memory lifecycle: create, use, teardown, recreate.
3. Log startup order timestamps for guest capture readiness vs host client launch.
4. Reduce system load and retest to separate transport issues from host contention.
5. Keep topology/device order stable; avoid simultaneous changes in GPU mapping and display transport.
6. If behavior regresses after machine-layout changes, compare with [QEMU Chipset Behavior](../qemu/qemu-chipset-behavior.md).

## Must-Have vs Optional Decision Rule

- Must-have items are required for deterministic connectivity and safe operation.
- Optional items improve smoothness/latency but should not gate first successful integration.

## Integration Exit Criteria

- Client receives stable frames after repeated guest restarts.
- Input path is predictable and recoverable.
- Failures leave actionable logs and clear recovery actions.
- Transport remains stable under representative workload.
