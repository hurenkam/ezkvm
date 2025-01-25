use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct QmpCapabilitiesRequest {
    execute: String,
}

impl QmpCapabilitiesRequest {
    pub fn new() -> QmpCapabilitiesRequest {
        QmpCapabilitiesRequest {
            execute: "qmp_capabilities".to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct QmpCapabilitiesResponse {
    #[serde(rename = "return")]
    result: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_guest_sync_request() {
        let cmd = QmpCapabilitiesRequest::new();
        let s = serde_json::to_string(&cmd).unwrap();
        assert_eq!("{\"execute\":\"qmp_capabilities\"}".to_string(), s);
    }

    const RESPONSE: &str = r#"{ "return": {}}"#;
    #[test]
    fn test_guest_sync_response() {
        let actual: QmpCapabilitiesResponse = serde_json::from_str(RESPONSE).unwrap();
        let expected: QmpCapabilitiesResponse = QmpCapabilitiesResponse {
            result: HashMap::new(),
        };
        assert_eq!(actual, expected);
    }
}
