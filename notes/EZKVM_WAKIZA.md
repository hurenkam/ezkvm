# EZKVM Wakiza Boot Resolution Notes

 ## Objective
 Boot `examples/wakiza.yaml` in ezkvm with behavior matching Proxmox VM 108 as closely as possible, including:
 - successful Windows 11 boot
 - working GPU passthrough path
 - working Looking Glass connection
 - reachable guest networking and RDP

 ## Final Outcome
 `wakiza.yaml` now boots successfully in ezkvm.

 Observed working behavior:
 - physical monitor shows Proxmox splash, then Windows 11 boot and login screens
 - Looking Glass connects and shows guest output
 - guest is responsive
 - RDP works

 This means the major boot, display, TPM, and passthrough issues encountered during the debugging process were resolved.

 ## Most Important Learnings
 1. The black-screen failures were caused by multiple independent issues, not one single root cause.
 2. Command parity with Proxmox was necessary but not sufficient; several small incompatibilities each blocked progress at different stages.
 3. Runtime symptoms that looked visually similar had different causes:
 - some were display-device conflicts
 - some were TPM/NVRAM related firmware stalls
 - some were plain QEMU argument incompatibilities
 1. Proxmox parity had to be achieved at both the static command level and the runtime orchestration level.

 ## Fixes That Turned Out To Matter

 ### 1. Display path cleanup for non-passthrough boots
 - Removed conflicting virtual display setup (`virtio-gpu` plus implicit `qxl-vga`).
 - Ensured SPICE-backed boot path used a single coherent display device.

 Why it mattered:
 - eliminated an early black-screen condition unrelated to passthrough.

 ### 2. SPICE and auxiliary launch corrections
 - Mapped wildcard SPICE bind host to localhost for client connections.
 - Suppressed `remote-viewer` auto-launch for passthrough display mode.
 - Restricted auto Looking Glass launch to passthrough scenarios.
 - Added waiting for SPICE readiness before client launch.

 Why it mattered:
 - removed misleading client-side failures and race conditions.

 ### 3. CPU feature handling corrected
 - Hyper-V features were changed from invalid `+hv_*` form to valid bare `hv_*` form.
 - `kvm=off` was added to match Proxmox.
 - CPU feature validation was relaxed so bare Hyper-V feature names are accepted.

 Why it mattered:
 - fixed immediate QEMU startup failures like missing CPU property errors.

 ### 4. Q35 topology parity via `-readconfig`
 - Added support for `system.readconfig`.
 - Configured `/usr/share/qemu-server/pve-q35-4.0.cfg`.

 Why it mattered:
 - created the PCI bridge and root-port topology expected by the Proxmox-style bus assignments.
 - fixed errors such as missing `pci.0` and missing downstream buses.

 ### 5. SCSI argument ordering corrected
 - Emitted the SCSI controller before drives referencing `scsihw0.0`.

 Why it mattered:
 - fixed `Bus 'scsihw0.0' not found`.

 ### 6. VFIO argument compatibility fixes
 - Removed unsupported `pcie=1` from `vfio-pci`.
 - Switched `x-vga=1` to `x-vga=on` when tested.
 - Eventually removed `x_vga` again after the hardware/QEMU stack reported the device does not support it.

 Why it mattered:
 - fixed multiple direct QEMU startup errors in passthrough mode.

 Key lesson:
 - the working configuration for this GPU does **not** require `x_vga`.

 ### 7. TPM state backend support and swtpm launch parity
 - Added `state_backend_uri` support in config.
 - Normalized malformed `file://dev/...` URIs into `file:///dev/...`.
 - Aligned swtpm launch closer to Proxmox/manual form:
 - `--tpmstate backend-uri=...,mode=0600`
 - `--ctrl type=unixio,path=...,mode=0600`
 - `--pid file=...`
 - `--terminate`
 - `--log file=...,level=1`
 - `--daemon`
 - Added exact swtpm command logging for runtime comparison.

 Why it mattered:
 - resolved TPM initialization failures and brought swtpm behavior in line with the manually proven working command.

 ### 8. TPM socket path parity
 - Configured TPM chardev path to `/var/run/qemu-server/108.swtpm`.

 Why it mattered:
 - removed one more meaningful runtime difference relative to Proxmox.

 ### 9. UEFI pflash model and vars-drive parity
 - Switched UEFI handling to pflash code + vars drives.
 - Added explicit support for UEFI vars size.
 - Set `uefi_vars_size: 540672` to match Proxmox.

 Why it mattered:
 - reduced a boot-critical firmware/NVRAM delta.

 ### 10. NVRAM state was a real factor
 - Fresh OVMF VARS files changed behavior significantly during debugging.
 - Stale NVRAM state could produce black-screen behavior that looked like passthrough failure.

 Why it mattered:
 - confirmed that firmware state must be treated as an independent variable during debugging.

 ## Runtime Signals That Were Useful
 These signals were repeatedly helpful in distinguishing different failure classes.

 ### Firmware or pre-service stall indicators
 - `tap108i0` RX stayed at 0
 - no ARP/neighbor entry for VM MAC
 - no active peer on `108.qga`
 - guest looked busy in QEMU but never reached RDP or guest-agent availability

 Interpretation:
 - guest likely did not reach normal service startup

 ### QMP indicators
 Once QMP socket permissions were relaxed, `query-status` confirmed the VM could be genuinely `running` even while the screen stayed black.

 Interpretation:
 - “running” does not imply successful OS bring-up or successful passthrough handoff

 ### Proxmox reference stability mattered
 At one point Proxmox itself temporarily stopped booting the VM correctly.

 Interpretation:
 - host-level state can invalidate command-level conclusions
 - rebooting the host was necessary to restore a valid reference baseline

 ## Final Working Characteristics
 By the end of the debugging process, the working ezkvm setup includes:
 1. Proxmox-like machine type and Q35 readconfig topology
 2. Proxmox-like CPU flags including `hv_vendor_id=proxmox`
 3. pflash UEFI with explicit vars size
 4. TPM emulator using Proxmox TPM state volume via swtpm backend URI
 5. VFIO GPU/audio passthrough with explicit bus/address placement and multifunction
 6. USB passthrough enabled
 7. passthrough headless shape using `-vga none -nographic`
 8. Looking Glass/ivshmem working again once the rest of the boot path was correct

 ## What Was Proven False or Less Important
 1. `romfile` was not required to get this VM working in ezkvm, and Proxmox did not need it either.
 2. `x_vga` was not required and in this hardware/QEMU combination was actively incompatible.
 3. USB passthrough was not the main explanation for the earlier total boot failures, although it remains part of final Proxmox parity.
 4. Some black-screen states were not GPU handoff problems at all; they were TPM, NVRAM, or argument-shape issues.

 ## Recommended Practices For Future Similar Cases
 1. Keep one Proxmox reference command available and diff against it continuously.
 2. Treat firmware state, TPM state, and display path as separate variables.
 3. Use QMP early once socket permissions allow it.
 4. When a passthrough VM black-screens, check:
 - guest-agent peer state
 - tap RX counters
 - QMP `query-status`
 - QMP block state
 - exact swtpm launch command
 5. Reboot the host if Proxmox itself stops behaving normally; otherwise the reference becomes untrustworthy.

 ## Suggested Next Engineering Follow-Ups
 1. Keep `wakiza.yaml` close to the now-working state and avoid unnecessary cleanup until stability is proven over repeated reboots.
 2. Add more regression tests for:
 - TPM backend URI generation
 - pflash vars size
 - VFIO compatibility output without `pcie=1`
 - passthrough mode auxiliary-launch policy
 3. Consider adding optional debug commands to ezkvm for:
 - QMP status query
 - tap/qga quick health checks
 - rendered swtpm launch preview

 ## Short Resolution Summary
 The ezkvm failure was not caused by one missing passthrough flag. It was the cumulative effect of:
 - invalid CPU feature encoding
 - missing Q35 readconfig topology
 - bad SCSI argument ordering
 - incompatible VFIO option emission
 - TPM backend and URI issues
 - UEFI vars/pflash parity gaps
 - stale/variable NVRAM state

 After correcting those issues and converging on Proxmox runtime parity, `wakiza.yaml` now boots successfully in ezkvm with GPU passthrough, Looking Glass, and RDP working.
