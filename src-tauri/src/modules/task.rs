use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::task::{CreateTaskInput, Task, UpdateTaskInput};

pub fn get_all(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, start_time, end_time,
                all_day, recurrence, tags, created_at, updated_at
         FROM tasks ORDER BY created_at DESC",
    )?;
    let tasks = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                start_time: row.get(5)?,
                end_time: row.get(6)?,
                all_day: row.get::<_, i64>(7)? != 0,
                recurrence: row.get(8)?,
                tags: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tasks)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, start_time, end_time,
                all_day, recurrence, tags, created_at, updated_at
         FROM tasks WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            all_day: row.get::<_, i64>(7)? != 0,
            recurrence: row.get(8)?,
            tags: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn create(conn: &Connection, input: CreateTaskInput) -> Result<Task> {
    let title = normalize_required_title(&input.title)?;
    let status = validate_status(input.status.as_deref().unwrap_or("todo"))?.to_string();
    let priority = validate_priority(input.priority.as_deref().unwrap_or("medium"))?.to_string();
    validate_time_range(input.start_time, input.end_time)?;

    let now = Utc::now().timestamp();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title,
        description: input.description,
        status,
        priority,
        start_time: input.start_time,
        end_time: input.end_time,
        all_day: input.all_day.unwrap_or(false),
        recurrence: input.recurrence,
        tags: input.tags,
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO tasks (id, title, description, status, priority, start_time,
                            end_time, all_day, recurrence, tags, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            task.id,
            task.title,
            task.description,
            task.status,
            task.priority,
            task.start_time,
            task.end_time,
            task.all_day as i64,
            task.recurrence,
            task.tags,
            task.created_at,
            task.updated_at
        ],
    )?;
    Ok(task)
}

pub fn update(conn: &Connection, input: UpdateTaskInput) -> Result<Option<Task>> {
    let existing = match get_by_id(conn, &input.id)? {
        Some(task) => task,
        None => return Ok(None),
    };

    let title = match input.title {
        Some(title) => normalize_required_title(&title)?,
        None => existing.title,
    };
    let description = input.description.unwrap_or(existing.description);
    let status = match input.status {
        Some(status) => validate_status(&status)?.to_string(),
        None => existing.status,
    };
    let priority = match input.priority {
        Some(priority) => validate_priority(&priority)?.to_string(),
        None => existing.priority,
    };
    let start_time = input.start_time.unwrap_or(existing.start_time);
    let end_time = input.end_time.unwrap_or(existing.end_time);
    validate_time_range(start_time, end_time)?;
    let all_day = input.all_day.unwrap_or(existing.all_day);
    let recurrence = input.recurrence.unwrap_or(existing.recurrence);
    let tags = input.tags.unwrap_or(existing.tags);
    let now = Utc::now().timestamp();
    let rows = conn.execute(
        "UPDATE tasks SET
            title = ?2,
            description = ?3,
            status = ?4,
            priority = ?5,
            start_time = ?6,
            end_time = ?7,
            all_day = ?8,
            recurrence = ?9,
            tags = ?10,
            updated_at = ?11
         WHERE id = ?1",
        params![
            input.id,
            title,
            description,
            status,
            priority,
            start_time,
            end_time,
            all_day as i64,
            recurrence,
            tags,
            now
        ],
    )?;
    if rows == 0 {
        return Ok(None);
    }
    get_by_id(conn, &input.id)
}

fn normalize_required_title(title: &str) -> Result<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        bail!("title cannot be blank");
    }
    Ok(trimmed.to_string())
}

fn validate_status(status: &str) -> Result<&str> {
    match status {
        "todo" | "in_progress" | "done" => Ok(status),
        _ => bail!("status must be todo, in_progress, or done"),
    }
}

fn validate_priority(priority: &str) -> Result<&str> {
    match priority {
        "low" | "medium" | "high" => Ok(priority),
        _ => bail!("priority must be low, medium, or high"),
    }
}

fn validate_time_range(start_time: Option<i64>, end_time: Option<i64>) -> Result<()> {
    if let (Some(start), Some(end)) = (start_time, end_time) {
        if end < start {
            bail!("end_time cannot be before start_time");
        }
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
    let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

pub fn get_by_date_range(conn: &Connection, from: i64, to: i64) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, start_time, end_time,
                all_day, recurrence, tags, created_at, updated_at
         FROM tasks
         WHERE (start_time >= ?1 AND start_time <= ?2)
            OR (end_time >= ?1 AND end_time <= ?2)
            OR (start_time <= ?1 AND end_time >= ?2)
         ORDER BY start_time ASC",
    )?;
    let tasks = stmt
        .query_map(params![from, to], |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                start_time: row.get(5)?,
                end_time: row.get(6)?,
                all_day: row.get::<_, i64>(7)? != 0,
                recurrence: row.get(8)?,
                tags: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db;

    fn memory_conn() -> Connection {
        db::init_in_memory().expect("database should initialize")
    }

    #[test]
    fn create_rejects_blank_title() {
        let conn = memory_conn();
        let err = create(
            &conn,
            CreateTaskInput {
                title: "   ".to_string(),
                description: None,
                status: None,
                priority: None,
                start_time: None,
                end_time: None,
                all_day: None,
                recurrence: None,
                tags: None,
            },
        )
        .expect_err("blank title should fail");

        assert!(err.to_string().contains("title"));
    }

    #[test]
    fn create_rejects_end_time_before_start_time() {
        let conn = memory_conn();
        let err = create(
            &conn,
            CreateTaskInput {
                title: "Timed task".to_string(),
                description: None,
                status: None,
                priority: None,
                start_time: Some(200),
                end_time: Some(100),
                all_day: None,
                recurrence: None,
                tags: None,
            },
        )
        .expect_err("invalid time range should fail");

        assert!(err.to_string().contains("end_time"));
    }

    #[test]
    fn update_can_clear_nullable_fields() {
        let conn = memory_conn();
        let created = create(
            &conn,
            CreateTaskInput {
                title: "Draft".to_string(),
                description: Some("notes".to_string()),
                status: None,
                priority: None,
                start_time: Some(100),
                end_time: Some(200),
                all_day: None,
                recurrence: None,
                tags: Some("work".to_string()),
            },
        )
        .expect("task should be created");

        let updated = update(
            &conn,
            UpdateTaskInput {
                id: created.id,
                title: None,
                description: Some(None),
                status: None,
                priority: None,
                start_time: Some(None),
                end_time: Some(None),
                all_day: None,
                recurrence: None,
                tags: Some(None),
            },
        )
        .expect("update should succeed")
        .expect("task should exist");

        assert_eq!(updated.description, None);
        assert_eq!(updated.start_time, None);
        assert_eq!(updated.end_time, None);
        assert_eq!(updated.tags, None);
    }
}
