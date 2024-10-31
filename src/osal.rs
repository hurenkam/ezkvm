use std::{fs, process};
use std::fmt::Display;
use std::fs::File;
use std::path::Path;
use std::process::{Child, Command};
use log::{debug, error};

#[derive(Debug,PartialEq)]
pub enum OsalError {
    OpenError(Option<String>),
    ReadError(Option<String>),
    WriteError(Option<String>),
    ExecError(Option<String>),
    DeleteError(Option<String>),
    ParseError(Option<String>),
    Busy(Option<String>)
}

pub struct Osal {}
#[cfg_attr(test,mockall::automock)]
#[allow(unused)]
impl Osal {
    pub fn read_file<P: 'static + AsRef<Path>>(path: P) -> Result<String, OsalError>
    {
        let file = format!("{:?}",path.as_ref());
        let content = fs::read(path).map_err(|_| OsalError::ReadError(Some(file.clone())))?;
        Ok(String::from_utf8(content).map_err(|_| OsalError::ParseError(Some(file)))?)
    }
    pub fn write_file<P: 'static + AsRef<Path>, S: 'static + AsRef<str>>(path: P, content: S) -> Result<(), OsalError>
    {
        let file = format!("{:?}",path.as_ref());
        Ok(fs::write(path, content.as_ref()).map_err(|_| OsalError::WriteError(Some(file)))?)
    }
    pub fn delete_file<P: 'static + AsRef<Path>>(path: P) -> Result<(), OsalError>
    {
        let file = format!("{:?}",path.as_ref());
        Ok(fs::remove_file(path).map_err(|_| OsalError::DeleteError(Some(file)))?)
    }
    pub fn execute_command<P: 'static + Display + AsRef<Path>>(command: &mut Command, log_path: Option<P>) -> Result<Child, OsalError>
    {
        if let Some(log_path) = log_path {
            let log_file = File::create(format!("{}.log", log_path)).unwrap();
            let err_file = log_file.try_clone().expect("unable to clone log_file");
            let log = process::Stdio::from(log_file);
            //let err_file = File::create(format!("{}.err", log_path)).unwrap();
            let err = process::Stdio::from(err_file);

            command.stdout(log).stderr(err);
        }

        match command.spawn().map_err(|_| OsalError::ExecError(Some(format!("{:?}", command)))) {
            Ok(child) => {
                debug!("Osal::execute_command(): Spawned '{:?} {:?}' with pid {}", command.get_program(),command.get_args(), child.id());
                Ok(child)
            },
            Err(error) => {
                error!("Osal::execute_command(): Unable to spawn '{:?} {:?}' due to error {:?}", command.get_program(),command.get_args(), error);
                Err(error)
            }
        }
    }

    pub fn _execute_command(command: &mut Command, log_path: Option<String>) -> Result<Child, OsalError>
    {
        if let Some(log_path) = log_path {
            let log_file = File::create(format!("{}.log", log_path)).unwrap();
            let err_file = log_file.try_clone().expect("unable to clone log_file");
            let log = process::Stdio::from(log_file);
            //let err_file = File::create(format!("{}.err", log_path)).unwrap();
            let err = process::Stdio::from(err_file);

            command.stdout(log).stderr(err);
        }

        match command.spawn().map_err(|_| OsalError::ExecError(Some(format!("{:?}",command)))) {
            Ok(child) => {
                debug!("Osal::execute_command(): Spawned '{:?} {:?}' with pid {}", command.get_program(),command.get_args(), child.id());
                Ok(child)
            },
            Err(error) => {
                error!("Osal::execute_command(): Unable to spawn '{:?} {:?}' due to error {:?}", command.get_program(),command.get_args(), error);
                Err(error)
            }
        }
/*
        match log_path
        {
            Some(log_path) =>
                {
                    let log_file = File::create(format!("{}.log", log_path)).unwrap();
                    let log = process::Stdio::from(log_file);
                    let err_file = File::create(format!("{}.err", log_path)).unwrap();
                    let err = process::Stdio::from(err_file);

                    Ok(command.stdout(log).stderr(err).spawn().map_err(|_| OsalError::ExecError(Some(format!("{:?}",command))))?)
                }
            _ => {
                Ok(command.spawn().map_err(|_| OsalError::ExecError(Some(format!("{:?}",command))))?)
            }
        }
 */
    }
}
