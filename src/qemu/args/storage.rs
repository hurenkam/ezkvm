use crate::qemu::types::QemuArgs;

impl QemuArgs {
    /// Add drive argument
    pub fn add_drive(&mut self, path: &str, interface: &str, format: &str, readonly: bool) {
        self.push_str("-drive");
        let mut spec = format!("file={},if={},format={}", path, interface, format);
        if readonly {
            spec.push_str(",readonly=on");
        }
        self.push(spec);
    }

    /// Add enhanced drive with advanced options
    #[allow(clippy::too_many_arguments)]
    pub fn add_drive_enhanced(
        &mut self,
        path: &str,
        interface: &str,
        format: &str,
        readonly: bool,
        discard: bool,
        _ssd: bool,
        controller: Option<&str>,
    ) {
        self.push_str("-drive");
        let mut spec = format!("file={},if={},format={}", path, interface, format);

        if readonly {
            spec.push_str(",readonly=on");
        }

        if discard {
            spec.push_str(",discard=unmap");
        }

        if let Some(controller) = controller {
            spec.push_str(&format!(",bus={}", controller));
        }

        self.push(spec);
    }

    /// Add UEFI firmware
    pub fn add_uefi(
        &mut self,
        code_path: Option<&str>,
        vars_path: Option<&str>,
        vars_size: Option<u64>,
        secure_boot: bool,
    ) {
        let firmware_code = code_path.unwrap_or("/usr/share/ovmf/OVMF.fd");

        self.push_str("-drive");
        self.push(format!(
            "if=pflash,unit=0,format=raw,readonly=on,file={}",
            firmware_code
        ));

        if let Some(vars) = vars_path {
            self.push_str("-drive");
            let mut vars_spec = format!(
                "if=pflash,unit=1,id=drive-efidisk0,format=raw,file={}",
                vars
            );
            if let Some(size) = vars_size {
                vars_spec.push_str(&format!(",size={}", size));
            }
            self.push(vars_spec);
        }

        // Enable secure boot if requested
        if secure_boot {
            // Secure boot is enabled by using the correct OVMF firmware
            // The actual secure boot configuration is handled by the firmware
        }
    }

    /// Add iSCSI disk
    #[allow(clippy::too_many_arguments)]
    pub fn add_iscsi_disk(
        &mut self,
        id: &str,
        portal: &str,
        target: &str,
        lun: u32,
        initiator: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        controller: Option<&str>,
    ) {
        // Add iSCSI block device
        self.push_str("-blockdev");
        let mut blockdev_spec = format!(
            "driver=iscsi,portal={},target={},lun={},node-name={}",
            portal, target, lun, id
        );

        if let Some(initiator) = initiator {
            blockdev_spec.push_str(&format!(",initiator-name={}", initiator));
        }

        if let Some(username) = username {
            blockdev_spec.push_str(&format!(",user={}", username));
        }

        if let Some(password) = password {
            blockdev_spec.push_str(&format!(",password={}", password));
        }

        self.push(blockdev_spec);

        // Add device attachment
        self.push_str("-device");
        let device_type = if controller.is_some() {
            "scsi-hd"
        } else {
            "virtio-blk-pci"
        };
        let mut device_spec = format!("{},drive={},id={}", device_type, id, id);

        if let Some(controller) = controller {
            device_spec.push_str(&format!(",bus={}.0", controller));
        }

        self.push(device_spec);
    }
}
