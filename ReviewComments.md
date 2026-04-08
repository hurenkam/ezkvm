# Review Status

Date: 2026-04-08  
Scope: Entire Rust source under src/  
Validation: cargo test (77 passed, 0 failed)

## Completed

- All checklist items 1-11 were implemented and validated.
- VNC handling is now explicit: configured `vnc.port` is treated as TCP port and converted to QEMU display number (`5900 -> :0`).
- Passthrough TPM support is implemented with QEMU args in `src/vm/config/system/tpm/pass_through_tpm.rs`.

## Remaining Decision

- [ ] Decide whether to require live integration validation (QEMU/QMP/QGA) in addition to unit tests.

## Overall Assessment

- Remaining uncertainty is integration-level behavior on real QEMU/QMP/QGA endpoints and host-specific environments.

## Config Syntax Gap Assessment

Date: 2026-04-08  
Scope: YAML config syntax coverage vs popular QEMU features

### Prioritized Implementation Checklist

P1 (High impact)

- [ ] Add typed CPU topology fields: threads, dies, clusters; add NUMA node mapping and distance controls.
- [ ] Add typed memory backend options: hugepages, prealloc, mem-path, backend kind, NUMA placement.
- [ ] Implement typed virtio balloon and virtio-rng support; wire `memory.balloon` to emitted QEMU args.
- [ ] Add typed storage performance/reliability options: aio (including io_uring/native), iothreads, throttling, serial/wwn, snapshot mode, error policy.
- [ ] Add additional typed network backends: user/slirp, socket/vhost-user, macvtap/macvlan, multiqueue options.

P2 (Medium impact)

- [ ] Add typed SPICE/VNC security and transport options: auth, TLS, websocket/password controls.
- [ ] Add typed boot policy controls: boot order, boot once, legacy/firmware policy toggles.
- [ ] Add typed PCIe topology controls: root ports, switch/slot planning, deterministic placement.
- [ ] Add typed QMP/monitor endpoint customization instead of fixed paths/options.

P3 (Documentation alignment)

- [ ] Update [doc/CONFIGURATION.md](doc/CONFIGURATION.md) to reflect implemented TPM passthrough support.
- [ ] Reconcile storage controller docs in [doc/CONFIGURATION.md](doc/CONFIGURATION.md) with current implementation (ide/sata/pvscsi vs documented virtio controller).

### Note

- Raw `extras` can still express many advanced QEMU flags until typed schema support is added.

