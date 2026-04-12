# ezkvm Profile System Proposal

## Goal

Reduce per-VM YAML verbosity by allowing VM configs to reference reusable profiles such as windows_11 and gpu_passthrough.

## Problem Statement

Current VM config files often include many repeated details:
- Guest OS defaults (machine type, CPU feature sets, secure boot choices)
- Passthrough defaults (display disablement, input devices, ivshmem)
- Reused device model choices and policy flags

This makes configs hard to read and maintain.

## Proposed User Experience

### VM file usage

A VM config can declare profile references:

```yaml
name: "wakiza"
backend: "qemu"
profiles:
  - "windows_11"
  - "gpu_passthrough"

system:
  memory: 16384
  vcpus: 8

boot:
  uefi_code: "/usr/share/pve-edk2-firmware/OVMF_CODE_4M.secboot.fd"
  uefi_vars: "/dev/vm1/vm-108-efidisk"

hostpci:
  - id: "hostpci0.0"
    device: "0000:03:00.0"
  - id: "hostpci0.1"
    device: "0000:03:00.1"
```

### Profile directory layout

Profiles live as separate YAML files in a profile directory.
Default directory:

```text
/etc/ezkvm/profiles.d
```

Example files:

```text
/etc/ezkvm/profiles.d/windows_11.yaml
/etc/ezkvm/profiles.d/gpu_passthrough.yaml
```

Example profile file content:

```yaml
system:
  architecture: "x86_64"
  machine: "pc-q35-8.1+pve0"
  cpu_model: "host"
  cpu_features:
    - name: "hv_relaxed"
    - name: "hv_time"
boot:
  firmware: "uefi"
  secure_boot: true
spice:
  enabled: true
  vdagent: true
```

## Resolution Rules

Apply sources in this order:
1. Empty base config
2. Referenced profiles in listed order
3. VM file content

Result: the VM file always wins on conflicts.

### Merge semantics (MVP)

- Scalar values: replace
- Maps: deep merge
- Lists: replace entirely

This is simple, deterministic, and low risk.

### Merge semantics (Phase 2, optional)

For selected object lists, merge by id:
- devices.drives
- devices.networks
- hostpci
- usb_devices
- scsi_controllers
- xhci_controllers
- audio_devices

For list-like flags/options, append unique values:
- system.cpu_features
- system.machine_options
- options.global_options

## Integration Plan

### 1) Schema additions

Add to VM config parsing layer:
- Optional profiles: Vec<String>

Add to central config:
- locations.profile_dir: Option<String>

Default when unset:
- /etc/ezkvm/profiles.d

### 2) Loader flow

In VM load path:
1. Read VM YAML as raw YAML value
2. Read central config
3. Resolve profile directory from locations.profile_dir or default /etc/ezkvm/profiles.d
4. Load referenced profile files by name (for example windows_11 -> windows_11.yaml)
5. Merge profile values + VM value
6. Deserialize merged result into VmConfig
7. Run existing validation

### 3) Validation behavior

Keep current VmConfig validation mostly unchanged.
Add small pre-validation checks:
- Unknown profile name -> explicit error
- Non-map profile value -> explicit error
- Missing/unreadable profile file -> explicit error with attempted path
- Cycles only relevant if profile inheritance is later added

### 4) CLI and debug visibility

Add one of:
- ezkvm config resolve <vm.yaml>
- ezkvm start <vm.yaml> --show-resolved-config

This prints the final merged config for transparency and troubleshooting.

## Backward Compatibility

- Existing VM YAML without profiles continues to work unchanged.
- Existing validation and runtime behavior are preserved after merge.
- Profile support is additive.

## Example Refactor for wakiza

Once profiles exist, wakiza.yaml can shrink to mostly VM-specific data:
- name and backend
- memory and vcpus overrides
- storage paths
- network tap details and MAC
- hostpci BDF addresses
- UUIDs and socket paths

Everything else can come from windows_11 + gpu_passthrough defaults.

## Risks and Mitigations

### Risk: confusing override behavior
Mitigation:
- Fixed merge order and documented precedence
- Add resolved-config output command

### Risk: list replacement surprises
Mitigation:
- Document MVP list behavior clearly
- Add phase 2 id-based merge only where safe

### Risk: profile drift over time
Mitigation:
- Keep profile files in a dedicated directory with code review
- Add tests for important profile combinations

## Testing Plan

Unit tests for resolver:
- Single profile merge
- Multiple profile order precedence
- VM overrides profile values
- Missing profile error
- Non-map profile error
- List replace behavior (MVP)

Integration tests:
- Existing non-profile config still parses and runs
- Profile-based config resolves to expected VmConfig

## Suggested Rollout

1. Implement MVP profile references with list-replace semantics
2. Add resolved-config visibility command/flag
3. Migrate one example VM (wakiza) to prove reduced verbosity
4. Optionally implement id-based list merge for selected arrays

## Summary

A profile layering model gives ezkvm the biggest usability win with minimal architectural churn:
- shorter VM YAML files
- reusable defaults for common VM classes
- preserved compatibility with current loading and validation pipeline
- clear, deterministic conflict resolution
