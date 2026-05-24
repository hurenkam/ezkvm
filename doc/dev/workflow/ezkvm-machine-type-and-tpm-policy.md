# ezkvm Machine Type and TPM Policy

Date: 2026-05-23
Scope: ezkvm-specific policy and workflow expectations for machine type selection, topology parity, and swtpm integration
Purpose: Define project policy that does not belong in domain-knowledge notes.

## 1. Machine Type Contract

In ezkvm, machine type is part of the guest-visible runtime contract and must be treated as compatibility-critical.

- `q35` is handled as PCIe-centric topology intent.
- `i440fx` is handled as legacy PCI-centric topology intent.
- Machine version pinning should be preserved when reproducibility or migration compatibility is required.

## 2. QEMU Placement Defaults in ezkvm Workflows

ezkvm command generation must account for QEMU default bus and address behavior when fields are omitted.

- Omitted values still produce deterministic placement in QEMU.
- Import and replay workflows must preserve explicit topology when present.
- Compact output is acceptable only when runtime-equivalent placement is preserved.

## 3. Topology Parity Policy

For parity-sensitive workloads:

- Preserve explicit `bus`, `addr`, `chassis`, and `port` values from imported configuration.
- Do not flatten structured topologies unless runtime equivalence is demonstrated.
- Treat changes to bridge and root-port synthesis as compatibility-sensitive.

## 4. swtpm Integration Policy

swtpm integration is part of reproducible VM runtime state.

- Keep TPM argument generation and process wiring stable across import/export cycles.
- Treat TPM paths and command options as parity-relevant state.
- Validate TPM wiring together with machine type and bus topology.

## 5. Operational Validation Checklist

- Compare generated QEMU command lines against trusted captures.
- Confirm machine type/version stability across round trips.
- Confirm topology stability for guest-visible devices.
- Confirm TPM backend wiring remains functionally equivalent.

## Cross-References

- Domain background (Q35): `doc/dev/domain-knowledge/q35-chipset-domain.md`
- Domain background (i440fx): `doc/dev/domain-knowledge/i440fx-chipset-domain.md`
- QEMU behavior details: `doc/dev/domain-knowledge/qemu-chipset-behavior.md`