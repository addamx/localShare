use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[allow(dead_code)]
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("state error: {0}")]
    State(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    code: &'static str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload = ErrorPayload {
            code: match self {
                AppError::Message(_) => "APP_ERROR",
                AppError::Io(_) => "IO_ERROR",
                AppError::Database(_) => "DATABASE_ERROR",
                AppError::Unauthorized(_) => "UNAUTHORIZED",
                AppError::Forbidden(_) => "FORBIDDEN",
                AppError::Validation(_) => "INVALID_ARGUMENT",
                AppError::NotFound(_) => "NOT_FOUND",
                AppError::Conflict(_) => "CONFLICT",
                AppError::RateLimited(_) => "RATE_LIMITED",
                AppError::State(_) => "STATE_ERROR",
            },
            message: self.to_string(),
        };
        payload.serialize(serializer)
    }
}

impl AppError {
    pub fn http_status_code(&self) -> u16 {
        match self {
            AppError::Message(_) => 500,
            AppError::Io(_) => 500,
            AppError::Database(_) => 500,
            AppError::Unauthorized(_) => 401,
            AppError::Forbidden(_) => 403,
            AppError::Validation(_) => 400,
            AppError::NotFound(_) => 404,
            AppError::Conflict(_) => 409,
            AppError::RateLimited(_) => 429,
            AppError::State(_) => 503,
        }
    }
}
