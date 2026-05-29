use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::pomodoro::{CreatePomodoroSessionInput, PomodoroSession};

pub fn create(conn: &Connection, input: CreatePomodoroSessionInput) -> Result<PomodoroSession> {
    if input.duration <= 0 {
        bail!("duration must be positive");
    }
    let session = PomodoroSession {
        id: Uuid::new_v4().to_string(),
        started_at: input.started_at,
        duration: input.duration,
        completed: input.completed,
    };

    conn.execute(
        "INSERT INTO pomodoro_sessions (id, started_at, duration, completed)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            session.id,
            session.started_at,
            session.duration,
            session.completed as i64,
        ],
    )?;

    Ok(session)
}

pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<PomodoroSession>> {
    let safe_limit = limit.clamp(1, 50);
    let mut stmt = conn.prepare(
        "SELECT id, started_at, duration, completed
         FROM pomodoro_sessions
         ORDER BY started_at DESC
         LIMIT ?1",
    )?;
    let sessions = stmt
        .query_map(params![safe_limit], |row| {
            Ok(PomodoroSession {
                id: row.get(0)?,
                started_at: row.get(1)?,
                duration: row.get(2)?,
                completed: row.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db;

    #[test]
    fn creates_and_lists_recent_sessions() {
        let conn = db::init_in_memory().expect("database should initialize");
        create(
            &conn,
            CreatePomodoroSessionInput {
                started_at: 100,
                duration: 1500,
                completed: true,
            },
        )
        .expect("first session should save");
        let latest = create(
            &conn,
            CreatePomodoroSessionInput {
                started_at: 200,
                duration: 900,
                completed: true,
            },
        )
        .expect("second session should save");

        let sessions = list_recent(&conn, 10).expect("sessions should list");

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, latest.id);
        assert_eq!(sessions[0].duration, 900);
    }
}
