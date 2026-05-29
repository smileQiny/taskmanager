use tauri::State;

use crate::models::sync::{
    SetTaskSyncInput, SyncAccount, SyncRunResult, TaskSyncState, UpsertSyncAccountInput,
};
use crate::modules::sync as sync_module;
use crate::AppState;

#[tauri::command]
pub fn list_sync_accounts(state: State<AppState>) -> Result<Vec<SyncAccount>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::list_accounts(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_sync_account(
    state: State<AppState>,
    input: UpsertSyncAccountInput,
) -> Result<SyncAccount, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::upsert_account(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_provider_now(
    state: State<AppState>,
    provider: String,
) -> Result<SyncRunResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::sync_now(&conn, provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_task_sync_states(
    state: State<AppState>,
    task_id: String,
) -> Result<Vec<TaskSyncState>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::list_task_sync_states(&conn, &task_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_task_sync(
    state: State<AppState>,
    input: SetTaskSyncInput,
) -> Result<Vec<TaskSyncState>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::set_task_sync(&conn, input).map_err(|e| e.to_string())
}
