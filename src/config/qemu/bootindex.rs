//! Device-id reconstruction + `bootindex` lookup, closing CONTEXT.md's D-07 gap.
//!
//! `Runtime::boot_order()` (added in Plan 07-01) stores Proxmox-style device-id label
//! strings (e.g. `"scsi0"`, `"ide2"`, `"net0"`) parsed from `boot: order=a;b;c`. Runtime
//! itself addresses devices by typed bus-address tuples (`ScsiAddress { target, lun }`,
//! `IdeAddress { channel, device }`, ...), not by these label strings, so the emitter must
//! reconstruct the label from the typed address before it can look up a `bootindex`.
//!
//! `bootindex` values are `100 + position-in-boot_order` (confirmed by the felucia sample,
//! RESEARCH.md's Critical Gap section); a device whose reconstructed label is absent from
//! `boot_order` gets no `bootindex` token at all.

/// `ScsiAddress { target, .. }` -> Proxmox label. Assumes `lun == 0`, true for all Proxmox
/// single-disk-per-target usage per RESEARCH.md's Critical Gap mapping table.
pub(crate) fn scsi_label(target: u8) -> String {
    format!("scsi{}", target)
}

/// `IdeAddress { channel, device }` -> Proxmox label. `N = channel * 2 + device`, confirmed
/// by the felucia sample (`channel=1,device=0` -> `bus=ide.1,unit=0` -> `ide2`).
pub(crate) fn ide_label(channel: u8, device: u8) -> String {
    format!("ide{}", channel * 2 + device)
}

/// `SataAddress { port, .. }` -> Proxmox label.
/// `[ASSUMED]` — not present in the felucia sample, follows the scsi/ide `{bus}{index}`
/// convention per RESEARCH.md Assumption A2.
pub(crate) fn sata_label(port: u8) -> String {
    format!("sata{}", port)
}

/// `PcieAddress`-derived NIC ordinal -> Proxmox label.
/// `[ASSUMED]` — ordinal = position among `VirtioNetPcie` entries sorted by `PcieAddress`;
/// only one NIC in the only available corpus file, per RESEARCH.md Assumption A1.
pub(crate) fn net_label(ordinal: u8) -> String {
    format!("net{}", ordinal)
}

/// `100 + position-in-boot_order`, or `None` when `label` is absent from `boot_order`
/// (including the common case of an empty `boot_order` — never panics).
pub(crate) fn lookup_bootindex(boot_order: &[String], label: &str) -> Option<u32> {
    boot_order
        .iter()
        .position(|entry| entry == label)
        .map(|pos| 100 + pos as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_07_04_label_reconstruction_formulas() {
        assert_eq!(scsi_label(0), "scsi0");
        assert_eq!(ide_label(1, 0), "ide2");
        assert_eq!(sata_label(3), "sata3");
        assert_eq!(net_label(0), "net0");
    }

    #[test]
    fn test_07_04_lookup_bootindex_felucia_mapping() {
        let boot_order = vec!["scsi0".to_string(), "ide2".to_string(), "net0".to_string()];
        assert_eq!(lookup_bootindex(&boot_order, "scsi0"), Some(100));
        assert_eq!(lookup_bootindex(&boot_order, "ide2"), Some(101));
        assert_eq!(lookup_bootindex(&boot_order, "net0"), Some(102));
    }

    #[test]
    fn test_07_04_lookup_bootindex_absent_and_empty_never_panic() {
        let boot_order = vec!["scsi0".to_string()];
        assert_eq!(lookup_bootindex(&boot_order, "sata0"), None);
        assert_eq!(lookup_bootindex(&[], "scsi0"), None);
    }
}
