pub mod io;
pub mod mapper;
pub mod proxmox_model;
pub mod proxmox_parser;
pub mod proxmox_storage_parser;
pub mod report;

pub use report::EzkvmImportResult;

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    ParseError(String),
    ValidationError(String),
    IoError(String),
}
