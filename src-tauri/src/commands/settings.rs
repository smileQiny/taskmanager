use tauri::State;

use crate::models::settings::{AppSettings, UpdateSettingsInput};
use crate::modules::settings as settings_module;
use crate::AppState;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    settings_module::get(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(
    state: State<AppState>,
    input: UpdateSettingsInput,
) -> Result<AppSettings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    settings_module::update(&conn, input).map_err(|e| e.to_string())
}
