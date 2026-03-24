use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::persistence::{ClipboardItemRecord, SessionStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<AppError>,
    pub ts: i64,
}

impl<T> ApiEnvelope<T> {
    pub fn ok(data: T, ts: i64) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            ts,
        }
    }

    pub fn err(error: AppError, ts: i64) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
            ts,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub service: String,
    pub status: String,
    pub bind_host: String,
    pub preferred_port: u16,
    pub effective_port: Option<u16>,
    pub database_ready: bool,
    pub session_ready: bool,
    pub mobile_base_path: String,
    pub health_endpoint: String,
    pub sse_endpoint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub device_name: String,
    pub public_host: String,
    pub public_port: u16,
    pub access_url: String,
    pub health_endpoint: String,
    pub sse_endpoint: String,
    pub mobile_base_path: String,
    pub session_id: String,
    pub session_status: SessionStatus,
    pub expires_at: i64,
    pub token_ttl_minutes: u64,
    pub bearer_header_name: String,
    pub token_query_key: String,
    pub rotation_enabled: bool,
    pub max_text_bytes: usize,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemSummary {
    pub id: String,
    pub preview: String,
    pub char_count: usize,
    pub source_kind: String,
    pub source_device_id: Option<String>,
    pub pinned: bool,
    pub is_current: bool,
    pub deleted_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemDetail {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub hash: String,
    pub preview: String,
    pub char_count: usize,
    pub source_kind: String,
    pub source_device_id: Option<String>,
    pub pinned: bool,
    pub is_current: bool,
    pub deleted_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardListResponse {
    pub items: Vec<ClipboardItemSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardWriteResponse {
    pub item: ClipboardItemDetail,
    pub created: bool,
    #[serde(rename = "reusedExisting")]
    pub reused_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardWriteRequest {
    pub content: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub activate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardPinRequest {
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateTokenResponse {
    pub session_id: String,
    pub access_url: String,
    pub expires_at: i64,
    pub session_status: SessionStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardDeleteResponse {
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardClearResponse {
    pub cleared_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEvent {
    pub kind: String,
    pub scope: String,
    pub item_id: Option<String>,
    pub ts: i64,
}

impl ServerEvent {
    pub fn refresh(scope: impl Into<String>, item_id: Option<String>, ts: i64) -> Self {
        Self {
            kind: "refresh".to_string(),
            scope: scope.into(),
            item_id,
            ts,
        }
    }
}

impl From<ClipboardItemRecord> for ClipboardItemSummary {
    fn from(value: ClipboardItemRecord) -> Self {
        Self {
            id: value.id,
            preview: value.preview,
            char_count: value.char_count,
            source_kind: value.source_kind.as_str().to_string(),
            source_device_id: value.source_device_id,
            pinned: value.pinned,
            is_current: value.is_current,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<ClipboardItemRecord> for ClipboardItemDetail {
    fn from(value: ClipboardItemRecord) -> Self {
        Self {
            id: value.id,
            content: value.content,
            content_type: value.content_type,
            hash: value.hash,
            preview: value.preview,
            char_count: value.char_count,
            source_kind: value.source_kind.as_str().to_string(),
            source_device_id: value.source_device_id,
            pinned: value.pinned,
            is_current: value.is_current,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
