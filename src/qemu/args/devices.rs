use crate::qemu::types::QemuArgs;

impl QemuArgs {
    /// Add memory ballooning device
    pub fn add_balloon(
        &mut self,
        model: &str,
        free_page_reporting: bool,
        id: Option<&str>,
        bus: Option<&str>,
        addr: Option<&str>,
    ) {
        self.push_str("-device");
        let mut balloon_spec = model.to_string();
        if let Some(id) = id {
            balloon_spec.push_str(&format!(",id={}", id));
        }
        if let Some(bus) = bus {
            balloon_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            balloon_spec.push_str(&format!(",addr={}", addr));
        }
        if free_page_reporting {
            balloon_spec.push_str(",free-page-reporting=on");
        }
        self.push(balloon_spec);
    }

    /// Add VFIO-PCI device passthrough
    #[allow(clippy::too_many_arguments)]
    pub fn add_vfio_pci(
        &mut self,
        device: &str,
        id: &str,
        _pcie: bool,
        x_vga: bool,
        bus: Option<&str>,
        addr: Option<&str>,
        multifunction: bool,
        romfile: Option<&str>,
    ) {
        self.push_str("-device");
        let mut vfio_spec = format!("vfio-pci,host={},id={}", device, id);
        if x_vga {
            vfio_spec.push_str(",x-vga=on");
        }
        if let Some(bus) = bus {
            vfio_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(addr) = addr {
            vfio_spec.push_str(&format!(",addr={}", addr));
        }
        if multifunction {
            vfio_spec.push_str(",multifunction=on");
        }
        if let Some(rom) = romfile {
            vfio_spec.push_str(&format!(",romfile={}", rom));
        }
        self.push(vfio_spec);
    }

    /// Add Looking Glass shared memory device
    pub fn add_ivshmem(
        &mut self,
        size_mib: u32,
        _vectors: u32,
        id: &str,
        bus: Option<&str>,
        mem_path: &str,
    ) {
        self.push_str("-device");
        let mut device_spec = format!("ivshmem-plain,memdev={}", id);
        if let Some(bus) = bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }
        self.push(device_spec);

        self.push_str("-object");
        self.push(format!(
            "memory-backend-file,id={},share=on,mem-path={},size={}M",
            id, mem_path, size_mib
        ));
    }

    /// Add SCSI controller
    pub fn add_scsi_controller(
        &mut self,
        id: &str,
        controller_type: &str,
        iothread: Option<&str>,
        max_targets: Option<u32>,
        bus: Option<&str>,
        addr: Option<&str>,
    ) {
        self.push_str("-device");
        let mut controller_spec = format!("{},id={}", controller_type, id);

        if let Some(iothread) = iothread {
            controller_spec.push_str(&format!(",iothread={}", iothread));
        }

        if let Some(max_targets) = max_targets {
            controller_spec.push_str(&format!(",max_targets={}", max_targets));
        }

        if let Some(bus) = bus {
            controller_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = addr {
            controller_spec.push_str(&format!(",addr={}", addr));
        }

        self.push(controller_spec);
    }
}
