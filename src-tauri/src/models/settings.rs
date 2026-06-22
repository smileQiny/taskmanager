use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub theme: String,
    pub default_calendar_view: String,
    pub pomodoro_minutes: i64,
    pub cockpit_opacity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettingsInput {
    pub theme: Option<String>,
    pub default_calendar_view: Option<String>,
    pub pomodoro_minutes: Option<i64>,
    pub cockpit_opacity: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppMetadata {
    pub data_dir: String,
    pub db_path: String,
}
