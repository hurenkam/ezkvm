//! Process management for QEMU VMs
//!
//! Handles discovery, monitoring, and control of running QEMU processes.

mod control;
mod discovery;
mod shutdown_monitor;

pub use control::{is_vm_running, kill_vm, stop_vm};
pub use discovery::{find_qemu_processes, list_running_vms};
pub use shutdown_monitor::spawn_shutdown_monitor;
