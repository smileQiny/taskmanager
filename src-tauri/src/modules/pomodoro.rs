use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::pomodoro::{
    CreatePomodoroSessionInput, PomodoroSession, PomodoroTimerCompletion, PomodoroTimerSnapshot,
    PomodoroTimerState, UpdatePomodoroSessionRecordInput,
};

const TIMER_ROW_ID: i64 = 1;
const DEFAULT_DURATION: i64 = 25 * 60;

#[derive(Debug, Clone)]
struct StoredPomodoroTimer {
    duration: i64,
    remaining: i64,
    running: bool,
    started_at: Option<i64>,
    updated_at: i64,
}

pub fn create(conn: &Connection, input: CreatePomodoroSessionInput) -> Result<PomodoroSession> {
    if input.duration <= 0 {
        bail!("duration must be positive");
    }
    let session = PomodoroSession {
        id: Uuid::new_v4().to_string(),
        started_at: input.started_at,
        duration: input.duration,
        completed: input.completed,
        note: normalize_optional_text(input.note),
        task_id: normalize_optional_text(input.task_id),
    };

    conn.execute(
        "INSERT INTO pomodoro_sessions (id, started_at, duration, completed, note, task_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session.id,
            session.started_at,
            session.duration,
            session.completed as i64,
            session.note,
            session.task_id,
        ],
    )?;

    Ok(session)
}

pub fn get_timer(conn: &Connection) -> Result<PomodoroTimerSnapshot> {
    sync_timer_at(conn, now_seconds())
}

pub fn start_timer(conn: &Connection) -> Result<PomodoroTimerSnapshot> {
    let now = now_seconds();
    let snapshot = sync_timer_at(conn, now)?;
    if snapshot.completion.is_some() {
        return start_timer(conn);
    }

    let timer = snapshot.timer;
    let remaining = timer.remaining.clamp(0, timer.duration);
    let started_at = now - (timer.duration - remaining);
    let next = StoredPomodoroTimer {
        duration: timer.duration,
        remaining,
        running: true,
        started_at: Some(started_at),
        updated_at: now,
    };
    save_timer(conn, &next)?;
    Ok(snapshot_from_stored(next, now, None))
}

pub fn pause_timer(conn: &Connection) -> Result<PomodoroTimerSnapshot> {
    let now = now_seconds();
    let snapshot = sync_timer_at(conn, now)?;
    if snapshot.completion.is_some() {
        return Ok(snapshot);
    }

    let timer = snapshot.timer;
    let next = StoredPomodoroTimer {
        duration: timer.duration,
        remaining: timer.remaining,
        running: false,
        started_at: None,
        updated_at: now,
    };
    save_timer(conn, &next)?;
    Ok(snapshot_from_stored(next, now, None))
}

pub fn reset_timer(conn: &Connection) -> Result<PomodoroTimerSnapshot> {
    let mut timer = read_timer(conn)?;
    let now = now_seconds();
    timer.remaining = timer.duration;
    timer.running = false;
    timer.started_at = None;
    timer.updated_at = now;
    save_timer(conn, &timer)?;
    Ok(snapshot_from_stored(timer, now, None))
}

pub fn set_timer_duration(conn: &Connection, minutes: i64) -> Result<PomodoroTimerSnapshot> {
    if !(5..=120).contains(&minutes) {
        bail!("pomodoro timer minutes must be between 5 and 120");
    }
    let now = now_seconds();
    let snapshot = sync_timer_at(conn, now)?;
    if snapshot.timer.running {
        return Ok(snapshot);
    }

    let duration = minutes * 60;
    let timer = StoredPomodoroTimer {
        duration,
        remaining: duration,
        running: false,
        started_at: None,
        updated_at: now,
    };
    save_timer(conn, &timer)?;
    Ok(snapshot_from_stored(timer, now, None))
}

pub fn complete_timer(conn: &Connection) -> Result<PomodoroTimerSnapshot> {
    let now = now_seconds();
    let timer = read_timer(conn)?;
    complete_stored_timer(conn, timer, now)
}

pub fn update_session_record(
    conn: &Connection,
    input: UpdatePomodoroSessionRecordInput,
) -> Result<Option<PomodoroSession>> {
    conn.execute(
        "UPDATE pomodoro_sessions
         SET note = ?2, task_id = ?3
         WHERE id = ?1",
        params![
            &input.id,
            normalize_optional_text(input.note),
            normalize_optional_text(input.task_id),
        ],
    )?;
    get_session_by_id(conn, &input.id)
}

pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<PomodoroSession>> {
    let safe_limit = limit.clamp(1, 50);
    let mut stmt = conn.prepare(
        "SELECT id, started_at, duration, completed, note, task_id
         FROM pomodoro_sessions
         ORDER BY started_at DESC
         LIMIT ?1",
    )?;
    let sessions = stmt
        .query_map(params![safe_limit], |row| {
            map_session_row(row)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(sessions)
}

fn get_session_by_id(conn: &Connection, id: &str) -> Result<Option<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, duration, completed, note, task_id
         FROM pomodoro_sessions
         WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], map_session_row)?;
    Ok(rows.next().transpose()?)
}

fn map_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PomodoroSession> {
    Ok(PomodoroSession {
        id: row.get(0)?,
        started_at: row.get(1)?,
        duration: row.get(2)?,
        completed: row.get::<_, i64>(3)? != 0,
        note: row.get(4)?,
        task_id: row.get(5)?,
    })
}

fn sync_timer_at(conn: &Connection, now: i64) -> Result<PomodoroTimerSnapshot> {
    let timer = read_timer(conn)?;
    if timer.running && remaining_at(&timer, now) <= 0 {
        return complete_stored_timer(conn, timer, now);
    }
    Ok(snapshot_from_stored(timer, now, None))
}

fn complete_stored_timer(
    conn: &Connection,
    timer: StoredPomodoroTimer,
    now: i64,
) -> Result<PomodoroTimerSnapshot> {
    let session = create(conn, CreatePomodoroSessionInput {
        started_at: timer.started_at.unwrap_or(now),
        duration: timer.duration,
        completed: true,
        note: None,
        task_id: None,
    })?;
    let next = StoredPomodoroTimer {
        duration: timer.duration,
        remaining: timer.duration,
        running: false,
        started_at: None,
        updated_at: now,
    };
    save_timer(conn, &next)?;
    Ok(snapshot_from_stored(next, now, Some(PomodoroTimerCompletion {
        session,
        completed_at: now,
        duration: timer.duration,
    })))
}

fn read_timer(conn: &Connection) -> Result<StoredPomodoroTimer> {
    let result = conn.query_row(
        "SELECT duration, remaining, running, started_at, updated_at
         FROM pomodoro_timer_state
         WHERE id = ?1",
        params![TIMER_ROW_ID],
        |row| {
            Ok(StoredPomodoroTimer {
                duration: row.get(0)?,
                remaining: row.get(1)?,
                running: row.get::<_, i64>(2)? != 0,
                started_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    );

    match result {
        Ok(timer) => Ok(normalize_stored_timer(timer)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let timer = StoredPomodoroTimer {
                duration: DEFAULT_DURATION,
                remaining: DEFAULT_DURATION,
                running: false,
                started_at: None,
                updated_at: now_seconds(),
            };
            save_timer(conn, &timer)?;
            Ok(timer)
        }
        Err(err) => Err(err.into()),
    }
}

fn save_timer(conn: &Connection, timer: &StoredPomodoroTimer) -> Result<()> {
    conn.execute(
        "INSERT INTO pomodoro_timer_state (id, duration, remaining, running, started_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
           duration = excluded.duration,
           remaining = excluded.remaining,
           running = excluded.running,
           started_at = excluded.started_at,
           updated_at = excluded.updated_at",
        params![
            TIMER_ROW_ID,
            timer.duration,
            timer.remaining,
            timer.running as i64,
            timer.started_at,
            timer.updated_at,
        ],
    )?;
    Ok(())
}

fn snapshot_from_stored(
    timer: StoredPomodoroTimer,
    now: i64,
    completion: Option<PomodoroTimerCompletion>,
) -> PomodoroTimerSnapshot {
    PomodoroTimerSnapshot {
        timer: PomodoroTimerState {
            duration: timer.duration,
            remaining: remaining_at(&timer, now),
            running: timer.running && remaining_at(&timer, now) > 0,
            started_at: timer.started_at,
            updated_at: timer.updated_at,
        },
        completion,
    }
}

fn normalize_stored_timer(timer: StoredPomodoroTimer) -> StoredPomodoroTimer {
    let duration = if timer.duration > 0 { timer.duration } else { DEFAULT_DURATION };
    let remaining = timer.remaining.clamp(0, duration);
    StoredPomodoroTimer {
        duration,
        remaining,
        running: timer.running,
        started_at: timer.started_at,
        updated_at: timer.updated_at,
    }
}

fn remaining_at(timer: &StoredPomodoroTimer, now: i64) -> i64 {
    if !timer.running {
        return timer.remaining.clamp(0, timer.duration);
    }
    let started_at = timer.started_at.unwrap_or(now);
    (timer.duration - (now - started_at)).clamp(0, timer.duration)
}

fn now_seconds() -> i64 {
    Utc::now().timestamp()
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
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
                note: None,
                task_id: None,
            },
        )
        .expect("first session should save");
        let latest = create(
            &conn,
            CreatePomodoroSessionInput {
                started_at: 200,
                duration: 900,
                completed: true,
                note: None,
                task_id: None,
            },
        )
        .expect("second session should save");

        let sessions = list_recent(&conn, 10).expect("sessions should list");

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, latest.id);
        assert_eq!(sessions[0].duration, 900);
    }

    #[test]
    fn updates_existing_session_record_note_and_task_link() {
        let conn = db::init_in_memory().expect("database should initialize");
        let session = create(
            &conn,
            CreatePomodoroSessionInput {
                started_at: 200,
                duration: 900,
                completed: true,
                note: None,
                task_id: None,
            },
        )
        .expect("session should save");

        let updated = update_session_record(
            &conn,
            UpdatePomodoroSessionRecordInput {
                id: session.id,
                note: Some("  补写周会纪要  ".to_string()),
                task_id: Some("task-1".to_string()),
            },
        )
        .expect("record should update")
        .expect("session should exist");

        assert_eq!(updated.note, Some("补写周会纪要".to_string()));
        assert_eq!(updated.task_id, Some("task-1".to_string()));
    }

    #[test]
    fn running_timer_remaining_is_derived_from_backend_time() {
        let conn = db::init_in_memory().expect("database should initialize");
        save_timer(&conn, &StoredPomodoroTimer {
            duration: 1500,
            remaining: 1500,
            running: true,
            started_at: Some(1_000),
            updated_at: 1_000,
        })
        .expect("timer should save");

        let snapshot = sync_timer_at(&conn, 1_090).expect("timer should sync");

        assert_eq!(snapshot.timer.remaining, 1410);
        assert!(snapshot.timer.running);
        assert!(snapshot.completion.is_none());
    }

    #[test]
    fn elapsed_timer_creates_one_completion_record() {
        let conn = db::init_in_memory().expect("database should initialize");
        save_timer(&conn, &StoredPomodoroTimer {
            duration: 60,
            remaining: 60,
            running: true,
            started_at: Some(1_000),
            updated_at: 1_000,
        })
        .expect("timer should save");

        let first = sync_timer_at(&conn, 1_061).expect("expired timer should sync");
        let second = sync_timer_at(&conn, 1_062).expect("reset timer should sync");
        let sessions = list_recent(&conn, 10).expect("sessions should list");

        assert_eq!(first.completion.expect("completion should exist").duration, 60);
        assert!(second.completion.is_none());
        assert_eq!(second.timer.remaining, 60);
        assert!(!second.timer.running);
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn changing_duration_does_not_interrupt_running_timer() {
        let conn = db::init_in_memory().expect("database should initialize");
        save_timer(&conn, &StoredPomodoroTimer {
            duration: 1500,
            remaining: 1500,
            running: true,
            started_at: Some(now_seconds() - 30),
            updated_at: now_seconds() - 30,
        })
        .expect("timer should save");

        let snapshot = set_timer_duration(&conn, 15).expect("duration request should be ignored");

        assert_eq!(snapshot.timer.duration, 1500);
        assert!(snapshot.timer.running);
        assert!(snapshot.timer.remaining <= 1470);
    }
}
