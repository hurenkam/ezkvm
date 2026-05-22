use anyhow::Result;

mod create;
mod device;
mod import_proxmox;
mod import_qemu_cmd;
mod network;
mod storage;

pub(crate) use create::handle_create;
pub(crate) use device::handle_device;
pub(crate) use import_proxmox::handle_import_proxmox;
pub(crate) use import_qemu_cmd::handle_import_qemu_cmd;
pub(crate) use network::handle_network;
pub(crate) use storage::handle_storage;

pub(crate) type CliResult = Result<()>;
