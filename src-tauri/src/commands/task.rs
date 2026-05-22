use tauri::State;

use crate::models::task::{CreateTaskInput, Task, UpdateTaskInput};
use crate::modules::task as task_module;
use crate::AppState;

#[tauri::command]
pub fn get_tasks(state: State<AppState>) -> Result<Vec<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::get_all(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task(state: State<AppState>, id: String) -> Result<Option<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::get_by_id(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task(state: State<AppState>, input: CreateTaskInput) -> Result<Task, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::create(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task(state: State<AppState>, input: UpdateTaskInput) -> Result<Option<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::update(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task(state: State<AppState>, id: String) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::delete(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tasks_by_range(
    state: State<AppState>,
    from: i64,
    to: i64,
) -> Result<Vec<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::get_by_date_range(&conn, from, to).map_err(|e| e.to_string())
}
