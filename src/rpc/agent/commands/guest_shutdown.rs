use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuestShutdownRequest {
    execute: String,
    arguments: GuestShutdownRequestArguments,
}

impl GuestShutdownRequest {
    pub fn new(mode: String) -> GuestShutdownRequest {
        GuestShutdownRequest {
            execute: "guest-shutdown".to_string(),
            arguments: GuestShutdownRequestArguments { mode },
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuestShutdownRequestArguments {
    mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_guest_sync_request() {
        let cmd = GuestShutdownRequest::new("powerdown".to_string());
        let s = serde_json::to_string(&cmd).unwrap();
        println!("{}", s);
        assert_eq!(
            "{\"execute\":\"guest-shutdown\",\"arguments\":{\"mode\":\"powerdown\"}}".to_string(),
            s
        );
    }
}
