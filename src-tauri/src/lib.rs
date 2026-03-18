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

use app_state::AppState;
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

    let state = AppState::new(runtime_config, paths);

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::app::get_bootstrap_context,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            tracing::info!(
                bind_host = %state.runtime_config.lan_host,
                preferred_port = state.runtime_config.preferred_port,
                db_path = %state.paths.database_path,
                "LocalShare scaffold initialized"
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LocalShare application");
}
