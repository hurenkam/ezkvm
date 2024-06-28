use std::fs;
use std::fs::File;
use std::io::Read;
use log::debug;
use serde::{Deserialize, Serialize};
use crate::types::EzkvmError;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Lock {
    #[serde(skip)]
    filename: String,
    pub pid: u32,
    pub resources: Vec<String>,
}

impl Lock {
    pub fn new(filename: String, pid: u32, resources: Vec<String>) -> Self {
        Self {
            filename,
            pid,
            resources
        }
    }

    pub fn read(filename: &str) -> Result<Self, EzkvmError> {
        let mut file = File::open(&filename).map_err(|_|EzkvmError::OpenError { file: filename.to_string() })?;
        let mut contents = String::new();

        file.read_to_string(&mut contents).map_err(|_|EzkvmError::ReadError { file: filename.to_string() })?;

        let mut result:Lock = serde_yaml::from_str(contents.as_str()).map_err(|_|EzkvmError::ParseError { file: filename.to_string() })?;
        result.filename = filename.to_string();
        debug!("Lock::read({}): {:?}", filename, result);

        Ok(result)
    }

    pub fn write(&self) -> Result<(), EzkvmError> {
        let yaml = serde_yaml::to_string(&self).map_err(|_|EzkvmError::ParseError { file: self.filename.clone() })?;
        debug!("Lock[{}].write(): {:?}", self.filename.clone(), yaml);
        fs::write(&self.filename, yaml).map_err(|_|EzkvmError::WriteError { file: self.filename.clone() })?;

        Ok(())
    }

    pub fn delete(&self) -> Result<(), EzkvmError> {
        debug!("Lock[{}].delete()",self.filename.clone());
        fs::remove_file(&self.filename).map_err(|_|EzkvmError::DeleteError { file: self.filename.clone() })?;

        Ok(())
    }
}
