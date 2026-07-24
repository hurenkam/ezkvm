use crate::config::qemu::QemuCommandLine;

/// Ordered-segment collector for `QemuCommandLine`. Each `push_*` method appends into a
/// distinct segment `Vec`; final render order is fixed by `QemuCommandLine::fmt`, not by
/// the order handlers call `push_*` (see D-08 / Pitfall on TpmState chardev<tpmdev<device
/// ordering: it's guaranteed by segment placement, not call sequence).
#[derive(Debug, Default)]
pub(crate) struct QemuCommandLineBuilder {
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

impl QemuCommandLineBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push_machine(&mut self, arg: impl Into<String>) {
        self.machine.push(arg.into());
    }

    pub(crate) fn push_firmware(&mut self, arg: impl Into<String>) {
        self.firmware.push(arg.into());
    }

    pub(crate) fn push_drive(&mut self, arg: impl Into<String>) {
        self.drives.push(arg.into());
    }

    pub(crate) fn push_netdev(&mut self, arg: impl Into<String>) {
        self.netdevs.push(arg.into());
    }

    pub(crate) fn push_chardev(&mut self, arg: impl Into<String>) {
        self.chardevs.push(arg.into());
    }

    pub(crate) fn push_tpm(&mut self, arg: impl Into<String>) {
        self.tpm.push(arg.into());
    }

    pub(crate) fn push_object(&mut self, arg: impl Into<String>) {
        self.objects.push(arg.into());
    }

    pub(crate) fn push_device(&mut self, arg: impl Into<String>) {
        self.devices.push(arg.into());
    }

    pub(crate) fn push_misc(&mut self, arg: impl Into<String>) {
        self.misc.push(arg.into());
    }

    pub(crate) fn build(self) -> QemuCommandLine {
        QemuCommandLine {
            machine: self.machine,
            firmware: self.firmware,
            drives: self.drives,
            netdevs: self.netdevs,
            chardevs: self.chardevs,
            tpm: self.tpm,
            objects: self.objects,
            devices: self.devices,
            misc: self.misc,
        }
    }
}
