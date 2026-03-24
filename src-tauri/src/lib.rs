mod app_state;
mod auth;
mod clipboard;
mod commands;
mod config;
mod error;
mod http;
mod network;
mod persistence;

use std::fs;
use std::path::Path;
use std::time::Duration;

use tauri::{Emitter, Manager};

use app_state::AppState;
use clipboard::CLIPBOARD_REFRESH_EVENT_NAME;
use config::{resolve_app_paths, RuntimeConfig};
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .compact()
        .try_init();
}

fn ensure_app_dirs(paths: &config::AppPaths) -> Result<(), error::AppError> {
    for item in [&paths.app_dir, &paths.data_dir, &paths.logs_dir] {
        fs::create_dir_all(Path::new(item))?;
    }

    Ok(())
}

pub fn run() {
    init_tracing();

    let runtime_config = RuntimeConfig::default();
    let paths = resolve_app_paths(&runtime_config);

    if let Err(error) = ensure_app_dirs(&paths) {
        panic!("failed to prepare application directories: {error}");
    }

    let state = AppState::new(runtime_config, paths)
        .unwrap_or_else(|error| panic!("failed to initialize application state: {error}"));

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app::get_bootstrap_context,
            commands::workbench::list_clipboard_items,
            commands::workbench::get_clipboard_item,
            commands::workbench::activate_clipboard_item,
            commands::workbench::update_clipboard_item_pin,
            commands::workbench::delete_clipboard_item,
            commands::workbench::clear_clipboard_history,
            commands::workbench::rotate_session_token,
            commands::workbench::get_connectivity_report,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            state
                .clipboard
                .start(state.persistence.clone(), Some(state.device_id.clone()))
                .unwrap_or_else(|error| panic!("failed to start clipboard listener: {error}"));
            if let Err(error) = state.http_server.start(state.http_context()) {
                panic!("failed to start HTTP server: {error}");
            }
            if let Err(error) = state.http_server.wait_until_ready(Duration::from_secs(2)) {
                tracing::warn!(%error, "http server was not ready before UI bootstrap");
            }
            let clipboard_events = state.clipboard.subscribe();
            let app_handle = app.handle().clone();
            let http_server = state.http_server.clone();
            std::thread::Builder::new()
                .name("localshare-clipboard-events".to_string())
                .spawn(move || {
                    while let Ok(event) = clipboard_events.recv() {
                        http_server.publish_refresh("clipboard", Some(event.item_id.clone()));
                        if let Err(error) = app_handle.emit(CLIPBOARD_REFRESH_EVENT_NAME, &event) {
                            tracing::warn!(%error, "failed to emit clipboard refresh event");
                        }
                    }
                })
                .unwrap_or_else(|error| {
                    panic!("failed to start clipboard event bridge: {error}");
                });
            tracing::info!(
                bind_host = %state.runtime_config.lan_host,
                preferred_port = state.runtime_config.preferred_port,
                db_path = %state.paths.database_path,
                schema_version = state.persistence.status().schema_version,
                http_status = ?state.http_server.status(),
                "LocalShare scaffold initialized"
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LocalShare application");
}
