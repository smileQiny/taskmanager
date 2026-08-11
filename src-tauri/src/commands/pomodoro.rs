use tauri::State;

use crate::models::pomodoro::{
    CreatePomodoroSessionInput, PomodoroSession, PomodoroTimerSnapshot,
    UpdatePomodoroSessionRecordInput,
};
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

#[tauri::command]
pub fn get_pomodoro_timer(state: State<AppState>) -> Result<PomodoroTimerSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::get_timer(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_pomodoro_timer(state: State<AppState>) -> Result<PomodoroTimerSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::start_timer(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pause_pomodoro_timer(state: State<AppState>) -> Result<PomodoroTimerSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::pause_timer(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_pomodoro_timer(state: State<AppState>) -> Result<PomodoroTimerSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::reset_timer(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pomodoro_timer_duration(
    state: State<AppState>,
    minutes: i64,
) -> Result<PomodoroTimerSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::set_timer_duration(&conn, minutes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_pomodoro_timer(state: State<AppState>) -> Result<PomodoroTimerSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::complete_timer(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_pomodoro_session_record(
    state: State<AppState>,
    input: UpdatePomodoroSessionRecordInput,
) -> Result<Option<PomodoroSession>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    pomodoro_module::update_session_record(&conn, input).map_err(|e| e.to_string())
}
