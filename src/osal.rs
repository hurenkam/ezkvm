use log::{debug, error};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::{fs, process};

#[derive(Debug, PartialEq)]
pub enum OsalError {
    OpenError(Option<String>),
    ReadError(Option<String>),
    WriteError(Option<String>),
    ExecError(Option<String>),
    DeleteError(Option<String>),
    ParseError(Option<String>),
    Busy(Option<String>),
}

pub struct Osal {}
#[cfg_attr(test, mockall::automock)]
#[allow(unused)]
impl Osal {
    fn resolve_search_location(location: &str) -> PathBuf {
        if let Some(stripped) = location.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(stripped);
            }
        }

        PathBuf::from(location)
    }

    pub fn get_uid_and_gid() -> (u32, u32) {
        (
            u32::from(nix::unistd::getuid()),
            u32::from(nix::unistd::getgid()),
        )
    }
    pub fn get_euid_and_egid() -> (u32, u32) {
        (
            u32::from(nix::unistd::geteuid()),
            u32::from(nix::unistd::getegid()),
        )
    }
    pub fn find_files<S, T>(pattern: S, locations: Vec<T>) -> Vec<PathBuf>
    where
        S: 'static + AsRef<str>,
        T: 'static + AsRef<str>,
    {
        let mut results = BTreeSet::new();
        let pattern = pattern.as_ref();

        for location in locations {
            let location = Self::resolve_search_location(location.as_ref());
            let search_pattern = location.join(pattern);

            if let Ok(matches) = glob::glob(search_pattern.to_string_lossy().as_ref()) {
                for path in matches.flatten() {
                    results.insert(path);
                }
            }
        }

        results.into_iter().collect()
    }
    pub fn read_file<P: 'static + AsRef<Path>>(path: P) -> Result<String, OsalError> {
        let file = format!("{:?}", path.as_ref());
        let content = fs::read(path).map_err(|_| OsalError::ReadError(Some(file.clone())))?;
        String::from_utf8(content).map_err(|_| OsalError::ParseError(Some(file)))
    }
    pub fn read_yaml_file<P, T>(path: P) -> Result<T, OsalError>
    where
        P: 'static + AsRef<Path>,
        T: 'static + for<'a> Deserialize<'a>,
    {
        let content = Self::read_file(path)?;
        serde_yaml::from_str(content.as_str()).map_err(|_| OsalError::ParseError(None))
    }
    pub fn write_file<P: 'static + AsRef<Path>, S: 'static + AsRef<str>>(
        path: P,
        content: S,
    ) -> Result<(), OsalError> {
        let file = format!("{:?}", path.as_ref());
        fs::write(path, content.as_ref()).map_err(|_| OsalError::WriteError(Some(file)))
    }
    pub fn delete_file<P: 'static + AsRef<Path>>(path: P) -> Result<(), OsalError> {
        let file = format!("{:?}", path.as_ref());
        fs::remove_file(path).map_err(|_| OsalError::DeleteError(Some(file)))
    }
    pub fn execute_command<P: 'static + Display + AsRef<Path>>(
        command: &mut Command,
        log_path: Option<P>,
    ) -> Result<Child, OsalError> {
        if let Some(log_path) = log_path {
            let log_file = File::create(format!("{}.log", log_path)).unwrap();
            let err_file = log_file.try_clone().expect("unable to clone log_file");
            //let err_file = File::create(format!("{}.err", log_path)).unwrap();
            let log = process::Stdio::from(log_file);
            let err = process::Stdio::from(err_file);

            command.stdout(log).stderr(err);
        }

        match command
            .spawn()
            .map_err(|_| OsalError::ExecError(Some(format!("{:?}", command))))
        {
            Ok(child) => {
                let mut cmd = format!("");
                for arg in command.get_args() {
                    cmd = cmd + &format!(" {}", arg.to_str().expect(""))
                }
                debug!(
                    "Osal::execute_command(): Spawned '{:?}' with pid {}",
                    cmd,
                    child.id()
                );
                Ok(child)
            }
            Err(error) => {
                let mut cmd = format!("");
                for arg in command.get_args() {
                    cmd = cmd + &format!(" {}", arg.to_str().expect(""))
                }
                error!(
                    "Osal::execute_command(): Unable to spawn '{:?}' due to error {:?}",
                    cmd, error
                );
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Osal;
    use std::path::PathBuf;

    #[test]
    fn test_find_files_searches_all_locations() {
        let tmp = tempfile::tempdir().unwrap();
        let loc1 = tmp.path().join("a");
        let loc2 = tmp.path().join("b");
        std::fs::create_dir_all(&loc1).unwrap();
        std::fs::create_dir_all(&loc2).unwrap();

        let file_name = "vm-test.yaml";
        let file1 = loc1.join(file_name);
        let file2 = loc2.join(file_name);
        std::fs::write(&file1, "one").unwrap();
        std::fs::write(&file2, "two").unwrap();

        let mut actual = Osal::find_files(
            file_name,
            vec![
                loc1.to_string_lossy().to_string(),
                loc2.to_string_lossy().to_string(),
            ],
        );
        actual.sort();

        let mut expected = vec![file1, file2];
        expected.sort();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_resolve_search_location_expands_tilde() {
        let home = std::env::var("HOME").unwrap();
        let actual = Osal::resolve_search_location("~/.ezkvm");
        let expected = PathBuf::from(home).join(".ezkvm");

        assert_eq!(actual, expected);
    }
}
