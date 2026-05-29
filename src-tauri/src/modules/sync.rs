use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::sync::{
    SetTaskSyncInput, SyncAccount, SyncRunResult, TaskSyncState, UpsertSyncAccountInput,
};

const PROVIDERS: [&str; 4] = ["feishu", "macos", "wecom", "google"];

pub fn list_accounts(conn: &Connection) -> Result<Vec<SyncAccount>> {
    ensure_default_accounts(conn)?;
    let mut stmt = conn.prepare(
        "SELECT id, provider, enabled, expires_at, config, status
         FROM sync_accounts
         ORDER BY CASE provider
            WHEN 'feishu' THEN 0
            WHEN 'macos' THEN 1
            WHEN 'wecom' THEN 2
            WHEN 'google' THEN 3
            ELSE 9
         END",
    )?;
    let accounts = stmt
        .query_map([], |row| {
            Ok(SyncAccount {
                id: row.get(0)?,
                provider: row.get(1)?,
                enabled: row.get::<_, i64>(2)? != 0,
                expires_at: row.get(3)?,
                config: row.get(4)?,
                status: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(accounts)
}

pub fn upsert_account(conn: &Connection, input: UpsertSyncAccountInput) -> Result<SyncAccount> {
    validate_provider(&input.provider)?;
    let status = if input.enabled { "pending_auth" } else { "not_configured" };
    conn.execute(
        "INSERT INTO sync_accounts (id, provider, enabled, config, status)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(provider) DO UPDATE SET
            enabled = excluded.enabled,
            config = excluded.config,
            status = excluded.status",
        params![
            Uuid::new_v4().to_string(),
            input.provider,
            input.enabled as i64,
            input.config,
            status,
        ],
    )?;
    get_account(conn, &input.provider)
}

pub fn sync_now(conn: &Connection, provider: String) -> Result<SyncRunResult> {
    validate_provider(&provider)?;
    ensure_default_accounts(conn)?;
    let account = get_account(conn, &provider)?;
    if !account.enabled {
        return Ok(SyncRunResult {
            provider,
            status: "not_configured".to_string(),
            message: "Provider is disabled. Enable it in Settings before syncing.".to_string(),
            affected_tasks: 0,
        });
    }
    let affected_tasks = mark_provider_mappings(conn, &account.provider, "needs_auth")?;
    let message = match account.provider.as_str() {
        "macos" => "macOS calendar sync bridge is prepared; EventKit authorization is required before live sync.",
        "feishu" => "Feishu sync requires OAuth credentials before live sync.",
        "wecom" => "WeCom sync requires CorpID and OAuth credentials before live sync.",
        "google" => "Google Calendar sync requires OAuth credentials before live sync.",
        _ => "Provider is not supported.",
    };
    Ok(SyncRunResult {
        provider: account.provider,
        status: "needs_auth".to_string(),
        message: message.to_string(),
        affected_tasks,
    })
}

pub fn list_task_sync_states(conn: &Connection, task_id: &str) -> Result<Vec<TaskSyncState>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, provider, sync_status, external_id, last_synced
         FROM sync_mappings
         WHERE task_id = ?1
         ORDER BY CASE provider
            WHEN 'feishu' THEN 0
            WHEN 'macos' THEN 1
            WHEN 'wecom' THEN 2
            WHEN 'google' THEN 3
            ELSE 9
         END",
    )?;
    let states = stmt
        .query_map(params![task_id], |row| {
            Ok(TaskSyncState {
                task_id: row.get(0)?,
                provider: row.get(1)?,
                sync_status: row.get(2)?,
                external_id: row.get(3)?,
                last_synced: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(states)
}

pub fn set_task_sync(conn: &Connection, input: SetTaskSyncInput) -> Result<Vec<TaskSyncState>> {
    validate_provider(&input.provider)?;
    if input.enabled {
        conn.execute(
            "INSERT INTO sync_mappings (id, task_id, provider, external_id, last_synced, sync_status)
             VALUES (?1, ?2, ?3, '', 0, 'pending')
             ON CONFLICT(task_id, provider) DO UPDATE SET sync_status = 'pending'",
            params![Uuid::new_v4().to_string(), input.task_id, input.provider],
        )?;
    } else {
        conn.execute(
            "DELETE FROM sync_mappings WHERE task_id = ?1 AND provider = ?2",
            params![input.task_id, input.provider],
        )?;
    }
    list_task_sync_states(conn, &input.task_id)
}

fn ensure_default_accounts(conn: &Connection) -> Result<()> {
    for provider in PROVIDERS {
        conn.execute(
            "INSERT OR IGNORE INTO sync_accounts (id, provider, enabled, status)
             VALUES (?1, ?2, 0, 'not_configured')",
            params![Uuid::new_v4().to_string(), provider],
        )?;
    }
    Ok(())
}

fn get_account(conn: &Connection, provider: &str) -> Result<SyncAccount> {
    let mut stmt = conn.prepare(
        "SELECT id, provider, enabled, expires_at, config, status
         FROM sync_accounts WHERE provider = ?1",
    )?;
    let account = stmt.query_row(params![provider], |row| {
        Ok(SyncAccount {
            id: row.get(0)?,
            provider: row.get(1)?,
            enabled: row.get::<_, i64>(2)? != 0,
            expires_at: row.get(3)?,
            config: row.get(4)?,
            status: row.get(5)?,
        })
    })?;
    Ok(account)
}

fn validate_provider(provider: &str) -> Result<()> {
    if PROVIDERS.contains(&provider) {
        return Ok(());
    }
    bail!("unsupported sync provider");
}

fn mark_provider_mappings(conn: &Connection, provider: &str, status: &str) -> Result<i64> {
    let rows = conn.execute(
        "UPDATE sync_mappings SET sync_status = ?2 WHERE provider = ?1",
        params![provider, status],
    )?;
    Ok(rows as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::CreateTaskInput;
    use crate::modules::db;
    use crate::modules::task as task_module;

    #[test]
    fn lists_default_provider_accounts() {
        let conn = db::init_in_memory().expect("database should initialize");
        let accounts = list_accounts(&conn).expect("accounts should list");

        assert_eq!(accounts.len(), 4);
        assert_eq!(accounts[0].provider, "feishu");
        assert!(accounts.iter().all(|account| !account.enabled));
    }

    #[test]
    fn upserts_provider_account_and_reports_auth_needed() {
        let conn = db::init_in_memory().expect("database should initialize");
        let account = upsert_account(
            &conn,
            UpsertSyncAccountInput {
                provider: "feishu".to_string(),
                enabled: true,
                config: Some("{\"calendar\":\"primary\"}".to_string()),
            },
        )
        .expect("account should upsert");

        assert!(account.enabled);
        assert_eq!(account.status, "pending_auth");

        let result = sync_now(&conn, "feishu".to_string()).expect("sync status should return");
        assert_eq!(result.status, "needs_auth");
        assert!(result.message.contains("OAuth"));
    }

    #[test]
    fn task_sync_mapping_can_be_enabled_and_marked_by_sync_run() {
        let conn = db::init_in_memory().expect("database should initialize");
        let task = task_module::create(
            &conn,
            CreateTaskInput {
                title: "Synced task".to_string(),
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
        .expect("task should be created");
        upsert_account(
            &conn,
            UpsertSyncAccountInput {
                provider: "google".to_string(),
                enabled: true,
                config: None,
            },
        )
        .expect("account should upsert");

        let states = set_task_sync(
            &conn,
            SetTaskSyncInput {
                task_id: task.id.clone(),
                provider: "google".to_string(),
                enabled: true,
            },
        )
        .expect("mapping should be enabled");

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].sync_status, "pending");

        let result = sync_now(&conn, "google".to_string()).expect("sync should update mapping state");
        assert_eq!(result.affected_tasks, 1);

        let states = list_task_sync_states(&conn, &task.id).expect("states should list");
        assert_eq!(states[0].sync_status, "needs_auth");
    }
}
