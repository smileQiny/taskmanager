use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn init(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn init_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
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
        CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_mappings_task_provider ON sync_mappings(task_id, provider);
        CREATE TABLE IF NOT EXISTS sync_accounts (
            id         TEXT PRIMARY KEY,
            provider   TEXT NOT NULL UNIQUE,
            enabled    INTEGER NOT NULL DEFAULT 0,
            expires_at INTEGER,
            config     TEXT,
            status     TEXT NOT NULL DEFAULT 'not_configured'
        );
        CREATE INDEX IF NOT EXISTS idx_sync_accounts_provider ON sync_accounts(provider);
        CREATE TABLE IF NOT EXISTS pomodoro_sessions (
            id         TEXT PRIMARY KEY,
            started_at INTEGER NOT NULL,
            duration   INTEGER NOT NULL DEFAULT 1500,
            completed  INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS app_settings (
            key        TEXT PRIMARY KEY,
            value      TEXT NOT NULL
        );"
    )?;
    add_column_if_missing(conn, "sync_accounts", "enabled", "INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "sync_accounts", "status", "TEXT NOT NULL DEFAULT 'not_configured'")?;
    Ok(())
}

fn add_column_if_missing(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if columns.iter().any(|existing| existing == column) {
        return Ok(());
    }
    conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition};"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_sync_account_columns_to_existing_database() {
        let conn = Connection::open_in_memory().expect("database should open");
        conn.execute_batch(
            "CREATE TABLE sync_accounts (
                id         TEXT PRIMARY KEY,
                provider   TEXT NOT NULL UNIQUE,
                expires_at INTEGER,
                config     TEXT
            );",
        )
        .expect("legacy table should create");

        migrate(&conn).expect("migration should update legacy table");

        let mut stmt = conn
            .prepare("PRAGMA table_info(sync_accounts)")
            .expect("pragma should prepare");
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("columns should map")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("columns should collect");

        assert!(columns.contains(&"enabled".to_string()));
        assert!(columns.contains(&"status".to_string()));
    }
}
