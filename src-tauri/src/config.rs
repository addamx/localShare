use std::env;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub lan_host: String,
    pub preferred_port: u16,
    pub max_text_bytes: usize,
    pub clipboard_poll_interval_ms: u64,
    pub token_ttl_minutes: u64,
    pub database_file_name: String,
    pub mobile_route: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            lan_host: "0.0.0.0".to_string(),
            preferred_port: 8765,
            max_text_bytes: 65_536,
            clipboard_poll_interval_ms: 800,
            token_ttl_minutes: 30,
            database_file_name: "localshare.db".to_string(),
            mobile_route: "/m".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub app_dir: String,
    pub data_dir: String,
    pub database_path: String,
    pub logs_dir: String,
}

fn fallback_root() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".localshare")
}

pub fn resolve_app_paths(config: &RuntimeConfig) -> AppPaths {
    let root = ProjectDirs::from("com", "localshare", "LocalShare")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(fallback_root);

    let data_dir = root.join("data");
    let logs_dir = root.join("logs");
    let database_path = data_dir.join(&config.database_file_name);

    AppPaths {
        app_dir: root.to_string_lossy().into_owned(),
        data_dir: data_dir.to_string_lossy().into_owned(),
        database_path: database_path.to_string_lossy().into_owned(),
        logs_dir: logs_dir.to_string_lossy().into_owned(),
    }
}
