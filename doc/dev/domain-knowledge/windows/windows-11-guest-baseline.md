# Windows 11 Guest Baseline (Q35-Oriented)

Date: 2026-05-28  
Scope: Practical baseline checks for Windows 11 guests on QEMU/KVM-style stacks  
Audience: Operators validating guest readiness and troubleshooting first-boot/runtime issues

## Purpose

Provide implementation-agnostic, operational guidance to reach a stable Windows 11 baseline before advanced features (GPU passthrough, Looking Glass, tuning).

## Cross-References

- Requirements baseline: [Product Requirements](../../requirements/product-requirements.md)
  - FR-001 predictable runtime workflow
  - FR-006 validation before execution
  - FR-007 dry-run preview
  - NFR-003 actionable error reporting
- Machine behavior: [QEMU Chipset Behavior](../qemu/qemu-chipset-behavior.md)
- Chipset context: [Q35 Chipset Domain](../q35/q35-chipset-domain.md)

## Must-Have Prerequisites

- CPU virtualization extensions enabled and usable by host kernel/hypervisor layer.
- UEFI-capable guest boot path.
- TPM 2.0-equivalent guest capability available (hardware, software TPM, or equivalent stack feature).
- Secure Boot-capable firmware path available when policy requires it.
- Q35-capable machine model path (or equivalent PCIe-first model).
- Storage and ISO media paths are readable and stable.

## Optional But Recommended

- Virtio-class storage/network drivers staged early to avoid migration friction.
- Time sync hardening (guest tools and host time discipline).
- Baseline snapshot immediately after successful OS + driver install.
- Dedicated recovery ISO attached for first-boot troubleshooting.

## Validation Checks (Before Install)

1. Confirm machine model and firmware intent match Windows 11 expectations (UEFI + TPM readiness).
2. Run dry-run/effective-command preview and verify deterministic machine/device arguments (FR-005, FR-007 alignment).
3. Validate boot order and media paths (installer media first, target disk writable).
4. Verify guest-visible vCPU and memory allocations are within tested host capacity.
5. Verify TPM backend health (service active, socket/path accessible, no stale lock state).

## Validation Checks (After Install)

1. Windows setup completes without bypass flags for TPM/Secure Boot policy.
2. Device Manager has no unknown core platform devices after baseline drivers.
3. Reboot cycle is stable across at least 3 cold boots.
4. Guest clock drift remains bounded after workload + reboot.
5. Logs/errors are captured for repeatability (host launch log + guest setup diagnostics).

## Common Failure Signatures

- Installer says this PC cannot run Windows 11:
  - Common causes: missing TPM signal, non-UEFI boot, Secure Boot policy mismatch.
- Boot loops after first reboot:
  - Common causes: storage-controller mismatch, unstable firmware vars, media order errors.
- Intermittent BSOD during driver phase:
  - Common causes: aggressive CPU feature exposure, unstable memory overcommit, conflicting virtual devices.
- Guest appears installed but key devices are missing:
  - Common causes: absent virtio/guest drivers or unexpected bus placement behavior.

## Troubleshooting Cues

1. Re-check machine type/version pinning and bus placement assumptions against [QEMU Chipset Behavior](../qemu/qemu-chipset-behavior.md).
2. Reduce complexity: single disk, single NIC, no passthrough devices; then reintroduce one change at a time.
3. Validate firmware variable persistence path and permissions.
4. Compare effective runtime arguments between good and failing boots for ordering/topology drift.
5. If TPM appears flaky, restart TPM backend and clear stale runtime artifacts before retest.
6. If install passes but runtime is unstable, verify PCIe hierarchy simplicity per [Q35 Chipset Domain](../q35/q35-chipset-domain.md).

## Minimal Baseline Exit Criteria

- Windows 11 installs without bypass hacks.
- Three consecutive reboot cycles are stable.
- Core devices enumerate cleanly.
- Launch arguments are deterministic across runs.
- Logs provide enough context to diagnose future regressions (NFR-005).
