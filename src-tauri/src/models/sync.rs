use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncAccount {
    pub id: String,
    pub provider: String,
    pub enabled: bool,
    pub expires_at: Option<i64>,
    pub config: Option<String>,
    pub status: String,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSyncAccountInput {
    pub provider: String,
    pub enabled: bool,
    pub config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncRunResult {
    pub provider: String,
    pub status: String,
    pub message: String,
    pub affected_tasks: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskSyncState {
    pub task_id: String,
    pub provider: String,
    pub sync_status: String,
    pub external_id: Option<String>,
    pub last_synced: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTaskSyncInput {
    pub task_id: String,
    pub provider: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleConnection {
    pub status: String,
    pub connected: bool,
    pub client_id: String,
    pub calendar_id: String,
    pub has_client_secret: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGoogleSettingsInput {
    pub client_id: String,
    pub calendar_id: Option<String>,
    pub client_secret: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveSyncConflictInput {
    pub task_id: String,
    pub provider: String,
    pub strategy: String,
}
