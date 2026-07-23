# Phase 06 Research: YAML↔Runtime

## Existing conversion surface

- `TryFrom<Runtime> for ConfigSchema` in `src/config/ezkvm/runtime/parser.rs` currently handles memory and chipset buses but ignores root EFI, TPM, audio, and raw-args devices and synthesizes storage resource IDs instead of preserving resources.
- `Builder::build` in `src/config/ezkvm/runtime/builder.rs` returns `Result<Runtime, String>` and rebuilds empty storage resources, omits root devices, and rejects PCI HostPci. These are direct gaps against YAML-01/YAML-02.
- Phase 5 schema types already represent EFI fields, TPM version/resource, audio, raw args, HostPci functions, and memory resources.

## Required approach

1. Introduce a typed YAML-runtime conversion error and use it at both conversion boundaries.
2. Build host resources from Runtime before serializing device references; preserve storage paths rather than synthetic IDs.
3. Reconstruct every modeled root device and chipset device, including HostPci, EFI, TPM, audio, and raw args.
4. Apply schema defaults only to absent optional values; reject missing required references and unsupported variants at the first deterministic error.
5. Add a tracer round-trip test for the Felucia-derived Runtime, then expand to all v1 device fields and failure cases.

## Pitfalls

- `RawArgs` are opaque and must be emitted/restored verbatim.
- Resource IDs and device topology must remain semantically identical.
- YAML output formatting can normalize, but semantic fields cannot.
- Existing `String` errors violate the locked typed-error contract.
