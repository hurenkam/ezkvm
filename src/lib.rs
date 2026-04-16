//! ezkvm - Easy KVM virtual machine manager
//!
//! A simple alternative to libvirt and virt-manager that uses YAML
//! configuration files to manage QEMU/KVM virtual machines.

pub mod cli;
pub mod config;
pub mod device;
pub mod import;
pub mod network;
pub mod qemu;
pub mod state;
pub mod storage;

#[cfg(test)]
pub(crate) mod test_support;
