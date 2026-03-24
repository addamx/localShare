use rusqlite::Connection;

pub const CURRENT_SCHEMA_VERSION: i64 = 1;

const INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    token_hash TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'rotated', 'expired')),
    created_at INTEGER NOT NULL,
    rotated_at INTEGER
);

CREATE TABLE IF NOT EXISTS clipboard_items (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_type TEXT NOT NULL CHECK (content_type = 'text/plain'),
    hash TEXT NOT NULL,
    preview TEXT NOT NULL,
    char_count INTEGER NOT NULL,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('desktop_local', 'mobile_web')),
    source_device_id TEXT,
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    is_current INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0, 1)),
    deleted_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(source_device_id) REFERENCES devices(id)
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    action TEXT NOT NULL CHECK (action IN ('create', 'activate', 'delete', 'rotate_token', 'reject')),
    item_id TEXT,
    ip TEXT,
    user_agent TEXT,
    reason TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(item_id) REFERENCES clipboard_items(id)
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_status_expires_at ON sessions(status, expires_at DESC);

CREATE INDEX IF NOT EXISTS idx_clipboard_items_hash ON clipboard_items(hash);
CREATE INDEX IF NOT EXISTS idx_clipboard_items_created_at_desc
    ON clipboard_items(created_at DESC, id DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_clipboard_items_pinned_created_at_desc
    ON clipboard_items(pinned DESC, created_at DESC, id DESC)
    WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_clipboard_items_single_current
    ON clipboard_items(is_current)
    WHERE is_current = 1 AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at_desc
    ON audit_logs(created_at DESC, id DESC);
"#;

pub fn migrate(connection: &mut Connection) -> rusqlite::Result<i64> {
    let tx = connection.transaction()?;
    let current_version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current_version < 1 {
        tx.execute_batch(INITIAL_SCHEMA)?;
        tx.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    }

    let schema_version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
    tx.commit()?;

    Ok(schema_version)
}
