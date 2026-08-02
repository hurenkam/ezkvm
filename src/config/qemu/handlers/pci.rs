use crate::config::qemu::QemuCommandLineBuilder;
use crate::runtime::{
    GenericPciDevice, GenericScsiController, PciBusDeviceKind, PciDevice, PciDeviceKind,
};

/// Dispatch over both `PciBusDeviceKind` variants a `Q35Chipset.pci_bus` entry can hold.
///
/// The `PvScsi` arm calls `handlers::pcie::emit_scsi_controller` directly rather than
/// reimplementing the controller+scsi_bus-recursion logic (D-01 "shared, not rebuilt"
/// pattern) — a `PvScsi` found on `pci_bus` emits identically to one found on `pcie_bus`.
/// `boot_order` is forwarded unmodified to `emit_scsi_controller` (Plan 07-04, D-07).
pub(crate) fn emit_pci_device(
    builder: &mut QemuCommandLineBuilder,
    device: &dyn PciDevice,
    boot_order: &[String],
) {
    match device.device_kind() {
        PciBusDeviceKind::GenericPci => {
            let generic = device.as_any().downcast_ref::<GenericPciDevice>().unwrap();
            builder.push_device(format!(
                "-device {}",
                match generic.kind() {
                    PciDeviceKind::NetworkController => "e1000",
                    PciDeviceKind::QxlGpu => "qxl-vga",
                    PciDeviceKind::Ac97 => "ac97",
                }
            ));
        }
        PciBusDeviceKind::PvScsi => {
            let scsi_controller = device
                .as_any()
                .downcast_ref::<GenericScsiController>()
                .unwrap();
            crate::config::qemu::handlers::pcie::emit_scsi_controller(
                builder,
                scsi_controller,
                boot_order,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{GenericScsiControllerBuilder, ScsiAddress, Ssd};
    use std::sync::Arc;

    #[test]
    fn test_07_03_generic_pci_ac97_emits_device() {
        let generic = GenericPciDevice::new(PciDeviceKind::Ac97);
        let mut builder = QemuCommandLineBuilder::new();
        emit_pci_device(&mut builder, &generic, &[]);
        let output = builder.build().to_string();

        assert!(output.contains("ac97"), "output was: {output}");
    }

    #[test]
    fn test_07_03_pvscsi_on_pci_bus_reuses_emit_pvscsi() {
        let scsi_controller = GenericScsiControllerBuilder::new()
            .with_scsi_device(
                Some(ScsiAddress::new(0, 0)),
                Arc::new(Ssd::new("/dev/vm1/vm-108-boot".to_string())),
            )
            .build();

        let mut builder = QemuCommandLineBuilder::new();
        emit_pci_device(&mut builder, &scsi_controller, &[]);
        let output = builder.build().to_string();

        assert!(output.contains("pvscsi,id=scsihw0"), "output was: {output}");
        assert!(output.contains("if=none,id=drive-scsi0"), "output was: {output}");
        assert!(output.contains("scsi-hd"), "output was: {output}");
        assert!(output.contains("drive=drive-scsi0"), "output was: {output}");
        assert!(output.contains("id=scsi0"), "output was: {output}");
    }
}
