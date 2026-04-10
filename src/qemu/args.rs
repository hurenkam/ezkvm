//! QEMU argument management
//!
//! Structured representation of QEMU command-line arguments.

use super::types::QemuArgs;

impl QemuArgs {
    /// Add a machine type argument
    pub fn add_machine(&mut self, machine: &str) {
        self.push_str("-machine");
        self.push(format!("type={}", machine));
    }

    /// Add VM name argument
    pub fn add_name(&mut self, name: &str) {
        self.push_str("-name");
        self.push(name.to_string());
    }

    /// Add a CPU argument
    pub fn add_cpu(&mut self, cpu: &str) {
        self.push_str("-cpu");
        self.push(cpu.to_string());
    }

    /// Add a CPU feature
    pub fn add_cpu_feature(&mut self, feature: &str) {
        // CPU features are added to the existing CPU argument
        if let Some(last) = self.last_mut() {
            if *last != "-cpu" {
                last.push(',');
                last.push_str(feature);
                return;
            }
        }
        // If no CPU argument exists, add a default one
        self.add_cpu(&format!("host{}", feature));
    }

    /// Add memory argument
    pub fn add_memory(&mut self, memory_mib: u32) {
        self.push_str("-m");
        self.push(format!("{}M", memory_mib));
    }

    /// Add SMP argument
    pub fn add_smp(&mut self, cpus: u32) {
        self.push_str("-smp");
        self.push(format!("cpus={}", cpus));
    }
    
    /// Add drive argument
    pub fn add_drive(&mut self, path: &str, interface: &str, format: &str, readonly: bool) {
        self.push_str("-drive");
        let mut spec = format!("file={},if={},format={}", path, interface, format);
        if readonly {
            spec.push_str(",readonly=on");
        }
        self.push(spec);
    }

    /// Add network argument
    pub fn add_network(&mut self, model: &str, mode: &str, mac: Option<&str>) {
        // Generate a unique ID for this network device
        let id = format!("net{}", self.len());

        self.push_str("-netdev");
        self.push(format!("id={},{}", id, mode));

        self.push_str("-device");
        let mut device_spec = format!("{},netdev={}", model, id);
        if let Some(mac_addr) = mac {
            device_spec.push_str(&format!(",mac={}", mac_addr));
        }
        self.push(device_spec);
    }

    /// Add display argument
    pub fn add_display(&mut self, display_type: &str, vram_mib: Option<u32>) {
        self.push_str("-device");
        let mut spec = display_type.to_string();
        if let Some(vram) = vram_mib {
            spec.push_str(&format!(",vram={}", vram * 1024 * 1024)); // Convert MiB to bytes
        }
        self.push(spec);
    }

    /// Add boot order argument
    pub fn add_boot_order(&mut self, order: &[String]) {
        self.push_str("-boot");
        let order_str = order.join(",");
        self.push(format!("order={}", order_str));
    }

    /// Add kernel argument
    pub fn add_kernel(&mut self, kernel: &str) {
        self.push_str("-kernel");
        self.push(kernel.to_string());
    }

    /// Add initrd argument
    pub fn add_initrd(&mut self, initrd: &str) {
        self.push_str("-initrd");
        self.push(initrd.to_string());
    }
    
    /// Add append argument
    pub fn add_append(&mut self, cmdline: &str) {
        self.push_str("-append");
        self.push(cmdline.to_string());
    }

    /// Add TPM device
    pub fn add_tpm(&mut self, version: &str, backend: &str, state_path: Option<&str>, model: &str) {
        match backend {
            "emulator" => {
                // Add chardev for TPM emulator
                let chardev_id = "tpmchar";
                self.push_str("-chardev");
                let mut chardev_spec = format!("socket,id={},server=on,wait=off", chardev_id);
                if let Some(path) = state_path {
                    chardev_spec.push_str(&format!(",path={}", path));
                } else {
                    // Default path for TPM state
                    chardev_spec.push_str(",path=/var/run/qemu-server/tpm");
                }
                self.push(chardev_spec);

                // Add TPM device
                self.push_str("-tpmdev");
                self.push(format!("emulator,id=tpmdev,chardev={}", chardev_id));
            }
            "passthrough" => {
                self.push_str("-tpmdev");
                self.push("passthrough,id=tpmdev".to_string());
            }
            _ => panic!("Unsupported TPM backend: {}", backend),
        }

        // Add TPM device
        self.push_str("-device");
        self.push(format!("{},tpmdev=tpmdev", model));
    }

    /// Add QEMU guest agent
    pub fn add_guest_agent(&mut self, socket_path: Option<&str>, freeze_cpu: bool) {
        // Add virtio-serial device for guest agent
        self.push_str("-device");
        self.push("virtio-serial-pci,id=virtio-serial0".to_string());

        // Add chardev for guest agent
        let chardev_id = "qga0";
        self.push_str("-chardev");
        let mut chardev_spec = format!("socket,path={},server=on,wait=off,id={}",
            socket_path.unwrap_or("/var/run/qemu-server/qga.sock"), chardev_id);
        self.push(chardev_spec);

        // Add guest agent channel
        self.push_str("-device");
        let mut channel_spec = format!("virtserialport,chardev={},name=org.qemu.guest_agent.0", chardev_id);
        if freeze_cpu {
            channel_spec.push_str(",freeze=on");
        }
        self.push(channel_spec);
    }

    /// Add memory ballooning device
    pub fn add_balloon(&mut self, model: &str, free_page_reporting: bool) {
        self.push_str("-device");
        let mut balloon_spec = model.to_string();
        if free_page_reporting {
            balloon_spec.push_str(",free-page-reporting=on");
        }
        self.push(balloon_spec);
    }

    /// Add UEFI firmware
    pub fn add_uefi(&mut self, code_path: Option<&str>, vars_path: Option<&str>, secure_boot: bool) {
        // Set firmware to UEFI
        self.push_str("-bios");
        if let Some(code) = code_path {
            // Custom OVMF firmware
            self.push(code.to_string());
        } else {
            // Use system OVMF
            self.push("/usr/share/ovmf/OVMF.fd".to_string());
        }

        // Add UEFI variables if specified
        if let Some(vars) = vars_path {
            self.push_str("-drive");
            self.push(format!("if=pflash,format=raw,file={},readonly=on", vars));
        }

        // Enable secure boot if requested
        if secure_boot {
            // Secure boot is enabled by using the correct OVMF firmware
            // The actual secure boot configuration is handled by the firmware
        }
    }

    /// Add VFIO-PCI device passthrough
    pub fn add_vfio_pci(&mut self, device: &str, id: &str, pcie: bool, x_vga: bool, romfile: Option<&str>) {
        self.push_str("-device");
        let mut vfio_spec = format!("vfio-pci,host={},id={}", device, id);
        if pcie {
            vfio_spec.push_str(",pcie=1");
        }
        if x_vga {
            vfio_spec.push_str(",x-vga=1");
        }
        if let Some(rom) = romfile {
            vfio_spec.push_str(&format!(",romfile={}", rom));
        }
        self.push(vfio_spec);
    }

    /// Add USB host device passthrough
    pub fn add_usb_host(&mut self, host_spec: &str, id: &str, bus: Option<&str>, port: Option<&str>) {
        self.push_str("-device");
        let mut usb_spec = format!("usb-host,host={},id={}", host_spec, id);
        if let Some(bus) = bus {
            usb_spec.push_str(&format!(",bus={}", bus));
        }
        if let Some(port) = port {
            usb_spec.push_str(&format!(",port={}", port));
        }
        self.push(usb_spec);
    }

    /// Add XHCI USB controller
    pub fn add_xhci_controller(&mut self, id: &str) {
        self.push_str("-device");
        self.push(format!("qemu-xhci,id={}", id));
    }

    /// Add SPICE display server
    pub fn add_spice(&mut self, port: u16, addr: &str, disable_ticketing: bool, audio: bool, vdagent: bool) {
        self.push_str("-spice");
        let mut spice_spec = format!("port={},addr={}", port, addr);
        if disable_ticketing {
            spice_spec.push_str(",disable-ticketing=on");
        }
        self.push(spice_spec);

        // Add SPICE display device
        self.push_str("-device");
        self.push("qxl-vga,id=video0".to_string());

        // Add SPICE audio if enabled
        if audio {
            self.push_str("-device");
            self.push("spice-audio,id=audio0".to_string());
        }

        // Add vdagent channel for clipboard sharing
        if vdagent {
            self.push_str("-device");
            self.push("virtio-serial-pci,id=virtio-serial0".to_string());
            self.push_str("-chardev");
            self.push("spicevmc,id=vdagent,name=vdagent".to_string());
            self.push_str("-device");
            self.push("virtserialport,chardev=vdagent,name=com.redhat.spice.0".to_string());
        }
    }

    /// Add Looking Glass shared memory device
    pub fn add_ivshmem(&mut self, size_mib: u32, vectors: u32, id: &str) {
        self.push_str("-device");
        let size_bytes = size_mib * 1024 * 1024;
        self.push(format!("ivshmem-plain,memdev=ivshmem,id={},size={}", id, size_bytes));

        self.push_str("-object");
        self.push(format!("memory-backend-file,id=ivshmem,share=on,mem-path=/dev/shm/looking-glass,size={}M", size_mib));
    }

    /// Add SCSI controller
    pub fn add_scsi_controller(&mut self, id: &str, controller_type: &str, iothread: Option<&str>, max_targets: Option<u32>) {
        self.push_str("-device");
        let mut controller_spec = format!("{},id={}", controller_type, id);
        
        if let Some(iothread) = iothread {
            controller_spec.push_str(&format!(",iothread={}", iothread));
        }
        
        if let Some(max_targets) = max_targets {
            controller_spec.push_str(&format!(",max_targets={}", max_targets));
        }
        
        self.push(controller_spec);
    }

    /// Add iSCSI disk
    pub fn add_iscsi_disk(&mut self, id: &str, portal: &str, target: &str, lun: u32, 
                         initiator: Option<&str>, username: Option<&str>, password: Option<&str>,
                         controller: Option<&str>) {
        // Add iSCSI block device
        self.push_str("-blockdev");
        let mut blockdev_spec = format!("driver=iscsi,portal={},target={},lun={},node-name={}",
            portal, target, lun, id);
        
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
        let device_type = if controller.is_some() { "scsi-hd" } else { "virtio-blk-pci" };
        let mut device_spec = format!("{},drive={},id={}", device_type, id, id);
        
        if let Some(controller) = controller {
            device_spec.push_str(&format!(",bus={}.0", controller));
        }
        
        self.push(device_spec);
    }

    /// Add enhanced drive with advanced options
    pub fn add_drive_enhanced(&mut self, path: &str, interface: &str, format: &str, readonly: bool,
                             discard: bool, ssd: bool, controller: Option<&str>) {
        self.push_str("-drive");
        let mut spec = format!("file={},if={},format={}", path, interface, format);
        
        if readonly {
            spec.push_str(",readonly=on");
        }
        
        if discard {
            spec.push_str(",discard=unmap");
        }
        
        if ssd {
            spec.push_str(",ssd=on");
        }
        
        if let Some(controller) = controller {
            spec.push_str(&format!(",bus={}", controller));
        }
        
        self.push(spec);
    }

    /// Add QMP monitoring
    pub fn add_qmp(&mut self, socket_path: Option<&str>, socket_type: &str) {
        self.push_str("-qmp");
        
        let socket_spec = match socket_type {
            "tcp" => {
                // Default to localhost:4444 if no path specified
                socket_path.map(|p| format!("tcp:{}", p)).unwrap_or_else(|| "tcp:127.0.0.1:4444".to_string())
            },
            _ => {
                // Unix socket (default)
                socket_path.map(|p| format!("unix:{}", p)).unwrap_or_else(|| "unix:/var/run/qemu-monitor.sock".to_string())
            }
        };
        
        self.push(socket_spec);
    }

    /// Add SMBIOS system information
    pub fn add_smbios(&mut self, manufacturer: Option<&str>, product: Option<&str>, 
                     version: Option<&str>, serial: Option<&str>, uuid: Option<&str>,
                     sku: Option<&str>, family: Option<&str>) {
        let mut smbios_spec = "type=1".to_string();
        
        if let Some(manufacturer) = manufacturer {
            smbios_spec.push_str(&format!(",manufacturer={}", manufacturer));
        }
        
        if let Some(product) = product {
            smbios_spec.push_str(&format!(",product={}", product));
        }
        
        if let Some(version) = version {
            smbios_spec.push_str(&format!(",version={}", version));
        }
        
        if let Some(serial) = serial {
            smbios_spec.push_str(&format!(",serial={}", serial));
        }
        
        if let Some(uuid) = uuid {
            smbios_spec.push_str(&format!(",uuid={}", uuid));
        }
        
        if let Some(sku) = sku {
            smbios_spec.push_str(&format!(",sku={}", sku));
        }
        
        if let Some(family) = family {
            smbios_spec.push_str(&format!(",family={}", family));
        }
        
        self.push_str("-smbios");
        self.push(smbios_spec);
    }

    /// Add VM generation ID (for Windows Server 2016+)
    pub fn add_vm_generation_id(&mut self, generation_id: &str) {
        self.push_str("-device");
        self.push(format!("vmgenid,guid={}", generation_id));
    }

    /// Add NUMA node configuration
    pub fn add_numa_node(&mut self, node_id: u32, memory_mib: u32, cpus: &[u32], host_node: Option<u32>) {
        self.push_str("-numa");
        let mut numa_spec = format!("node,nodeid={},mem={}", node_id, memory_mib);
        
        if let Some(host_node) = host_node {
            numa_spec.push_str(&format!(",memdev=mem{}", host_node));
        }
        
        self.push(numa_spec);
        
        // Add CPU assignment to this node
        for cpu in cpus {
            self.push_str("-numa");
            self.push(format!("cpu,node-id={},socket-id={}", node_id, cpu));
        }
    }

    /// Add Hyper-V enlightenments
    pub fn add_hyperv(&mut self, relaxed: bool, vapic: bool, time: bool, crash: bool,
                     reset: bool, vendor_id: Option<&str>, frequencies: bool,
                     reenlightenment: bool, tlbflush: bool, ipi: bool,
                     spinlock_retry: Option<u32>) {
        // Add Hyper-V CPU features
        if relaxed {
            self.push_str("-cpu");
            self.push_str("host,+hypervisor,+invtsc,hv_relaxed,hv_spinlocks=0x1fff,hv_vapic,hv_time");
        }
        
        // Add individual Hyper-V features
        if vapic {
            self.add_cpu_feature("+hv_vapic");
        }
        
        if time {
            self.add_cpu_feature("+hv_time");
        }
        
        if crash {
            self.add_cpu_feature("+hv_crash");
        }
        
        if reset {
            self.add_cpu_feature("+hv_reset");
        }
        
        if let Some(vendor_id) = vendor_id {
            self.add_cpu_feature(&format!("+hv_vendor_id={}", vendor_id));
        }
        
        if frequencies {
            self.add_cpu_feature("+hv_frequencies");
        }
        
        if reenlightenment {
            self.add_cpu_feature("+hv_reenlightenment");
        }
        
        if tlbflush {
            self.add_cpu_feature("+hv_tlbflush");
        }
        
        if ipi {
            self.add_cpu_feature("+hv_ipi");
        }
        
        if let Some(retry) = spinlock_retry {
            self.add_cpu_feature(&format!("+hv_spinlocks=0x{:x}", retry));
        }
    }

    /// Add enable KVM argument
    pub fn add_enable_kvm(&mut self) {
        self.push_str("-enable-kvm");
    }

    /// Add daemonize argument
    pub fn add_daemonize(&mut self) {
        self.push_str("-daemonize");
    }

    /// Add a custom argument
    pub fn add_arg(&mut self, arg: &str) {
        self.push(arg.to_string());
    }

    /// Add a key-value argument
    pub fn add_key_value(&mut self, key: &str, value: &str) {
        self.push(format!("-{}", key));
        self.push(value.to_string());
    }

    /// Build the final argument list
    pub fn build(self) -> Vec<String> {
        self.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_args() {
        let mut args = QemuArgs::new();
        args.add_memory(1024);
        args.add_smp(2);
        
        let built = args.build();
        assert_eq!(built, vec!["-m", "1024M", "-smp", "cpus=2"]);
    }
    
    #[test]
    fn test_drive_args() {
        let mut args = QemuArgs::new();
        args.add_drive("/path/to/disk.qcow2", "virtio", "qcow2", false);
        
        let built = args.build();
        assert_eq!(built, vec!["-drive", "file=/path/to/disk.qcow2,if=virtio,format=qcow2"]);
    }
    
    #[test]
    fn test_readonly_drive() {
        let mut args = QemuArgs::new();
        args.add_drive("/path/to/cd.iso", "ide", "raw", true);
        
        let built = args.build();
        assert_eq!(built, vec!["-drive", "file=/path/to/cd.iso,if=ide,format=raw,readonly=on"]);
    }
}