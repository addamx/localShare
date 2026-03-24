#![allow(dead_code)]
#![allow(unused_imports)]

mod migrations;
mod models;
mod repository;

use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::types::{Type, Value};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::AppError;

pub use models::{
    AppSettingRecord, AuditAction, AuditLogRecord, ClipboardItemRecord, ClipboardListQuery,
    ClipboardSaveResult, CreateAuditLogInput, DeviceRecord, SaveClipboardItemInput, SessionRecord,
    SessionStatus, SourceKind,
};
pub use repository::PersistenceRepository;

const TEXT_CONTENT_TYPE: &str = "text/plain";
const PREVIEW_CHAR_LIMIT: usize = 120;

#[derive(Debug)]
pub struct PersistenceLayer {
    database_path: String,
    schema_version: i64,
    connection: Mutex<Connection>,
}

impl PersistenceLayer {
    pub fn new(database_path: String) -> Result<Self, AppError> {
        let mut connection = Connection::open(&database_path)?;
        connection.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            "#,
        )?;

        let schema_version = migrations::migrate(&mut connection)?;

        Ok(Self {
            database_path,
            schema_version,
            connection: Mutex::new(connection),
        })
    }

    pub fn status(&self) -> PersistenceStatus {
        PersistenceStatus {
            database_path: self.database_path.clone(),
            migrations_enabled: true,
            schema_version: self.schema_version,
            ready: true,
        }
    }

    pub fn hash_text(value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn lock_connection(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::State("persistence connection lock poisoned".to_string()))
    }
}

impl PersistenceRepository for PersistenceLayer {
    fn upsert_device(&self, name: &str) -> Result<DeviceRecord, AppError> {
        let normalized_name = name.trim();
        if normalized_name.is_empty() {
            return Err(AppError::Validation(
                "device name cannot be empty".to_string(),
            ));
        }

        let now = now_ms();
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        if let Some(existing) = query_device_by_name(&tx, normalized_name)? {
            tx.execute(
                "UPDATE devices SET updated_at = ?1 WHERE id = ?2",
                params![now, existing.id],
            )?;
            let updated = query_device_by_id(&tx, &existing.id)?
                .ok_or_else(|| AppError::NotFound("device disappeared after update".to_string()))?;
            tx.commit()?;
            return Ok(updated);
        }

        let device = DeviceRecord {
            id: new_id(),
            name: normalized_name.to_string(),
            created_at: now,
            updated_at: now,
        };

        tx.execute(
            "INSERT INTO devices (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![device.id, device.name, device.created_at, device.updated_at],
        )?;
        tx.commit()?;

        Ok(device)
    }

    fn get_device(&self, device_id: &str) -> Result<Option<DeviceRecord>, AppError> {
        let connection = self.lock_connection()?;
        query_device_by_id(&connection, device_id).map_err(AppError::from)
    }

    fn create_session(&self, token_hash: &str, expires_at: i64) -> Result<SessionRecord, AppError> {
        validate_token_hash(token_hash)?;

        let now = now_ms();
        if expires_at <= now {
            return Err(AppError::Validation(
                "session expires_at must be in the future".to_string(),
            ));
        }

        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        tx.execute(
            "UPDATE sessions
             SET status = ?1, rotated_at = COALESCE(rotated_at, ?2)
             WHERE status = ?3",
            params![
                SessionStatus::Expired.as_str(),
                now,
                SessionStatus::Active.as_str()
            ],
        )?;

        let session = SessionRecord {
            id: new_id(),
            token_hash: token_hash.to_string(),
            expires_at,
            status: SessionStatus::Active,
            created_at: now,
            rotated_at: None,
        };

        tx.execute(
            "INSERT INTO sessions (id, token_hash, expires_at, status, created_at, rotated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.token_hash,
                session.expires_at,
                session.status.as_str(),
                session.created_at,
                session.rotated_at
            ],
        )?;
        tx.commit()?;

        Ok(session)
    }

    fn get_current_session(&self, now_ms: i64) -> Result<Option<SessionRecord>, AppError> {
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;
        expire_active_sessions_internal(&tx, now_ms)?;
        let session = query_current_session(&tx, now_ms)?;
        tx.commit()?;
        Ok(session)
    }

    fn get_session_by_hash(
        &self,
        token_hash: &str,
        now_ms: i64,
    ) -> Result<Option<SessionRecord>, AppError> {
        validate_token_hash(token_hash)?;

        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;
        expire_active_sessions_internal(&tx, now_ms)?;

        let session = tx
            .query_row(
                "SELECT id, token_hash, expires_at, status, created_at, rotated_at
                 FROM sessions
                 WHERE token_hash = ?1 AND status = ?2 AND expires_at > ?3
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![token_hash, SessionStatus::Active.as_str(), now_ms],
                map_session_row,
            )
            .optional()?;

        tx.commit()?;
        Ok(session)
    }

    fn rotate_session(
        &self,
        current_session_id: &str,
        next_token_hash: &str,
        expires_at: i64,
        rotated_at: i64,
    ) -> Result<SessionRecord, AppError> {
        validate_token_hash(next_token_hash)?;
        if current_session_id.trim().is_empty() {
            return Err(AppError::Validation(
                "current_session_id cannot be empty".to_string(),
            ));
        }
        if expires_at <= rotated_at {
            return Err(AppError::Validation(
                "rotated session expires_at must be in the future".to_string(),
            ));
        }

        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        let updated_rows = tx.execute(
            "UPDATE sessions
             SET status = ?1, rotated_at = ?2
             WHERE id = ?3 AND status = ?4",
            params![
                SessionStatus::Rotated.as_str(),
                rotated_at,
                current_session_id,
                SessionStatus::Active.as_str()
            ],
        )?;

        if updated_rows == 0 {
            return Err(AppError::NotFound(format!(
                "active session `{current_session_id}` not found"
            )));
        }

        tx.execute(
            "UPDATE sessions
             SET status = ?1, rotated_at = ?2
             WHERE status = ?3",
            params![
                SessionStatus::Rotated.as_str(),
                rotated_at,
                SessionStatus::Active.as_str()
            ],
        )?;

        let session = SessionRecord {
            id: new_id(),
            token_hash: next_token_hash.to_string(),
            expires_at,
            status: SessionStatus::Active,
            created_at: rotated_at,
            rotated_at: None,
        };

        tx.execute(
            "INSERT INTO sessions (id, token_hash, expires_at, status, created_at, rotated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.token_hash,
                session.expires_at,
                session.status.as_str(),
                session.created_at,
                session.rotated_at
            ],
        )?;
        tx.commit()?;

        Ok(session)
    }

    fn expire_active_sessions(&self, now_ms: i64) -> Result<usize, AppError> {
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;
        let changed = expire_active_sessions_internal(&tx, now_ms)?;
        tx.commit()?;
        Ok(changed)
    }

    fn save_clipboard_item(
        &self,
        input: SaveClipboardItemInput,
        dedup_window_ms: i64,
        max_text_bytes: usize,
    ) -> Result<ClipboardSaveResult, AppError> {
        validate_clipboard_input(&input, max_text_bytes)?;

        let content_hash = Self::hash_text(&input.content);
        let preview = build_preview(&input.content);
        let char_count = input.content.chars().count();
        let now = now_ms();
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        if let Some(source_device_id) = &input.source_device_id {
            if query_device_by_id(&tx, source_device_id)?.is_none() {
                return Err(AppError::Validation(format!(
                    "source device `{source_device_id}` does not exist"
                )));
            }
        }

        if input.mark_current {
            tx.execute(
                "UPDATE clipboard_items
                 SET is_current = 0, updated_at = ?1
                 WHERE is_current = 1 AND deleted_at IS NULL",
                params![now],
            )?;
        }

        let dedup_threshold = now.saturating_sub(dedup_window_ms.max(0));
        if let Some(existing) =
            find_recent_clipboard_item_by_hash(&tx, &content_hash, dedup_threshold)?
        {
            let pinned_value = if existing.pinned || input.pinned {
                1
            } else {
                0
            };
            let current_value = if input.mark_current {
                1
            } else if existing.is_current {
                1
            } else {
                0
            };

            tx.execute(
                "UPDATE clipboard_items
                 SET pinned = ?1, is_current = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![pinned_value, current_value, now, existing.id],
            )?;

            let item = query_clipboard_item_by_id(&tx, &existing.id)?.ok_or_else(|| {
                AppError::NotFound("clipboard item disappeared after reuse".to_string())
            })?;
            tx.commit()?;

            return Ok(ClipboardSaveResult {
                item,
                created: false,
                reused_existing: true,
            });
        }

        let item = ClipboardItemRecord {
            id: new_id(),
            content: input.content,
            content_type: TEXT_CONTENT_TYPE.to_string(),
            hash: content_hash,
            preview,
            char_count,
            source_kind: input.source_kind,
            source_device_id: input.source_device_id,
            pinned: input.pinned,
            is_current: input.mark_current,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        };

        tx.execute(
            "INSERT INTO clipboard_items (
                id, content, content_type, hash, preview, char_count, source_kind,
                source_device_id, pinned, is_current, deleted_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                item.id,
                item.content,
                item.content_type,
                item.hash,
                item.preview,
                item.char_count as i64,
                item.source_kind.as_str(),
                item.source_device_id,
                bool_to_int(item.pinned),
                bool_to_int(item.is_current),
                item.deleted_at,
                item.created_at,
                item.updated_at
            ],
        )?;
        tx.commit()?;

        Ok(ClipboardSaveResult {
            item,
            created: true,
            reused_existing: false,
        })
    }

    fn list_clipboard_items(
        &self,
        query: &ClipboardListQuery,
    ) -> Result<Vec<ClipboardItemRecord>, AppError> {
        let connection = self.lock_connection()?;
        let mut sql = String::from(
            "SELECT id, content, content_type, hash, preview, char_count, source_kind,
                    source_device_id, pinned, is_current, deleted_at, created_at, updated_at
             FROM clipboard_items
             WHERE 1 = 1",
        );
        let mut values = Vec::<Value>::new();

        if !query.include_deleted {
            sql.push_str(" AND deleted_at IS NULL");
        }

        if query.pinned_only {
            sql.push_str(" AND pinned = 1");
        }

        if let Some(search) = query.search.as_ref().map(|value| value.trim()) {
            if !search.is_empty() {
                sql.push_str(" AND (content LIKE ? OR preview LIKE ?)");
                let pattern = format!("%{search}%");
                values.push(Value::from(pattern.clone()));
                values.push(Value::from(pattern));
            }
        }

        if let (Some(created_before), Some(before_id)) =
            (query.created_before, query.before_id.as_ref())
        {
            sql.push_str(" AND (created_at < ? OR (created_at = ? AND id < ?))");
            values.push(Value::from(created_before));
            values.push(Value::from(created_before));
            values.push(Value::from(before_id.clone()));
        }

        sql.push_str(" ORDER BY pinned DESC, created_at DESC, id DESC LIMIT ?");
        values.push(Value::from(clamp_limit(query.limit) as i64));

        let mut statement = connection.prepare(&sql)?;
        let rows =
            statement.query_map(rusqlite::params_from_iter(values), map_clipboard_item_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        Ok(items)
    }

    fn get_clipboard_item(&self, item_id: &str) -> Result<Option<ClipboardItemRecord>, AppError> {
        let connection = self.lock_connection()?;
        query_clipboard_item_by_id(&connection, item_id).map_err(AppError::from)
    }

    fn activate_clipboard_item(&self, item_id: &str) -> Result<ClipboardItemRecord, AppError> {
        validate_identifier(item_id, "item_id")?;

        let now = now_ms();
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        let target = query_clipboard_item_by_id(&tx, item_id)?
            .ok_or_else(|| AppError::NotFound(format!("clipboard item `{item_id}` not found")))?;

        tx.execute(
            "UPDATE clipboard_items
             SET is_current = 0, updated_at = ?1
             WHERE is_current = 1 AND deleted_at IS NULL",
            params![now],
        )?;
        tx.execute(
            "UPDATE clipboard_items
             SET is_current = 1, updated_at = ?1
             WHERE id = ?2 AND deleted_at IS NULL",
            params![now, item_id],
        )?;

        let item = query_clipboard_item_by_id(&tx, &target.id)?
            .ok_or_else(|| AppError::NotFound(format!("clipboard item `{item_id}` not found")))?;
        tx.commit()?;

        Ok(item)
    }

    fn update_clipboard_item_pin(
        &self,
        item_id: &str,
        pinned: bool,
    ) -> Result<ClipboardItemRecord, AppError> {
        validate_identifier(item_id, "item_id")?;

        let now = now_ms();
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        let changed = tx.execute(
            "UPDATE clipboard_items
             SET pinned = ?1, updated_at = ?2
             WHERE id = ?3 AND deleted_at IS NULL",
            params![bool_to_int(pinned), now, item_id],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound(format!(
                "clipboard item `{item_id}` not found"
            )));
        }

        let item = query_clipboard_item_by_id(&tx, item_id)?
            .ok_or_else(|| AppError::NotFound(format!("clipboard item `{item_id}` not found")))?;
        tx.commit()?;

        Ok(item)
    }

    fn soft_delete_clipboard_item(&self, item_id: &str) -> Result<(), AppError> {
        validate_identifier(item_id, "item_id")?;

        let now = now_ms();
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;

        let changed = tx.execute(
            "UPDATE clipboard_items
             SET deleted_at = ?1, is_current = 0, updated_at = ?2
             WHERE id = ?3 AND deleted_at IS NULL",
            params![now, now, item_id],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound(format!(
                "clipboard item `{item_id}` not found"
            )));
        }

        tx.commit()?;
        Ok(())
    }

    fn clear_clipboard_history(&self) -> Result<usize, AppError> {
        let now = now_ms();
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction()?;
        let changed = tx.execute(
            "UPDATE clipboard_items
             SET deleted_at = ?1, is_current = 0, updated_at = ?2
             WHERE deleted_at IS NULL",
            params![now, now],
        )?;
        tx.commit()?;

        Ok(changed)
    }

    fn append_audit_log(&self, input: CreateAuditLogInput) -> Result<AuditLogRecord, AppError> {
        let now = now_ms();
        let entry = AuditLogRecord {
            id: new_id(),
            action: input.action,
            item_id: input.item_id,
            ip: normalize_optional_text(input.ip),
            user_agent: normalize_optional_text(input.user_agent),
            reason: normalize_optional_text(input.reason),
            created_at: now,
        };

        let connection = self.lock_connection()?;
        connection.execute(
            "INSERT INTO audit_logs (id, action, item_id, ip, user_agent, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.action.as_str(),
                entry.item_id,
                entry.ip,
                entry.user_agent,
                entry.reason,
                entry.created_at
            ],
        )?;

        Ok(entry)
    }

    fn get_setting(&self, key: &str) -> Result<Option<AppSettingRecord>, AppError> {
        let normalized_key = normalize_setting_key(key)?;
        let connection = self.lock_connection()?;

        connection
            .query_row(
                "SELECT key, value, updated_at FROM app_settings WHERE key = ?1",
                params![normalized_key],
                map_setting_row,
            )
            .optional()
            .map_err(AppError::from)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<AppSettingRecord, AppError> {
        let normalized_key = normalize_setting_key(key)?;
        let now = now_ms();
        let setting = AppSettingRecord {
            key: normalized_key.to_string(),
            value: value.to_string(),
            updated_at: now,
        };

        let connection = self.lock_connection()?;
        connection.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![setting.key, setting.value, setting.updated_at],
        )?;

        Ok(setting)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceStatus {
    pub database_path: String,
    pub migrations_enabled: bool,
    pub schema_version: i64,
    pub ready: bool,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn clamp_limit(limit: usize) -> usize {
    limit.clamp(1, 200)
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_setting_key(key: &str) -> Result<&str, AppError> {
    let normalized = key.trim();
    if normalized.is_empty() {
        return Err(AppError::Validation(
            "setting key cannot be empty".to_string(),
        ));
    }

    Ok(normalized)
}

fn validate_identifier(value: &str, field_name: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "{field_name} cannot be empty"
        )));
    }

    Ok(())
}

fn validate_token_hash(token_hash: &str) -> Result<(), AppError> {
    if token_hash.trim().is_empty() {
        return Err(AppError::Validation(
            "token_hash cannot be empty".to_string(),
        ));
    }

    Ok(())
}

fn validate_clipboard_input(
    input: &SaveClipboardItemInput,
    max_text_bytes: usize,
) -> Result<(), AppError> {
    if input.content.trim().is_empty() {
        return Err(AppError::Validation(
            "clipboard content cannot be empty".to_string(),
        ));
    }

    let byte_len = input.content.as_bytes().len();
    if byte_len > max_text_bytes {
        return Err(AppError::Validation(format!(
            "clipboard content exceeds max size: {byte_len} > {max_text_bytes}"
        )));
    }

    Ok(())
}

fn build_preview(content: &str) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let total_chars = normalized.chars().count();
    if total_chars <= PREVIEW_CHAR_LIMIT {
        return normalized;
    }

    let mut preview = normalized
        .chars()
        .take(PREVIEW_CHAR_LIMIT)
        .collect::<String>();
    preview.push_str("...");
    preview
}

fn query_device_by_name(
    connection: &Connection,
    name: &str,
) -> rusqlite::Result<Option<DeviceRecord>> {
    connection
        .query_row(
            "SELECT id, name, created_at, updated_at FROM devices WHERE name = ?1 LIMIT 1",
            params![name],
            map_device_row,
        )
        .optional()
}

fn query_device_by_id(
    connection: &Connection,
    device_id: &str,
) -> rusqlite::Result<Option<DeviceRecord>> {
    connection
        .query_row(
            "SELECT id, name, created_at, updated_at FROM devices WHERE id = ?1 LIMIT 1",
            params![device_id],
            map_device_row,
        )
        .optional()
}

fn query_current_session(
    connection: &Connection,
    now_ms: i64,
) -> rusqlite::Result<Option<SessionRecord>> {
    connection
        .query_row(
            "SELECT id, token_hash, expires_at, status, created_at, rotated_at
             FROM sessions
             WHERE status = ?1 AND expires_at > ?2
             ORDER BY created_at DESC
             LIMIT 1",
            params![SessionStatus::Active.as_str(), now_ms],
            map_session_row,
        )
        .optional()
}

fn query_clipboard_item_by_id(
    connection: &Connection,
    item_id: &str,
) -> rusqlite::Result<Option<ClipboardItemRecord>> {
    connection
        .query_row(
            "SELECT id, content, content_type, hash, preview, char_count, source_kind,
                    source_device_id, pinned, is_current, deleted_at, created_at, updated_at
             FROM clipboard_items
             WHERE id = ?1 AND deleted_at IS NULL
             LIMIT 1",
            params![item_id],
            map_clipboard_item_row,
        )
        .optional()
}

fn find_recent_clipboard_item_by_hash(
    connection: &Connection,
    hash: &str,
    created_after: i64,
) -> rusqlite::Result<Option<ClipboardItemRecord>> {
    connection
        .query_row(
            "SELECT id, content, content_type, hash, preview, char_count, source_kind,
                    source_device_id, pinned, is_current, deleted_at, created_at, updated_at
             FROM clipboard_items
             WHERE hash = ?1 AND deleted_at IS NULL AND created_at >= ?2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            params![hash, created_after],
            map_clipboard_item_row,
        )
        .optional()
}

fn expire_active_sessions_internal(
    connection: &Connection,
    now_ms: i64,
) -> rusqlite::Result<usize> {
    connection.execute(
        "UPDATE sessions
         SET status = ?1, rotated_at = COALESCE(rotated_at, ?2)
         WHERE status = ?3 AND expires_at <= ?2",
        params![
            SessionStatus::Expired.as_str(),
            now_ms,
            SessionStatus::Active.as_str()
        ],
    )
}

fn map_device_row(row: &Row<'_>) -> rusqlite::Result<DeviceRecord> {
    Ok(DeviceRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn map_session_row(row: &Row<'_>) -> rusqlite::Result<SessionRecord> {
    let status: String = row.get(3)?;
    Ok(SessionRecord {
        id: row.get(0)?,
        token_hash: row.get(1)?,
        expires_at: row.get(2)?,
        status: parse_session_status(&status)?,
        created_at: row.get(4)?,
        rotated_at: row.get(5)?,
    })
}

fn map_clipboard_item_row(row: &Row<'_>) -> rusqlite::Result<ClipboardItemRecord> {
    let source_kind: String = row.get(6)?;
    let char_count: i64 = row.get(5)?;
    let pinned: i64 = row.get(8)?;
    let is_current: i64 = row.get(9)?;

    Ok(ClipboardItemRecord {
        id: row.get(0)?,
        content: row.get(1)?,
        content_type: row.get(2)?,
        hash: row.get(3)?,
        preview: row.get(4)?,
        char_count: char_count.max(0) as usize,
        source_kind: parse_source_kind(&source_kind)?,
        source_device_id: row.get(7)?,
        pinned: pinned != 0,
        is_current: is_current != 0,
        deleted_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn map_setting_row(row: &Row<'_>) -> rusqlite::Result<AppSettingRecord> {
    Ok(AppSettingRecord {
        key: row.get(0)?,
        value: row.get(1)?,
        updated_at: row.get(2)?,
    })
}

fn parse_session_status(value: &str) -> rusqlite::Result<SessionStatus> {
    SessionStatus::from_str(value).ok_or_else(|| invalid_enum_value("status", value))
}

fn parse_source_kind(value: &str) -> rusqlite::Result<SourceKind> {
    SourceKind::from_str(value).ok_or_else(|| invalid_enum_value("source_kind", value))
}

fn invalid_enum_value(column: &'static str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid {column} value `{value}`"),
        )),
    )
}
