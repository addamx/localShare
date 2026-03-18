use serde::Serialize;

#[derive(Debug)]
pub struct AuthService {
    token_ttl_minutes: u64,
}

impl AuthService {
    pub fn new(token_ttl_minutes: u64) -> Self {
        Self { token_ttl_minutes }
    }

    pub fn status(&self) -> AuthStatus {
        AuthStatus {
            token_ttl_minutes: self.token_ttl_minutes,
            rotation_enabled: true,
            bearer_header_name: "Authorization".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub token_ttl_minutes: u64,
    pub rotation_enabled: bool,
    pub bearer_header_name: String,
}
