use crate::error::AppError;

use super::models::{
    AppSettingRecord, AuditLogRecord, ClipboardItemRecord, ClipboardListQuery, ClipboardSaveResult,
    CreateAuditLogInput, DeviceRecord, SaveClipboardItemInput, SessionRecord,
};

pub trait PersistenceRepository {
    fn upsert_device(&self, name: &str) -> Result<DeviceRecord, AppError>;
    fn get_device(&self, device_id: &str) -> Result<Option<DeviceRecord>, AppError>;

    fn create_session(&self, token_hash: &str, expires_at: i64) -> Result<SessionRecord, AppError>;
    fn get_current_session(&self, now_ms: i64) -> Result<Option<SessionRecord>, AppError>;
    fn get_session_by_hash(
        &self,
        token_hash: &str,
        now_ms: i64,
    ) -> Result<Option<SessionRecord>, AppError>;
    fn rotate_session(
        &self,
        current_session_id: &str,
        next_token_hash: &str,
        expires_at: i64,
        rotated_at: i64,
    ) -> Result<SessionRecord, AppError>;
    fn expire_active_sessions(&self, now_ms: i64) -> Result<usize, AppError>;

    fn save_clipboard_item(
        &self,
        input: SaveClipboardItemInput,
        dedup_window_ms: i64,
        max_text_bytes: usize,
    ) -> Result<ClipboardSaveResult, AppError>;
    fn list_clipboard_items(
        &self,
        query: &ClipboardListQuery,
    ) -> Result<Vec<ClipboardItemRecord>, AppError>;
    fn get_clipboard_item(&self, item_id: &str) -> Result<Option<ClipboardItemRecord>, AppError>;
    fn activate_clipboard_item(&self, item_id: &str) -> Result<ClipboardItemRecord, AppError>;
    fn update_clipboard_item_pin(
        &self,
        item_id: &str,
        pinned: bool,
    ) -> Result<ClipboardItemRecord, AppError>;
    fn soft_delete_clipboard_item(&self, item_id: &str) -> Result<(), AppError>;
    fn clear_clipboard_history(&self) -> Result<usize, AppError>;

    fn append_audit_log(&self, input: CreateAuditLogInput) -> Result<AuditLogRecord, AppError>;

    fn get_setting(&self, key: &str) -> Result<Option<AppSettingRecord>, AppError>;
    fn set_setting(&self, key: &str, value: &str) -> Result<AppSettingRecord, AppError>;
}
