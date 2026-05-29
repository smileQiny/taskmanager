use tauri::State;

use crate::models::pomodoro::{CreatePomodoroSessionInput, PomodoroSession};
use crate::modules::pomodoro as pomodoro_module;
use crate::AppState;

#[tauri::command]
pub fn create_pomodoro_session(
    state: State<AppState>,
    input: CreatePomodoroSessionInput,
) -> Result<PomodoroSession, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::create(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_pomodoro_sessions(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<PomodoroSession>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::list_recent(&conn, limit).map_err(|e| e.to_string())
}
