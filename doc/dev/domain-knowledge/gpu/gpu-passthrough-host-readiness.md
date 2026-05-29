# GPU Passthrough Host Readiness

Date: 2026-05-28  
Scope: Host-side operational readiness for GPU passthrough workloads  
Audience: Operators preparing Linux hosts for stable passthrough into guests

## Purpose

Provide implementation-agnostic checks that reduce first-boot failures and unstable guest graphics behavior when assigning a physical GPU to a VM.

## Cross-References

- Requirements baseline: [Product Requirements](../../requirements/product-requirements.md)
  - FR-001 predictable runtime workflow
  - FR-006 validation before execution
  - FR-007 dry-run preview
  - NFR-003 reliable diagnostics
- Machine behavior: [QEMU Chipset Behavior](../qemu/qemu-chipset-behavior.md)
- Chipset context: [Q35 Chipset Domain](../q35/q35-chipset-domain.md)

## Must-Have Prerequisites

- Hardware IOMMU support enabled in firmware and active in host kernel/runtime.
- Target GPU and companion functions (often HDMI audio) isolated in a viable IOMMU grouping strategy.
- Host can bind passthrough target devices to a neutral/assignment-ready driver path.
- Guest machine layout supports PCIe topology consistent with passthrough expectations.
- Reliable fallback display/management path on host (integrated GPU, secondary GPU, or remote management).

## Optional But Recommended

- ACS/slot planning that avoids fragile multi-function sharing.
- CPU pinning and memory reservation tuned for latency-sensitive graphics workloads.
- ROM handling plan for GPUs that are sensitive to initialization order.
- Separate testing profile that boots guest with minimal devices before full workload profile.

## Validation Checks (Before Attach)

1. Confirm IOMMU is active and no critical initialization errors appear in host logs.
2. Confirm target GPU functions are all accounted for (graphics + audio + USB controller blocks if present).
3. Confirm target functions are not actively used by host compositor/display stack.
4. Run dry-run/effective-command preview and confirm explicit bus and address placement for passthrough devices.
5. Verify guest topology remains shallow and predictable (avoid unnecessary bridge depth).

## Validation Checks (After Attach)

1. Guest enumerates GPU and companion functions consistently across reboots.
2. Driver installation succeeds without repeated code/load failures.
3. No host-side device reset storms or repeated detach/reattach churn.
4. Workload smoke test passes (display output, render path, and sustained activity).
5. Recovery path is proven: host remains operable if guest fails to initialize GPU.

## Common Failure Signatures

- Guest driver code errors (for example, initialization blocked or device cannot start):
  - Common causes: incomplete function passthrough, reset limitations, topology instability.
- Black screen after guest boot with no remote render:
  - Common causes: wrong primary display routing, guest firmware mismatch, device not fully attached.
- Host loses console/display unexpectedly:
  - Common causes: host still dependent on target GPU.
- Guest boots once, then fails on next reboot:
  - Common causes: reset behavior issues, inconsistent bus/address assignment, firmware-state drift.

## Troubleshooting Cues

1. Start from minimal topology and explicitly place passthrough devices on stable PCIe paths.
2. Ensure all related functions are passed together when hardware requires pairing.
3. Compare effective arguments between successful and failing runs for ordering/address drift.
4. Test reset behavior through multiple cold boots, not only warm reboots.
5. If instability appears after adding bridges/devices, simplify toward a flatter Q35 tree.
6. Keep host rescue access available during all iterations.

## Must-Have vs Optional Decision Rule

- Must-have means required for safe, repeatable assignment and recovery.
- Optional means performance or convenience improvements that should not be treated as blockers for initial bring-up.

## Readiness Exit Criteria

- IOMMU and device isolation validated.
- Passthrough device set attaches deterministically.
- Guest driver stack loads and survives multiple reboot cycles.
- Host recovery path remains available during failures.
