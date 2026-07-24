mod builder;
pub(crate) mod bootindex;
mod handlers;

pub(crate) use builder::QemuCommandLineBuilder;

use crate::runtime::Runtime;

use derive_getters::Getters;
use derive_new::new;

#[derive(Debug, thiserror::Error)]
pub enum QemuConversionError {
    #[error("HostPci at pcie address {address} has no functions defined")]
    EmptyHostPciFunctions { address: String },
}

/// Caller-built context carrying VM identity and resolved filesystem paths needed to
/// render a `QemuCommandLine`. Never parsed from untrusted input (D-06) — missing data
/// here is a programmer error surfaced via `.expect()`, not a typed error variant.
#[derive(Debug, Clone, Getters, new)]
pub struct QemuContext {
    vm_name: String,
    ovmf_code_path: String,
    tpm_socket_path: Option<String>,
}

/// The segmented, ordered QEMU commandline. Segments are always rendered in this fixed
/// order regardless of the order handlers pushed into them: machine, firmware, drives,
/// netdevs, chardevs, tpm, objects, devices, misc.
#[derive(Debug, Clone, Default, Getters)]
pub struct QemuCommandLine {
    machine: Vec<String>,
    firmware: Vec<String>,
    drives: Vec<String>,
    netdevs: Vec<String>,
    chardevs: Vec<String>,
    tpm: Vec<String>,
    objects: Vec<String>,
    devices: Vec<String>,
    misc: Vec<String>,
}

impl std::fmt::Display for QemuCommandLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in [
            &self.machine,
            &self.firmware,
            &self.drives,
            &self.netdevs,
            &self.chardevs,
            &self.tpm,
            &self.objects,
            &self.devices,
            &self.misc,
        ] {
            for entry in segment {
                write!(f, " {}", entry)?;
            }
        }
        Ok(())
    }
}

impl TryFrom<(Runtime, QemuContext)> for QemuCommandLine {
    type Error = QemuConversionError;

    fn try_from((runtime, ctx): (Runtime, QemuContext)) -> Result<Self, Self::Error> {
        let mut builder = QemuCommandLineBuilder::new();

        for device in runtime.root_devices() {
            handlers::root::emit_root_device(&mut builder, &ctx, &runtime, device.as_ref())?;
        }

        Ok(builder.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        AudioDevice, Chipset, CpuTopology, EfiDisk, Memory, Q35ChipsetBuilder, RawArgs,
        RuntimeBuilder, SpiceDisplay, TpmState, VgaConfig,
    };

    fn make_ctx() -> QemuContext {
        QemuContext::new(
            "testvm".to_string(),
            "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
            None,
        )
    }

    #[test]
    fn test_07_01_memory_chipset_efidisk_end_to_end() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(16384))
            .with_chipset(Chipset::Q35(Q35ChipsetBuilder::new().build()))
            .with_efidisk(EfiDisk::new(
                "vm1-pool:vm-108-efidisk".to_string(),
                Some("4m".to_string()),
                true,
                None,
                "4M".to_string(),
                Some(540672),
            ))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, make_ctx())).unwrap();
        let output = cmdline.to_string();

        assert!(output.contains("-m 16384"), "output was: {output}");
        assert!(output.contains("-machine q35"), "output was: {output}");
        assert_eq!(output.matches("-drive if=pflash,").count(), 2, "output was: {output}");

        let unit0_idx = output.find("unit=0").expect("unit=0 pflash line missing");
        let unit1_idx = output.find("unit=1").expect("unit=1 pflash line missing");
        assert!(unit0_idx < unit1_idx);

        // Slice out the unit=0 line's content up to unit=1 to check id=drive-efidisk0 placement
        let unit0_segment = &output[unit0_idx..unit1_idx];
        assert!(
            !unit0_segment.contains("id=drive-efidisk0"),
            "unit=0 segment should not contain id=drive-efidisk0: {unit0_segment}"
        );

        let unit1_segment = &output[unit1_idx..];
        assert!(
            unit1_segment.contains("id=drive-efidisk0"),
            "unit=1 segment should contain id=drive-efidisk0: {unit1_segment}"
        );
        assert!(
            unit1_segment.contains("vm1-pool:vm-108-efidisk"),
            "unit=1 segment should contain resolved storage_volume path: {unit1_segment}"
        );
    }

    #[test]
    fn test_07_01_tpmstate_chardev_tpmdev_device_ordering() {
        let ctx = QemuContext::new(
            "testvm".to_string(),
            "/usr/share/OVMF/OVMF_CODE.fd".to_string(),
            Some("/var/run/qemu-server/108.swtpm".to_string()),
        );
        let runtime = RuntimeBuilder::new()
            .with_tpmstate(TpmState::new(
                "vm1-pool:vm-108-tpmstate".to_string(),
                "v2.0".to_string(),
            ))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();

        let idx_chardev = output.find("id=tpmchar").expect("id=tpmchar missing");
        let idx_tpmdev_ref = output.find("chardev=tpmchar").expect("chardev=tpmchar missing");
        let idx_device = output.find("tpmdev=tpmdev").expect("tpmdev=tpmdev missing");

        assert!(idx_chardev < idx_tpmdev_ref, "output was: {output}");
        assert!(idx_tpmdev_ref < idx_device, "output was: {output}");
    }

    #[test]
    fn test_07_01_rawargs_verbatim_passthrough() {
        let ctx = make_ctx();
        let raw = "-spice port=5903 -device foo".to_string();
        let runtime = RuntimeBuilder::new()
            .with_raw_args(RawArgs(raw.clone()))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();

        assert_eq!(cmdline.misc(), &vec![raw.clone()]);

        let output = cmdline.to_string();
        let idx_raw = output.find(&raw).expect("raw args missing verbatim");
        // misc is the last segment, so raw args should be at the tail of the output
        assert_eq!(idx_raw + raw.len(), output.len());
    }

    #[test]
    fn test_07_01_audio_device_spice_codec_pair() {
        let ctx = make_ctx();
        let runtime = RuntimeBuilder::new()
            .with_audio_device(AudioDevice::new(
                "ich9-intel-hda".to_string(),
                "spice".to_string(),
            ))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();

        assert!(output.contains("hda-micro"), "output was: {output}");
        assert!(output.contains("hda-duplex"), "output was: {output}");
        assert!(
            output.contains("-audiodev spice,id=spice-backend0"),
            "output was: {output}"
        );
    }

    #[test]
    fn test_07_verify_spice_display_emits_spice_flag() {
        let ctx = make_ctx();
        let runtime = RuntimeBuilder::new()
            .with_spice_display(SpiceDisplay::new(
                Some(5903),
                Some("127.0.0.1".to_string()),
                true,
                true,
                Some("/dev/dri/renderD128".to_string()),
                true,
            ))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();

        assert!(output.contains("-spice"), "output was: {output}");
        assert!(output.contains("port=5903"), "output was: {output}");
        assert!(output.contains("addr=127.0.0.1"), "output was: {output}");
        assert!(output.contains("disable-ticketing=on"), "output was: {output}");
        assert!(output.contains("gl=on"), "output was: {output}");
        assert!(
            output.contains("rendernode=/dev/dri/renderD128"),
            "output was: {output}"
        );
    }

    #[test]
    fn test_07_verify_cpu_topology_emits_smp_and_cpu() {
        let ctx = make_ctx();
        let runtime = RuntimeBuilder::new()
            .with_cpu_topology(CpuTopology::new(2, 4, "host".to_string()))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();

        assert!(
            output.contains("-smp 8,sockets=2,cores=4,maxcpus=8"),
            "output was: {output}"
        );
        assert!(output.contains("-cpu host"), "output was: {output}");
    }

    #[test]
    fn test_07_verify_vga_config_none_emits_nographic() {
        let ctx = make_ctx();
        let runtime = RuntimeBuilder::new()
            .with_vga_config(VgaConfig::new("none".to_string()))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();

        assert!(output.contains("-vga none"), "output was: {output}");
        assert!(output.contains("-nographic"), "output was: {output}");
    }

    #[test]
    fn test_07_verify_vga_config_other_mode_emits_nothing() {
        let ctx = make_ctx();
        let runtime = RuntimeBuilder::new()
            .with_vga_config(VgaConfig::new("std".to_string()))
            .build()
            .unwrap();

        let cmdline = QemuCommandLine::try_from((runtime, ctx)).unwrap();
        let output = cmdline.to_string();

        assert!(!output.contains("-vga"), "output was: {output}");
        assert!(!output.contains("-nographic"), "output was: {output}");
    }
}
