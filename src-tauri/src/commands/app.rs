use tauri::State;

use crate::models::settings::AppMetadata;
use crate::AppState;

#[tauri::command]
pub fn get_app_metadata(state: State<AppState>) -> AppMetadata {
    AppMetadata {
        data_dir: state.data_dir.display().to_string(),
        db_path: state.db_path.display().to_string(),
    }
}
