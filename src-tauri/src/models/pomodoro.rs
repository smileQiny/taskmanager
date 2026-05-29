use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroSession {
    pub id: String,
    pub started_at: i64,
    pub duration: i64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePomodoroSessionInput {
    pub started_at: i64,
    pub duration: i64,
    pub completed: bool,
}
