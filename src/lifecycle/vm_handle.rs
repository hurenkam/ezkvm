use std::{
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::process;

#[derive(Debug, Clone, Serialize, Deserialize, Getters, new)]
pub struct VmHandle {
    vm_name: String,
    qemu_pid: u32,
    swtpm_pid: Option<u32>,
    ui_client_pid: Option<u32>,
    qmp_socket_path: String,
    tpm_socket_path: Option<String>,
}

#[derive(Debug, Error)]
pub enum VmHandleError {
    #[error("vm handle I/O failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("vm handle YAML failed for {path}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: crate::serde_yaml::Error,
    },
    #[error("refusing to follow symlinked vm handle path {path}")]
    UnsafePath { path: PathBuf },
}

impl VmHandle {
    pub fn state_path(state_dir: &Path, vm_name: &str) -> PathBuf {
        state_dir.join(format!("{vm_name}.state"))
    }

    pub fn write(&self, state_dir: &Path) -> Result<(), VmHandleError> {
        process::ensure_dir_secure(state_dir).map_err(|source| VmHandleError::Io {
            path: state_dir.to_path_buf(),
            source,
        })?;
        let path = Self::state_path(state_dir, &self.vm_name);
        reject_symlink(&path)?;
        let content = crate::serde_yaml::to_string(self).map_err(|source| VmHandleError::Yaml {
            path: path.clone(),
            source,
        })?;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)
            .map_err(|source| map_path_io_error(&path, source))?;
        file.write_all(content.as_bytes())
            .map_err(|source| VmHandleError::Io { path, source })
    }

    pub fn read(state_dir: &Path, vm_name: &str) -> Result<Option<VmHandle>, VmHandleError> {
        let path = Self::state_path(state_dir, vm_name);
        reject_symlink(&path)?;

        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)
        {
            Ok(file) => file,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(map_path_io_error(&path, source)),
        };

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|source| VmHandleError::Io {
                path: path.clone(),
                source,
            })?;

        let handle = crate::serde_yaml::from_str(&content).map_err(|source| VmHandleError::Yaml {
            path: path.clone(),
            source,
        })?;
        Ok(Some(handle))
    }

    pub fn remove(state_dir: &Path, vm_name: &str) -> Result<(), VmHandleError> {
        let path = Self::state_path(state_dir, vm_name);
        reject_symlink(&path)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(VmHandleError::Io { path, source }),
        }
    }

    pub fn is_stale(&self) -> bool {
        !process::is_pid_alive(self.qemu_pid)
    }
}

fn reject_symlink(path: &Path) -> Result<(), VmHandleError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(VmHandleError::UnsafePath {
            path: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(VmHandleError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn map_path_io_error(path: &Path, source: std::io::Error) -> VmHandleError {
    if source.raw_os_error() == Some(libc::ELOOP) {
        VmHandleError::UnsafePath {
            path: path.to_path_buf(),
        }
    } else {
        VmHandleError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}
