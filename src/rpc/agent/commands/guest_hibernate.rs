use serde::Serialize;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuestHibernateRequest {
    execute: String,
}

impl GuestHibernateRequest {
    pub fn new() -> GuestHibernateRequest {
        GuestHibernateRequest {
            execute: "guest-suspend-disk".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_guest_sync_request() {
        let cmd = GuestHibernateRequest::new();
        let s = serde_json::to_string(&cmd).unwrap();
        println!("{}", s);
        assert_eq!(
            "{\"execute\":\"guest-suspend-disk\"}".to_string(),
            s
        );
    }
}
