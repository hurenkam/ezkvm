# Improved Profiles Plan

This document outlines a more convenient profile system for ezkvm, based on two goals:

1. Profiles should provide defaults by policy, not by patching concrete devices by `id`.
2. Profiles should define placement policies for values such as `addr` and `scsi_id`, rather than forcing users to specify exact placements in each VM file.

## Current Friction

### 1. ID-based list patching is the wrong abstraction for defaults

Today, profile merging treats several lists as "patch by `id`" collections. That works when profiles are describing concrete devices, but it is awkward when the real intent is:

- all `scsi` `disk` drives should inherit a common default set
- all `ide` `cdrom` drives should inherit a different default set
- all `tap` networks with model `virtio-net-pci` should inherit another default set

For users, declaring a drive as:

```yaml
devices:
  drives:
    - { id: "scsi0", interface: "scsi", type: "disk", path: "/dev/vm1/vm-108-boot" }
```

is much more convenient than having to duplicate exact `cache`, `aio`, `controller`, `boot_index`, `bus`, or `scsi_id` values just so profiles can merge correctly.

### 2. Exact placement values do not scale well in profiles

Profiles currently encourage hard-coded placement details such as:

- `addr`
- `scsi_id`
- sometimes `bus`

This becomes fragile because profiles are describing implementation detail rather than intent. The better abstraction is:

- define which kinds of devices belong on which bus/controller
- define how placement numbers are assigned
- auto-fill missing values deterministically

## Proposed Direction

The profile system should move from "concrete object patching" to a two-part model:

1. VM config declares concrete instances.
2. Profiles declare reusable policies that apply defaults and placement rules.

That means the final config flow becomes:

1. Load and merge profile overlays and VM config.
2. Normalize legacy fields into structured internal representations.
3. Apply selector-based default policies to concrete devices.
4. Apply placement policies to fill missing `addr`, `scsi_id`, and related fields.
5. Validate uniqueness and conflicts.
6. Generate QEMU arguments.

## Proposed Network Schema

The current `devices.networks[].mode` field is a single string that mixes backend type and backend-specific arguments. That prevents policy matching and makes validation difficult.

### Current shape

```yaml
devices:
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      mode: "tap,ifname=tap108i0,script=/usr/libexec/qemu-server/pve-bridge,downscript=/usr/libexec/qemu-server/pve-bridgedown,vhost=on"
      mac: "BC:24:11:3A:21:B7"
```

### Proposed shape

Use a structured backend object that mirrors the QEMU split between `-netdev` and `-device`.

```yaml
devices:
  networks:
    - id: "net0"
      model: "virtio-net-pci"
      mac: "BC:24:11:3A:21:B7"
      backend:
        type: "tap"
        ifname: "tap108i0"
        script: "/usr/libexec/qemu-server/pve-bridge"
        downscript: "/usr/libexec/qemu-server/pve-bridgedown"
        vhost: true
```

### Recommended backend split

For future policy support, the backend should be represented with explicit fields rather than a free-form string.

Recommended common fields:

- `backend.type`: `user`, `tap`, `bridge`, `socket`, `vhost-user`, or other supported QEMU backend types
- `backend.ifname`: interface name for tap-style backends
- `backend.bridge`: host bridge name when a bridge-oriented flow is supported directly
- `backend.script`: helper script path
- `backend.downscript`: teardown helper path
- `backend.vhost`: boolean
- `backend.queues`: optional queue count
- `backend.hostfwd`: structured host forwarding rules for `user` mode
- `backend.listen`: listen address/path for socket-like backends
- `backend.connect`: remote address/path for socket-like backends

Device-facing fields should remain separate:

- `model`
- `mac`
- `rx_queue_size`
- `tx_queue_size`
- `boot_index`
- `bus`
- `addr`

### Why this split helps policies

With structured fields, network policies can match on real semantics instead of parsing strings. For example:

- all networks with `backend.type = tap`
- all networks with `model = virtio-net-pci`
- all networks with `backend.type = user`

That allows profile rules to supply defaults like `vhost`, queue sizes, helper paths, or placement guidance without requiring exact `id` matches.

## Proposed Profile Policy Model

Profiles should gain a dedicated `policies` section rather than overloading concrete device lists.

Example direction:

```yaml
policies:
  drives:
    - match:
        interface: "scsi"
        type: "disk"
      defaults:
        format: "raw"
        cache: "none"
        aio: "io_uring"
        detect_zeroes: "unmap"
        controller: "scsihw0"
      placement:
        scsi_id:
          scope: "controller"
          start: 0
          step: 1

    - match:
        interface: "ide"
        type: "cdrom"
      defaults:
        readonly: true
        bus: "ide.1"
      placement:
        unit:
          scope: "bus"
          start: 0
          step: 1

  networks:
    - match:
        model: "virtio-net-pci"
        backend_type: "tap"
      defaults:
        rx_queue_size: 1024
        tx_queue_size: 256
      placement:
        addr:
          scope: "bus"
          bus: "pci.0"
          start: "0x12"
          step: 1
```

### Match selectors

Selectors should be field-based and explicit. Initial selectors should include:

- drives:
  - `interface`
  - `type`
  - optional `controller`
- networks:
  - `model`
  - `backend_type`
- future device families:
  - device-specific fields relevant for defaults and placement

Selectors should be AND-based within one policy rule. Multiple rules may apply in profile order.

### Default application rules

Policy defaults should only fill fields that are missing in the concrete VM config item.

Precedence should be:

1. explicit VM config value
2. later profile policy default
3. earlier profile policy default
4. schema default

This preserves explicit user intent while making profiles useful as reusable templates.

## Proposed Placement Policy Model

Placement policy should be a separate pass after defaults are applied.

### Candidate auto-assigned fields

- `addr`
- `scsi_id`
- `unit`
- possibly `boot_index` in future, if desired

### Placement rule responsibilities

A placement rule should define:

- target field to assign
- scope used to maintain uniqueness
- optional required bus/controller
- start value
- increment step

Examples:

- `scsi_id` increments within a given controller
- `addr` increments within a given bus
- `unit` increments within `ide.1`

### Placement algorithm

For each device family:

1. Group concrete instances by placement scope.
2. Seed the used-value set with explicit values already present.
3. For each instance missing a placement field, select the matching placement policy.
4. Assign the next free value from `start`, incrementing by `step`.
5. Fail validation if no valid value is available or if explicit values collide.

This keeps assignments deterministic while allowing users to omit noisy placement data in common cases.

## Merge Behavior Changes

The current id-based merge strategy should be narrowed.

### Recommended direction

- Keep concrete per-VM device lists primarily append-oriented.
- Move reusable defaults out of those lists and into `policies`.
- Retain legacy id-based merge only as a compatibility path while migrating existing profiles.

This makes the mental model much clearer:

- VM file describes the devices that exist.
- Profiles describe how matching devices should behave.

## Backward Compatibility Strategy

To avoid a disruptive migration, implement this in stages.

### Stage 1: Structured network backend with compatibility parser

- Continue accepting the old `mode` string.
- Parse it into the new internal structured backend representation.
- Prefer the new YAML shape in docs and examples.
- Emit warnings when a legacy shape is used, once warning infrastructure is available.

### Stage 2: Policy engine alongside existing merge behavior

- Introduce `policies` as an additional profile capability.
- Apply policies after normal merge.
- Keep old id-based patch behavior working for existing profiles.

### Stage 3: Reduce reliance on id-based defaults

- Migrate built-in or example profiles to use policies.
- Update docs to recommend concrete-instance declaration plus policy defaults.
- Consider deprecating id-based patching for drives and networks once policy coverage is sufficient.

## Suggested Implementation Plan

### Phase 1: Network schema normalization

Files likely affected:

- `src/config/vm_schema/network.rs`
- `src/config/validation/devices/network.rs`
- QEMU network argument generation code
- `doc/user/CONFIG.md`

Tasks:

1. Add structured network backend types to the schema.
2. Preserve legacy `mode` parsing during transition.
3. Validate backend-specific fields explicitly.
4. Generate `-netdev` arguments from structured backend fields.

### Phase 2: Policy schema and loader support

Files likely affected:

- config schema modules for new policy types
- profile loading and merge modules
- docs and examples

Tasks:

1. Add `policies` schema for drives and networks.
2. Separate concrete config merging from policy collection.
3. Define deterministic precedence across multiple profiles.

### Phase 3: Policy application engine

Tasks:

1. Add selector matching for drives and networks.
2. Apply default values only when fields are missing.
3. Add placement assignment passes for `scsi_id`, `addr`, and `unit`.
4. Validate collisions and explicit-value conflicts.

### Phase 4: Migration and cleanup

Tasks:

1. Convert example profiles to policy-based defaults.
2. Update docs to prefer policy-based profiles over id-based patching.
3. Add regression coverage for mixed legacy and new-style profiles.

## Recommended Near-Term Decisions

These decisions should be made before implementation begins:

1. Whether `backend` should be a nested object or flattened `backend_type` plus backend-specific optional fields. Nested object is cleaner and better aligned with QEMU semantics.
2. Whether drive/network concrete lists should remain id-addressable for compatibility only, or continue as a first-class merge mechanism. Recommended: compatibility only.
3. Which device families should support placement policies in the first implementation. Recommended first wave: drives and networks.
4. Whether profile rules should support only exact equality matching initially. Recommended: yes, to keep the first version understandable and deterministic.

## Recommended Outcome

The end state should let users write concise VM configs that describe intent:

```yaml
devices:
  drives:
    - { id: "scsi0", interface: "scsi", type: "disk", path: "/dev/vm1/vm-108-boot" }
    - { id: "scsi1", interface: "scsi", type: "disk", path: "/dev/vm1/vm-108-tmp" }
    - { id: "ide0", interface: "ide", type: "cdrom" }

  networks:
    - id: "net0"
      model: "virtio-net-pci"
      backend:
        type: "tap"
        ifname: "tap108i0"
      mac: "BC:24:11:3A:21:B7"
```

and let profiles supply the repetitive and topology-specific details automatically.