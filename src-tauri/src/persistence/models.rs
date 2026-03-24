use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Rotated,
    Expired,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Rotated => "rotated",
            SessionStatus::Expired => "expired",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "active" => Some(SessionStatus::Active),
            "rotated" => Some(SessionStatus::Rotated),
            "expired" => Some(SessionStatus::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    DesktopLocal,
    MobileWeb,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::DesktopLocal => "desktop_local",
            SourceKind::MobileWeb => "mobile_web",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "desktop_local" => Some(SourceKind::DesktopLocal),
            "mobile_web" => Some(SourceKind::MobileWeb),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Activate,
    Delete,
    RotateToken,
    Reject,
}

impl AuditAction {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditAction::Create => "create",
            AuditAction::Activate => "activate",
            AuditAction::Delete => "delete",
            AuditAction::RotateToken => "rotate_token",
            AuditAction::Reject => "reject",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "create" => Some(AuditAction::Create),
            "activate" => Some(AuditAction::Activate),
            "delete" => Some(AuditAction::Delete),
            "rotate_token" => Some(AuditAction::RotateToken),
            "reject" => Some(AuditAction::Reject),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: String,
    pub token_hash: String,
    pub expires_at: i64,
    pub status: SessionStatus,
    pub created_at: i64,
    pub rotated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemRecord {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub hash: String,
    pub preview: String,
    pub char_count: usize,
    pub source_kind: SourceKind,
    pub source_device_id: Option<String>,
    pub pinned: bool,
    pub is_current: bool,
    pub deleted_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogRecord {
    pub id: String,
    pub action: AuditAction,
    pub item_id: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub reason: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingRecord {
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveClipboardItemInput {
    pub content: String,
    pub source_kind: SourceKind,
    pub source_device_id: Option<String>,
    pub pinned: bool,
    pub mark_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSaveResult {
    pub item: ClipboardItemRecord,
    pub created: bool,
    pub reused_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardListQuery {
    pub search: Option<String>,
    pub pinned_only: bool,
    pub include_deleted: bool,
    pub created_before: Option<i64>,
    pub before_id: Option<String>,
    pub limit: usize,
}

impl Default for ClipboardListQuery {
    fn default() -> Self {
        Self {
            search: None,
            pinned_only: false,
            include_deleted: false,
            created_before: None,
            before_id: None,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAuditLogInput {
    pub action: AuditAction,
    pub item_id: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub reason: Option<String>,
}
