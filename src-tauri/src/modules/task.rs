use anyhow::Result;
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
    let now = Utc::now().timestamp();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title: input.title,
        description: input.description,
        status: input.status.unwrap_or_else(|| "todo".to_string()),
        priority: input.priority.unwrap_or_else(|| "medium".to_string()),
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
    let now = Utc::now().timestamp();
    let rows = conn.execute(
        "UPDATE tasks SET
            title = COALESCE(?2, title),
            description = COALESCE(?3, description),
            status = COALESCE(?4, status),
            priority = COALESCE(?5, priority),
            start_time = COALESCE(?6, start_time),
            end_time = COALESCE(?7, end_time),
            all_day = COALESCE(?8, all_day),
            recurrence = COALESCE(?9, recurrence),
            tags = COALESCE(?10, tags),
            updated_at = ?11
         WHERE id = ?1",
        params![
            input.id,
            input.title,
            input.description,
            input.status,
            input.priority,
            input.start_time,
            input.end_time,
            input.all_day.map(|b| b as i64),
            input.recurrence,
            input.tags,
            now
        ],
    )?;
    if rows == 0 {
        return Ok(None);
    }
    get_by_id(conn, &input.id)
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
