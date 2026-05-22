use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn init(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id          TEXT PRIMARY KEY,
            title       TEXT NOT NULL,
            description TEXT,
            status      TEXT NOT NULL DEFAULT 'todo',
            priority    TEXT NOT NULL DEFAULT 'medium',
            start_time  INTEGER,
            end_time    INTEGER,
            all_day     INTEGER NOT NULL DEFAULT 0,
            recurrence  TEXT,
            tags        TEXT,
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_mappings (
            id          TEXT PRIMARY KEY,
            task_id     TEXT NOT NULL,
            provider    TEXT NOT NULL,
            external_id TEXT NOT NULL,
            last_synced INTEGER NOT NULL,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS sync_accounts (
            id         TEXT PRIMARY KEY,
            provider   TEXT NOT NULL UNIQUE,
            expires_at INTEGER,
            config     TEXT
        );
        CREATE TABLE IF NOT EXISTS pomodoro_sessions (
            id         TEXT PRIMARY KEY,
            started_at INTEGER NOT NULL,
            duration   INTEGER NOT NULL DEFAULT 1500,
            completed  INTEGER NOT NULL DEFAULT 0
        );"
    )?;
    Ok(())
}
