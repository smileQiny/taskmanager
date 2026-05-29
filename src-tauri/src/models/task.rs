use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub all_day: bool,
    pub recurrence: Option<String>,
    pub tags: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub all_day: Option<bool>,
    pub recurrence: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub start_time: Option<Option<i64>>,
    pub end_time: Option<Option<i64>>,
    pub all_day: Option<bool>,
    pub recurrence: Option<Option<String>>,
    pub tags: Option<Option<String>>,
}
