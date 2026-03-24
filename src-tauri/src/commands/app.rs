use tauri::State;

use crate::app_state::AppBootstrap;
use crate::app_state::AppState;
use crate::error::AppError;

#[tauri::command]
pub fn get_bootstrap_context(state: State<'_, AppState>) -> Result<AppBootstrap, AppError> {
    state.bootstrap()
}
