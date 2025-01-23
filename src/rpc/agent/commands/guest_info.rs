use crate::rpc::agent::{GuestAgentInfo, GuestAgentSupportedCommand};
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuestInfoRequest {
    execute: String,
}
impl GuestInfoRequest {
    pub fn new() -> GuestInfoRequest {
        GuestInfoRequest {
            execute: "guest-info".to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Getters)]
pub struct GuestInfoResponse {
    #[serde(rename = "return")]
    result: GuestInfoResponseResult,
}

#[allow(unused)]
impl GuestInfoResponse {
    pub fn new(
        version: String,
        supported_commands: Vec<GuestSupportedCommand>,
    ) -> GuestInfoResponse {
        GuestInfoResponse {
            result: GuestInfoResponseResult {
                version,
                supported_commands,
            },
        }
    }
}
impl From<GuestInfoResponse> for GuestAgentInfo {
    fn from(value: GuestInfoResponse) -> Self {
        let version = value.result.version.clone();
        let mut supported_commands = vec![];
        for command in value.result.supported_commands {
            supported_commands.push(command.into());
        }
        GuestAgentInfo::new(version, supported_commands)
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Getters)]
pub struct GuestInfoResponseResult {
    version: String,
    supported_commands: Vec<GuestSupportedCommand>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Getters)]
pub struct GuestSupportedCommand {
    enabled: bool,
    name: String,
    #[serde(rename = "success-response")]
    success_response: bool,
}
impl From<GuestSupportedCommand> for GuestAgentSupportedCommand {
    fn from(value: GuestSupportedCommand) -> Self {
        GuestAgentSupportedCommand::new(value.enabled, value.name, value.success_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_info_request() {
        let cmd = GuestInfoRequest::new();
        let s = serde_json::to_string(&cmd).unwrap();
        println!("{}", s);
        assert_eq!("{\"execute\":\"guest-info\"}".to_string(), s);
    }

    const RESPONSE: &str = r#"
{
    "return": {
        "version": "8.0.4",
        "supported_commands": [
            {"enabled": true, "name": "guest-get-osinfo", "success-response": true},
            {"enabled": true, "name": "guest-get-host-name", "success-response": true},
            {"enabled": true, "name": "guest-ping", "success-response": true},
            {"enabled": true, "name": "guest-sync", "success-response": true}
        ]
    }
}
    "#;

    #[test]
    fn test_guest_info_response() {
        let actual: GuestInfoResponse = serde_json::from_str(RESPONSE).unwrap();
        let version = "8.0.4".to_string();
        let responses = vec![
            GuestSupportedCommand {
                enabled: true,
                name: "guest-get-osinfo".to_string(),
                success_response: true,
            },
            GuestSupportedCommand {
                enabled: true,
                name: "guest-get-host-name".to_string(),
                success_response: true,
            },
            GuestSupportedCommand {
                enabled: true,
                name: "guest-ping".to_string(),
                success_response: true,
            },
            GuestSupportedCommand {
                enabled: true,
                name: "guest-sync".to_string(),
                success_response: true,
            },
        ];
        let expected: GuestInfoResponse = GuestInfoResponse::new(version, responses);
        assert_eq!(actual, expected);
    }
}
