mod commands;
mod models;
mod modules;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("taskmanager.db");
            let conn =
                modules::db::init(&db_path).expect("Failed to initialize database");
            app.manage(AppState {
                db: Mutex::new(conn),
                data_dir: app_dir,
                db_path,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_metadata,
            commands::task::get_tasks,
            commands::task::get_task,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::delete_task,
            commands::task::get_tasks_by_range,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::pomodoro::create_pomodoro_session,
            commands::pomodoro::list_pomodoro_sessions,
            commands::sync::list_sync_accounts,
            commands::sync::upsert_sync_account,
            commands::sync::sync_provider_now,
            commands::sync::list_task_sync_states,
            commands::sync::set_task_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
