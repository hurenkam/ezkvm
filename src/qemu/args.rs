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