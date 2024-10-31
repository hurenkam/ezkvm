use derive_getters::Getters;
use log::debug;
use serde::{Deserialize, Serialize};
use crate::osal::OsalError;
#[mockall_double::double]
use crate::osal::Osal;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Getters)]
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
            resources,
        }
    }

    pub fn add_resource(&mut self, id: String) {
        self.resources.push(id);
    }

    pub fn read(name: &str) -> Result<Self, OsalError> {
        let filename = format!("/var/ezkvm/lock/{}.yaml", name);
        let content = Osal::read_file(filename.clone())?;
        let result =
            serde_yaml::from_str(content.as_str()).map_err(|_| OsalError::ParseError(Some(filename)))?;
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn write(&self) -> Result<(), OsalError> {
        let filename = format!("/var/ezkvm/lock/{}.yaml", self.name);
        let content = serde_yaml::to_string(&self).map_err(|_| OsalError::ParseError(Some(filename.clone())))?;

        Ok(Osal::write_file(filename, content)?)
    }

    #[allow(dead_code)]
    pub fn delete(&self) -> Result<(), OsalError> {
        debug!("Lock[{}].delete()", self.name);
        let filename = format!("/var/ezkvm/lock/{}.yaml", self.name);

        Ok(Osal::delete_file(filename)?)
    }
}

#[cfg(test)]
mod test {
    use serial_test::serial;
    use super::*;

    #[test]
    #[serial]
    pub fn read_empty_lock_file_results_in_parse_error() {
        let ctx = Osal::read_file_context();
        ctx.expect()
            .returning(|_path: String|
                Ok("".to_string())
            );

        let actual = Lock::read("name");
        let expectation = Err(OsalError::ParseError(Some("/var/ezkvm/lock/name.yaml".to_string())));
        assert_eq!(actual,expectation)
    }

    #[test]
    #[serial]
    pub fn read_valid_lock_file_succeeds() {
        let ctx = Osal::read_file_context();
        ctx.expect()
            .returning(|_path: String|
                Ok(r#"
                    name: "wakiza"
                    pid: 12345
                    resources:
                "#.to_string())
            );

        let actual = Lock::read("wakiza").unwrap();
        let expectation = Lock {
            name: "wakiza".to_string(),
            pid: 12345,
            resources: vec![],
        };
        assert_eq!(actual,expectation)
    }
}
