use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroSession {
    pub id: String,
    pub started_at: i64,
    pub duration: i64,
    pub completed: bool,
    pub note: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePomodoroSessionInput {
    pub started_at: i64,
    pub duration: i64,
    pub completed: bool,
    pub note: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePomodoroSessionRecordInput {
    pub id: String,
    pub note: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroTimerState {
    pub duration: i64,
    pub remaining: i64,
    pub running: bool,
    pub started_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroTimerCompletion {
    pub session: PomodoroSession,
    pub completed_at: i64,
    pub duration: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroTimerSnapshot {
    pub timer: PomodoroTimerState,
    pub completion: Option<PomodoroTimerCompletion>,
}
