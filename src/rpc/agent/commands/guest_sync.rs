use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuestSyncRequest {
    execute: String,
    arguments: GuestSyncRequestArguments,
}

impl GuestSyncRequest {
    pub fn new(id: u32) -> GuestSyncRequest {
        GuestSyncRequest {
            execute: "guest-sync".to_string(),
            arguments: GuestSyncRequestArguments { id },
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuestSyncRequestArguments {
    id: u32,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct GuestSyncResponse {
    #[serde(rename = "return")]
    pub result: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_guest_sync_request() {
        let cmd = GuestSyncRequest::new(1234);
        let s = serde_json::to_string(&cmd).unwrap();
        println!("{}", s);
        assert_eq!(
            "{\"execute\":\"guest-sync\",\"arguments\":{\"id\":1234}}".to_string(),
            s
        );
    }

    const RESPONSE: &str = r#"{ "return": 1234 }"#;
    #[test]
    fn test_guest_sync_response() {
        let actual: GuestSyncResponse = serde_json::from_str(RESPONSE).unwrap();
        let expected: GuestSyncResponse = GuestSyncResponse { result: 1234 };
        assert_eq!(actual, expected);
    }
}
