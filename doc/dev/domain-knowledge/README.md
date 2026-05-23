# Development Domain Knowledge

This area stores reusable, non-project-specific technical knowledge relevant to ezkvm.

Example domains:
- qemu and q35 behavior
- proxmox concepts and mappings
- host OS/runtime behavior
- generic virtualization networking and storage behavior

Transition reference:
- `doc/dev/domain-knowledge/qemu-bus-and-addr-assignment.md`

Maintenance guidance:
- Include metadata headers in each note: date, scope, and purpose.
- Add a short validation section that lists how findings were verified (for example: source files, command traces, or reproducible test steps).
- Keep normative rules in architecture/workflow docs; keep domain notes factual and reference-driven.