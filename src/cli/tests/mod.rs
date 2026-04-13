use super::runtime;

fn base_config() -> crate::config::VmConfig {
    crate::config::VmConfig::from_str(
        r#"
name: "test-vm"
backend: "qemu"

system:
    architecture: "x86_64"
    machine: "q35"
    memory:
        size: 1024
        ivshmem:
            enabled: true
            size: 128
            id: "ivshmem0"
            mem_path: "/dev/kvmfr0"
    cpu:
        vcpus: 1
        model: "host"
host:
    pci:
        - device: "0000:03:00.0"
          id: "hostpci0"
          x_vga: true

spice:
    enabled: true
    port: 5903
    addr: "0.0.0.0"
        "#,
    )
    .unwrap()
}

mod looking_glass;
mod remote_viewer;
mod runtime_aux;
