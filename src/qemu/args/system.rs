use crate::qemu::types::QemuArgs;

impl QemuArgs {
    /// Add QMP monitoring with separate control and event sockets (Proxmox-style)
    /// This uses two monitors to prevent event starvation: control socket for commands,
    /// event socket for async SHUTDOWN/POWERDOWN events.
    pub fn add_qmp(&mut self, socket_path: Option<&str>, socket_type: &str) {
        match socket_type {
            "tcp" => {
                let control_spec = socket_path
                    .map(|p| format!("tcp:{},server=on,wait=off", p))
                    .unwrap_or_else(|| "tcp:127.0.0.1:4444,server=on,wait=off".to_string());
                let event_port = socket_path
                    .and_then(|p| p.split(':').last())
                    .and_then(|port| port.parse::<u16>().ok())
                    .map(|p| p + 1)
                    .unwrap_or(4445);
                let event_spec = format!("tcp:127.0.0.1:{},server=on,wait=off", event_port);

                self.push_str("-chardev");
                self.push(format!("socket,id=qmp,{}", control_spec));
                self.push_str("-mon");
                self.push("chardev=qmp,mode=control".to_string());

                self.push_str("-chardev");
                self.push(format!("socket,id=qmp-event,{}", event_spec));
                self.push_str("-mon");
                self.push("chardev=qmp-event,mode=control".to_string());
            }
            _ => {
                // Unix sockets: control and event
                let control_path = socket_path.unwrap_or("/var/run/qemu-monitor.sock");
                let event_path = if let Some(p) = socket_path {
                    let base = p.trim_end_matches(".qmp").trim_end_matches("/qmp");
                    format!("{}-event.sock", base)
                } else {
                    "/var/run/qemu-monitor-event.sock".to_string()
                };

                self.push_str("-chardev");
                self.push(format!(
                    "socket,id=qmp,path={},server=on,wait=off",
                    control_path
                ));
                self.push_str("-mon");
                self.push("chardev=qmp,mode=control".to_string());

                self.push_str("-chardev");
                self.push(format!(
                    "socket,id=qmp-event,path={},server=on,wait=off",
                    event_path
                ));
                self.push_str("-mon");
                self.push("chardev=qmp-event,mode=control".to_string());
            }
        }
    }

    /// Add SMBIOS system information
    #[allow(clippy::too_many_arguments)]
    pub fn add_smbios(
        &mut self,
        smbios_type: u8,
        manufacturer: Option<&str>,
        product: Option<&str>,
        version: Option<&str>,
        serial: Option<&str>,
        uuid: Option<&str>,
        sku: Option<&str>,
        family: Option<&str>,
    ) {
        let smbios_spec = build_smbios_spec(
            smbios_type,
            manufacturer,
            product,
            version,
            serial,
            uuid,
            sku,
            family,
        );

        self.push_str("-smbios");
        self.push(smbios_spec);
    }

    /// Add VM generation ID (for Windows Server 2016+)
    pub fn add_vm_generation_id(&mut self, generation_id: &str) {
        self.push_str("-device");
        self.push(format!("vmgenid,guid={}", generation_id));
    }

    /// Add NUMA node configuration with optional memory-backend binding
    pub fn add_numa_node(
        &mut self,
        node_id: u32,
        memory_mib: u32,
        cpus: &[u32],
        host_node: Option<u32>,
    ) {
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

    /// Add NUMA node with hugepages memory-backend binding
    pub fn add_numa_node_with_memdev(&mut self, node_id: u32, cpus: &[u32], memdev_id: &str) {
        self.push_str("-numa");
        self.push(format!("node,nodeid={},memdev={}", node_id, memdev_id));

        for cpu in cpus {
            self.push_str("-numa");
            self.push(format!("cpu,node-id={},socket-id={}", node_id, cpu));
        }
    }

    /// Add a hugepages memory-backend-file object for a single NUMA node
    pub fn add_hugepages_memory_backend(
        &mut self,
        id: &str,
        size_mib: u32,
        mem_path: &str,
        prealloc: bool,
    ) {
        self.push_str("-object");
        let mut spec = format!(
            "memory-backend-file,id={},size={}M,mem-path={},share=on",
            id, size_mib, mem_path
        );
        if prealloc {
            spec.push_str(",prealloc=yes");
        }
        self.push(spec);
    }

    /// Add Hyper-V enlightenments
    #[allow(clippy::too_many_arguments)]
    pub fn add_hyperv(
        &mut self,
        relaxed: bool,
        vapic: bool,
        time: bool,
        crash: bool,
        reset: bool,
        vendor_id: Option<&str>,
        frequencies: bool,
        reenlightenment: bool,
        tlbflush: bool,
        ipi: bool,
        spinlock_retry: Option<u32>,
    ) {
        self.add_hyperv_base_profile(relaxed);
        self.add_hyperv_flag_features(
            vapic,
            time,
            crash,
            reset,
            frequencies,
            reenlightenment,
            tlbflush,
            ipi,
        );
        self.add_hyperv_optional_features(vendor_id, spinlock_retry);
    }
}

#[allow(clippy::too_many_arguments)]
fn build_smbios_spec(
    smbios_type: u8,
    manufacturer: Option<&str>,
    product: Option<&str>,
    version: Option<&str>,
    serial: Option<&str>,
    uuid: Option<&str>,
    sku: Option<&str>,
    family: Option<&str>,
) -> String {
    let mut smbios_spec = format!("type={}", smbios_type);
    push_smbios_field(&mut smbios_spec, "manufacturer", manufacturer);
    push_smbios_field(&mut smbios_spec, "product", product);
    push_smbios_field(&mut smbios_spec, "version", version);
    push_smbios_field(&mut smbios_spec, "serial", serial);
    push_smbios_field(&mut smbios_spec, "uuid", uuid);
    push_smbios_field(&mut smbios_spec, "sku", sku);
    push_smbios_field(&mut smbios_spec, "family", family);
    smbios_spec
}

fn push_smbios_field(spec: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        spec.push_str(&format!(",{}={}", key, value));
    }
}

impl QemuArgs {
    fn add_hyperv_base_profile(&mut self, relaxed: bool) {
        if relaxed {
            self.push_str("-cpu");
            self.push_str(
                "host,+hypervisor,+invtsc,hv_relaxed,hv_spinlocks=0x1fff,hv_vapic,hv_time",
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_hyperv_flag_features(
        &mut self,
        vapic: bool,
        time: bool,
        crash: bool,
        reset: bool,
        frequencies: bool,
        reenlightenment: bool,
        tlbflush: bool,
        ipi: bool,
    ) {
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
    }

    fn add_hyperv_optional_features(
        &mut self,
        vendor_id: Option<&str>,
        spinlock_retry: Option<u32>,
    ) {
        if let Some(vendor_id) = vendor_id {
            self.add_cpu_feature(&format!("+hv_vendor_id={}", vendor_id));
        }
        if let Some(retry) = spinlock_retry {
            self.add_cpu_feature(&format!("+hv_spinlocks=0x{:x}", retry));
        }
    }
}
