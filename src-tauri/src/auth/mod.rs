use std::sync::{Arc, Mutex};

use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::persistence::{PersistenceLayer, PersistenceRepository, SessionRecord, SessionStatus};

const TOKEN_QUERY_KEY: &str = "token";

#[derive(Debug, Clone)]
pub struct AuthService {
    token_ttl_minutes: u64,
    runtime: Arc<Mutex<AuthRuntime>>,
}

#[derive(Debug, Default)]
struct AuthRuntime {
    current_session_id: Option<String>,
    current_token: Option<String>,
    current_token_hash: Option<String>,
    current_expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub token_ttl_minutes: u64,
    pub rotation_enabled: bool,
    pub bearer_header_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session_id: String,
    pub expires_at: i64,
    pub status: SessionStatus,
    pub access_url: String,
    pub public_host: String,
    pub public_port: u16,
    pub mobile_base_path: String,
    pub token_ttl_minutes: u64,
    pub bearer_header_name: String,
    pub token_query_key: String,
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub session: SessionRecord,
    pub token: String,
}

impl AuthService {
    pub fn new(token_ttl_minutes: u64) -> Self {
        Self {
            token_ttl_minutes,
            runtime: Arc::new(Mutex::new(AuthRuntime::default())),
        }
    }

    pub fn status(&self) -> AuthStatus {
        AuthStatus {
            token_ttl_minutes: self.token_ttl_minutes,
            rotation_enabled: true,
            bearer_header_name: "Authorization".to_string(),
        }
    }

    pub fn token_ttl_minutes(&self) -> u64 {
        self.token_ttl_minutes
    }

    pub fn token_ttl_ms(&self) -> i64 {
        self.token_ttl_minutes
            .saturating_mul(60_000)
            .min(i64::MAX as u64) as i64
    }

    pub fn hash_token(token: &str) -> String {
        PersistenceLayer::hash_text(token)
    }

    pub fn generate_token(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn ensure_session<R>(&self, repository: &R, now_ms: i64) -> Result<SessionContext, AppError>
    where
        R: PersistenceRepository,
    {
        {
            let runtime = self
                .runtime
                .lock()
                .map_err(|_| AppError::State("auth runtime lock poisoned".to_string()))?;
            if let (Some(session_id), Some(token), Some(expires_at)) = (
                runtime.current_session_id.as_ref(),
                runtime.current_token.as_ref(),
                runtime.current_expires_at,
            ) {
                if expires_at > now_ms {
                    if let Some(session) =
                        repository.get_session_by_hash(&Self::hash_token(token), now_ms)?
                    {
                        if session.id == *session_id {
                            return Ok(SessionContext {
                                session,
                                token: token.clone(),
                            });
                        }
                    }
                }
            }
        }

        let current = repository.get_current_session(now_ms)?;
        let token = self.generate_token();
        let token_hash = Self::hash_token(&token);
        let expires_at = now_ms.saturating_add(self.token_ttl_ms());

        let session = if let Some(existing) = current {
            if existing.expires_at <= now_ms || existing.status != SessionStatus::Active {
                repository.create_session(&token_hash, expires_at)?
            } else {
                repository.rotate_session(&existing.id, &token_hash, expires_at, now_ms)?
            }
        } else {
            repository.create_session(&token_hash, expires_at)?
        };

        self.cache_runtime(&session, token.clone(), token_hash)?;

        Ok(SessionContext { session, token })
    }

    pub fn rotate_session<R>(&self, repository: &R, now_ms: i64) -> Result<SessionContext, AppError>
    where
        R: PersistenceRepository,
    {
        let current = repository.get_current_session(now_ms)?.ok_or_else(|| {
            AppError::NotFound("no active session available to rotate".to_string())
        })?;

        let token = self.generate_token();
        let token_hash = Self::hash_token(&token);
        let expires_at = now_ms.saturating_add(self.token_ttl_ms());
        let session = repository.rotate_session(&current.id, &token_hash, expires_at, now_ms)?;

        self.cache_runtime(&session, token.clone(), token_hash)?;

        Ok(SessionContext { session, token })
    }

    pub fn validate_bearer_token<R>(
        &self,
        repository: &R,
        token: Option<&str>,
        now_ms: i64,
    ) -> Result<SessionRecord, AppError>
    where
        R: PersistenceRepository,
    {
        let token = token
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Unauthorized("missing bearer token".to_string()))?;

        let token_hash = Self::hash_token(token);
        repository
            .get_session_by_hash(&token_hash, now_ms)?
            .ok_or_else(|| AppError::Unauthorized("invalid or expired token".to_string()))
    }

    pub fn current_token(&self) -> Result<Option<String>, AppError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| AppError::State("auth runtime lock poisoned".to_string()))?;
        Ok(runtime.current_token.clone())
    }

    pub fn current_session_snapshot(
        &self,
        session: &SessionRecord,
        public_host: &str,
        public_port: u16,
        mobile_base_path: &str,
    ) -> Result<SessionSnapshot, AppError> {
        let token = self.current_token()?.ok_or_else(|| {
            AppError::State("current session token is not initialized".to_string())
        })?;

        Ok(SessionSnapshot {
            session_id: session.id.clone(),
            expires_at: session.expires_at,
            status: session.status,
            access_url: build_access_url(public_host, public_port, mobile_base_path, &token),
            public_host: public_host.to_string(),
            public_port,
            mobile_base_path: normalize_mobile_base_path(mobile_base_path),
            token_ttl_minutes: self.token_ttl_minutes,
            bearer_header_name: "Authorization".to_string(),
            token_query_key: TOKEN_QUERY_KEY.to_string(),
        })
    }

    fn cache_runtime(
        &self,
        session: &SessionRecord,
        token: String,
        token_hash: String,
    ) -> Result<(), AppError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| AppError::State("auth runtime lock poisoned".to_string()))?;
        runtime.current_session_id = Some(session.id.clone());
        runtime.current_token = Some(token);
        runtime.current_token_hash = Some(token_hash);
        runtime.current_expires_at = Some(session.expires_at);
        Ok(())
    }
}

pub fn build_access_url(
    public_host: &str,
    public_port: u16,
    mobile_base_path: &str,
    token: &str,
) -> String {
    let path = normalize_mobile_base_path(mobile_base_path);
    let separator = if path.contains('?') { '&' } else { '?' };
    format!(
        "http://{}:{}{}{}{}={}",
        public_host.trim(),
        public_port,
        path,
        separator,
        TOKEN_QUERY_KEY,
        token
    )
}

pub fn normalize_mobile_base_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/m".to_string();
    }

    let without_trailing = trimmed.trim_end_matches('/');
    if without_trailing.starts_with('/') {
        without_trailing.to_string()
    } else {
        format!("/{}", without_trailing)
    }
}
