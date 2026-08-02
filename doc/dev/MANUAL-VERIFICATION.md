# MANUAL ONLY — NOT AUTOMATED, NOT CI-GATING

This checklist is a **manual** verification step for real hardware. It is **never** run by CI, does **not** gate automated phase completion, and cannot be reproduced in a container or standard CI runner because it requires:

- a discrete GPU at PCI `0000:03:00` (`hostpci0` from `input/felucia/108.conf`)
- a physical USB device at bus-port `1-2.2` (`usb0` from `input/felucia/108.conf`)

A failure in this checklist should be treated first as a host hardware / BIOS / IOMMU / passthrough setup problem, not as a packaging defect.

## Fixture identity

This procedure validates the `wakiza` VM represented by `input/felucia/108.conf` (`name: wakiza`) and its passthrough expectations:

- `hostpci0: 0000:03:00,pcie=1,x-vga=1`
- `usb0: host=1-2.2`

## Preconditions

1. Use a real Debian 13 or Ubuntu 26.04 host.
2. Ensure the physical GPU and USB device above are actually attached to that host.
3. Install the built ezkvm `.deb` on that host.
4. Put the already round-tripped ezkvm YAML for felucia/108 at:
   - `/etc/ezkvm/vm.d/wakiza.yaml`

   The YAML may come from Phase 9's round-trip output or from a fresh Proxmox import, as long as it still represents the same `wakiza` hardware mapping.

## Manual verification steps

1. Confirm the host is using the packaged install and the `wakiza.yaml` file is present.
2. Run:

   ```bash
   ezkvm start wakiza
   ```

3. Confirm the QEMU process starts successfully.
4. Confirm the GPU attach succeeds without `vfio` passthrough errors.
5. Confirm the USB attach succeeds without `usb-host` errors.
6. If the VM is configured for a display client, confirm the client connects successfully:
   - Looking Glass, or
   - `remote-viewer`
7. After boot verification, run:

   ```bash
   ezkvm stop wakiza
   ```

8. Confirm graceful shutdown completes.

## Interpretation of failures

If `ezkvm start wakiza` fails here because the GPU or USB device cannot be attached, the likely causes are missing passthrough prerequisites on the physical host: BIOS settings, IOMMU/VFIO configuration, device ownership, or absent hardware. That is a real-hardware validation issue, not evidence that the `.deb` packaging or automated CI verification is wrong.
