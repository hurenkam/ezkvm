use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use log::debug;
use serde::{Deserialize, Serialize};

pub enum EzkvmError {
    OpenError { file: String },
    ReadError { file: String },
    WriteError { file: String },
    ExecError { file: String },
    DeleteError { file: String },
    ParseError { file: String },
    ResourceNotAvailable { pool: String }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Lock {
    name: String,
    pid: u32,
    resources: Vec<String>,
}

impl Lock {
    pub fn new(name: String, pid: u32, resources: Vec<String>) -> Self {
        Self {
            name,
            pid,
            resources
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_pid(&self) -> u32 {
        self.pid
    }

    pub fn get_resources(&self) -> Vec<String> {
        self.resources.clone()
    }

    pub fn add_resource(&mut self,id: String) {
        self.resources.push(id);
    }

    pub fn read(name: &str) -> Result<Self, EzkvmError> {
        let filename = format!("/var/ezkvm/lock/{}.yaml",name);
        let mut file = File::open(&filename).map_err(|_|EzkvmError::OpenError { file: filename.clone() })?;
        let mut contents = String::new();

        file.read_to_string(&mut contents).map_err(|_|EzkvmError::ReadError { file: filename.clone() })?;

        let result = serde_yaml::from_str(contents.as_str()).map_err(|_|EzkvmError::ParseError { file: filename.clone() })?;
        debug!("Lock::read({}): {:?}", name, result);

        Ok(result)
    }

    pub fn write(&self) -> Result<(), EzkvmError> {

        let filename = format!("/var/ezkvm/lock/{}.yaml",self.name);
        let yaml = serde_yaml::to_string(&self).map_err(|_|EzkvmError::ParseError { file: filename.clone() })?;
        debug!("Lock[{}].write(): {:?}",self.name, yaml);
        fs::write(&filename, yaml).map_err(|_|EzkvmError::WriteError { file: filename })?;

        Ok(())
    }

    pub fn delete(&self) -> Result<(), EzkvmError> {
        debug!("Lock[{}].delete()",self.name);
        let filename = format!("/var/ezkvm/lock/{}.yaml",self.name);
        fs::remove_file(&filename).map_err(|_|EzkvmError::DeleteError { file: filename })?;

        Ok(())
    }
}
