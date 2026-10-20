use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::models::sync::{GoogleConnection, SyncAccount, SyncRunResult, UpsertSyncAccountInput};
use crate::models::task::{Task, UpdateTaskInput};
use crate::modules::task as task_module;
use crate::modules::token_store::TokenStore;

pub const GOOGLE_PROVIDER: &str = "google";
const TASK_ID_PROPERTY: &str = "taskmanagerTaskId";
const TASK_UPDATED_PROPERTY: &str = "taskmanagerUpdatedAt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GooglePublicConfig {
    pub client_id: String,
    pub calendar_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventMoment {
    Date(String),
    DateTime(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDraft {
    pub summary: String,
    pub description: Option<String>,
    pub start: EventMoment,
    pub end: EventMoment,
    pub task_id: String,
    pub task_updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEvent {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub start_unix: Option<i64>,
    pub end_unix: Option<i64>,
    pub all_day: bool,
    pub updated_unix: i64,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    CreateRemote,
    PushLocal,
    PullRemote,
    AlreadySynced,
    Conflict,
}

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub task: Task,
    pub external_id: String,
    pub last_synced: i64,
    pub remote_hash: String,
}

#[derive(Debug, Clone)]
pub enum SyncOutcome {
    Clean {
        task_id: String,
        external_id: String,
        local_updated: i64,
        remote_updated: i64,
    },
    Pull {
        task: Task,
        remote: RemoteEvent,
    },
    Conflict {
        task_id: String,
    },
    Failed {
        task_id: String,
        message: String,
    },
}

#[async_trait]
pub trait CalendarApi: Send + Sync {
    async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<Option<RemoteEvent>>;
    async fn insert_event(&self, calendar_id: &str, draft: &EventDraft) -> Result<RemoteEvent>;
    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        draft: &EventDraft,
    ) -> Result<RemoteEvent>;

    /// Events overlapping `[start_unix, end_unix]` on one calendar.
    /// Providers that still sync from per-task mappings keep the empty default.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    async fn list_events(
        &self,
        _calendar_id: &str,
        _start_unix: i64,
        _end_unix: i64,
    ) -> Result<Vec<RemoteEvent>> {
        Ok(Vec::new())
    }
}

pub struct HttpCalendar {
    access_token: String,
}

impl HttpCalendar {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
        }
    }
}

#[async_trait]
impl CalendarApi for HttpCalendar {
    async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<Option<RemoteEvent>> {
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            form_path(calendar_id),
            form_path(event_id)
        );
        match request_json("GET", &url, &self.access_token, None).await {
            Ok(value) => parse_remote_event(&value).map(Some),
            Err(err) if is_not_found(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn insert_event(&self, calendar_id: &str, draft: &EventDraft) -> Result<RemoteEvent> {
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            form_path(calendar_id)
        );
        let value =
            request_json("POST", &url, &self.access_token, Some(draft_to_json(draft))).await?;
        parse_remote_event(&value)
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        draft: &EventDraft,
    ) -> Result<RemoteEvent> {
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            form_path(calendar_id),
            form_path(event_id)
        );
        let value = request_json(
            "PATCH",
            &url,
            &self.access_token,
            Some(draft_to_json(draft)),
        )
        .await?;
        parse_remote_event(&value)
    }
}

pub fn parse_public_google_config(raw: Option<&str>) -> GooglePublicConfig {
    let value = raw
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}));
    let client_id = value
        .get("client_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let calendar_id = value
        .get("calendar_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("primary")
        .to_string();
    GooglePublicConfig {
        client_id,
        calendar_id,
    }
}

pub fn serialize_public_google_config(config: &GooglePublicConfig) -> String {
    json!({
        "client_id": config.client_id,
        "calendar_id": config.calendar_id,
    })
    .to_string()
}

pub fn sanitize_provider_config(provider: &str, config: Option<String>) -> Option<String> {
    if provider != GOOGLE_PROVIDER {
        return config;
    }
    config.map(|value| serialize_public_google_config(&parse_public_google_config(Some(&value))))
}

pub fn task_to_event(task: &Task, offset_seconds: i32) -> EventDraft {
    let (start, end) = if task.all_day {
        let (start_date, end_date) = all_day_dates(task, offset_seconds);
        (EventMoment::Date(start_date), EventMoment::Date(end_date))
    } else {
        let start_unix = task.start_time.or(task.end_time).unwrap_or(task.created_at);
        let end_unix = task
            .end_time
            .filter(|end| *end > start_unix)
            .unwrap_or(start_unix + 3600);
        (
            EventMoment::DateTime(format_unix(start_unix)),
            EventMoment::DateTime(format_unix(end_unix)),
        )
    };
    EventDraft {
        summary: task.title.clone(),
        description: task.description.clone(),
        start,
        end,
        task_id: task.id.clone(),
        task_updated_at: task.updated_at,
    }
}

pub fn draft_to_json(draft: &EventDraft) -> Value {
    json!({
        "summary": draft.summary,
        "description": draft.description,
        "start": moment_json(&draft.start),
        "end": moment_json(&draft.end),
        "extendedProperties": {
            "private": {
                TASK_ID_PROPERTY: draft.task_id,
                TASK_UPDATED_PROPERTY: draft.task_updated_at.to_string(),
            }
        }
    })
}

pub fn parse_remote_event(value: &Value) -> Result<RemoteEvent> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Google event did not include an id"))?
        .to_string();
    let summary = value
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let description = value
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|text| !text.is_empty());
    let updated_unix = value
        .get("updated")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339)
        .unwrap_or(0);
    let (start_unix, end_unix, all_day) = parse_event_bounds(value);
    let task_id = value
        .pointer("/extendedProperties/private/taskmanagerTaskId")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(RemoteEvent {
        id,
        summary,
        description,
        start_unix,
        end_unix,
        all_day,
        updated_unix,
        task_id,
    })
}

pub fn decide_sync_action(
    local_updated: i64,
    remote_updated: Option<i64>,
    last_synced: i64,
) -> SyncAction {
    let Some(remote_updated) = remote_updated else {
        return SyncAction::CreateRemote;
    };
    let local_dirty = local_updated > last_synced;
    let remote_dirty = remote_updated > last_synced;
    match (local_dirty, remote_dirty) {
        (false, false) => SyncAction::AlreadySynced,
        (true, false) => SyncAction::PushLocal,
        (false, true) => SyncAction::PullRemote,
        (true, true) => SyncAction::Conflict,
    }
}

pub fn load_google_snapshots(conn: &Connection) -> Result<Vec<TaskSnapshot>> {
    load_provider_snapshots(conn, GOOGLE_PROVIDER)
}

pub fn load_provider_snapshots(conn: &Connection, provider: &str) -> Result<Vec<TaskSnapshot>> {
    let mut stmt = conn.prepare(
        "SELECT tasks.id, tasks.title, tasks.description, tasks.status, tasks.priority,
                tasks.start_time, tasks.end_time, tasks.all_day, tasks.recurrence, tasks.tags,
                tasks.created_at, tasks.updated_at, sync_mappings.external_id, sync_mappings.last_synced,
                sync_mappings.remote_hash
         FROM sync_mappings
         JOIN tasks ON tasks.id = sync_mappings.task_id
         WHERE sync_mappings.provider = ?1",
    )?;
    let snapshots = stmt
        .query_map(params![provider], |row| {
            Ok(TaskSnapshot {
                task: Task {
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
                },
                external_id: row.get(12)?,
                last_synced: row.get(13)?,
                remote_hash: row.get(14)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(snapshots)
}

pub async fn compute_outcomes(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshots: Vec<TaskSnapshot>,
) -> Result<Vec<SyncOutcome>> {
    let mut outcomes = Vec::new();
    for snapshot in snapshots {
        match outcome_for_snapshot(api, calendar_id, offset_seconds, &snapshot).await {
            Ok(outcome) => outcomes.push(outcome),
            Err(err) if is_auth_failure(&err) => return Err(err),
            Err(err) => outcomes.push(SyncOutcome::Failed {
                task_id: snapshot.task.id,
                message: err.to_string(),
            }),
        }
    }
    Ok(outcomes)
}

pub fn apply_outcomes(conn: &Connection, outcomes: &[SyncOutcome]) -> Result<SyncRunResult> {
    apply_provider_outcomes(conn, GOOGLE_PROVIDER, "Google Calendar", outcomes)
}

pub fn apply_provider_outcomes(
    conn: &Connection,
    provider: &str,
    label: &str,
    outcomes: &[SyncOutcome],
) -> Result<SyncRunResult> {
    let mut synced = 0_i64;
    let mut conflicts = 0_i64;
    let mut errors = 0_i64;
    let mut messages = Vec::new();
    for outcome in outcomes {
        match outcome {
            SyncOutcome::Clean {
                task_id,
                external_id,
                local_updated,
                remote_updated,
            } => {
                let last_synced = (*local_updated).max(*remote_updated);
                set_mapping(conn, provider, task_id, external_id, last_synced, "synced")?;
                synced += 1;
            }
            SyncOutcome::Pull { task, remote } => {
                let local_updated = apply_pull(conn, task, remote)?;
                let last_synced = local_updated.max(remote.updated_unix);
                set_mapping(conn, provider, &task.id, &remote.id, last_synced, "synced")?;
                synced += 1;
            }
            SyncOutcome::Conflict { task_id } => {
                set_mapping_status(conn, provider, task_id, "conflict")?;
                conflicts += 1;
            }
            SyncOutcome::Failed { task_id, message } => {
                set_mapping_status(conn, provider, task_id, "error")?;
                errors += 1;
                messages.push(format!("{task_id}: {message}"));
            }
        }
    }
    let (status, message) = if conflicts > 0 {
        (
            "conflict",
            if provider == GOOGLE_PROVIDER {
                format!("Google Calendar sync finished with {conflicts} conflict(s). Local and Google edits were both kept.")
            } else {
                format!("{label} sync finished with {conflicts} conflict(s). Local and {label} edits were both kept.")
            },
        )
    } else if errors > 0 {
        (
            "error",
            format!(
                "{label} sync finished with {errors} error(s). {}",
                messages.join("; ")
            ),
        )
    } else {
        ("synced", format!("{label} sync updated {synced} task(s)."))
    };
    let feedback = if conflicts > 0 || errors > 0 {
        Some(message.as_str())
    } else {
        None
    };
    set_provider_feedback(conn, provider, "authenticated", feedback)?;
    Ok(SyncRunResult {
        provider: provider.to_string(),
        status: status.to_string(),
        message,
        affected_tasks: synced + conflicts + errors,
    })
}

pub fn apply_pull(conn: &Connection, task: &Task, remote: &RemoteEvent) -> Result<i64> {
    let updated = task_module::update(
        conn,
        UpdateTaskInput {
            id: task.id.clone(),
            title: Some(if remote.summary.is_empty() {
                task.title.clone()
            } else {
                remote.summary.clone()
            }),
            description: Some(remote.description.clone()),
            status: None,
            priority: None,
            start_time: Some(remote.start_unix),
            end_time: Some(remote.end_unix),
            all_day: Some(remote.all_day),
            recurrence: None,
            tags: None,
        },
    )?
    .ok_or_else(|| anyhow!("task disappeared during Google sync"))?;
    Ok(updated.updated_at)
}

pub fn google_tokens_present(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM oauth_secrets WHERE id = 'google.tokens'",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn connection_view(
    conn: &Connection,
    store: &TokenStore,
    account: &SyncAccount,
) -> Result<GoogleConnection> {
    let config = parse_public_google_config(account.config.as_deref());
    let has_tokens = store.load_tokens(conn)?.is_some();
    let has_client_secret = store.load_client_secret(conn)?.is_some();
    Ok(GoogleConnection {
        connected: has_tokens && account.status == "authenticated",
        status: account.status.clone(),
        client_id: config.client_id,
        calendar_id: config.calendar_id,
        has_client_secret,
        last_error: account.last_error.clone(),
    })
}

pub fn save_settings(
    conn: &Connection,
    store: &TokenStore,
    input: &crate::models::sync::SaveGoogleSettingsInput,
    currently_enabled: bool,
) -> Result<()> {
    let existing = parse_public_google_config(account_config(conn)?.as_deref());
    let client_id = input.client_id.trim().to_string();
    let calendar_id = input
        .calendar_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.calendar_id.as_str())
        .to_string();
    if client_id != existing.client_id {
        store.delete_tokens(conn)?;
    }
    if let Some(secret) = &input.client_secret {
        if secret.trim().is_empty() {
            store.delete_client_secret(conn)?;
        } else {
            store.save_client_secret(conn, secret.trim())?;
        }
    }
    let enabled = input.enabled.unwrap_or(currently_enabled);
    let has_tokens = store.load_tokens(conn)?.is_some();
    let status = if !enabled {
        "not_configured"
    } else if client_id.is_empty() || !has_tokens {
        "needs_auth"
    } else {
        "authenticated"
    };
    let config = serialize_public_google_config(&GooglePublicConfig {
        client_id,
        calendar_id,
    });
    crate::modules::sync::upsert_account(
        conn,
        UpsertSyncAccountInput {
            provider: GOOGLE_PROVIDER.to_string(),
            enabled,
            config: Some(config),
        },
    )?;
    set_account_feedback(conn, status, None)
}

pub fn blocked_result(
    conn: &Connection,
    account: &SyncAccount,
    has_tokens: bool,
) -> Result<Option<SyncRunResult>> {
    if !account.enabled {
        return Ok(Some(SyncRunResult {
            provider: GOOGLE_PROVIDER.to_string(),
            status: "not_configured".to_string(),
            message: "Provider is disabled. Enable it in Settings before syncing.".to_string(),
            affected_tasks: 0,
        }));
    }
    let config = parse_public_google_config(account.config.as_deref());
    if config.client_id.is_empty() || !has_tokens {
        let affected = mark_mappings(conn, GOOGLE_PROVIDER, "needs_auth")?;
        let message = if config.client_id.is_empty() {
            "Google Calendar sync requires an OAuth Client ID before live sync."
        } else {
            "Google Calendar sync requires OAuth authorization before live sync."
        };
        set_account_feedback(conn, "needs_auth", Some(message))?;
        return Ok(Some(SyncRunResult {
            provider: GOOGLE_PROVIDER.to_string(),
            status: "needs_auth".to_string(),
            message: message.to_string(),
            affected_tasks: affected,
        }));
    }
    Ok(None)
}

pub fn mark_needs_auth(conn: &Connection, message: &str) -> Result<SyncRunResult> {
    let affected = mark_mappings(conn, GOOGLE_PROVIDER, "needs_auth")?;
    set_account_feedback(conn, "needs_auth", Some(message))?;
    Ok(SyncRunResult {
        provider: GOOGLE_PROVIDER.to_string(),
        status: "needs_auth".to_string(),
        message: message.to_string(),
        affected_tasks: affected,
    })
}

async fn outcome_for_snapshot(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshot: &TaskSnapshot,
) -> Result<SyncOutcome> {
    let remote = if snapshot.external_id.is_empty() {
        None
    } else {
        api.get_event(calendar_id, &snapshot.external_id).await?
    };
    let action = if remote.is_none() {
        SyncAction::CreateRemote
    } else {
        decide_sync_action(
            snapshot.task.updated_at,
            remote.as_ref().map(|event| event.updated_unix),
            snapshot.last_synced,
        )
    };
    match action {
        SyncAction::AlreadySynced => Ok(SyncOutcome::Clean {
            task_id: snapshot.task.id.clone(),
            external_id: snapshot.external_id.clone(),
            local_updated: snapshot.last_synced,
            remote_updated: snapshot.last_synced,
        }),
        SyncAction::Conflict => Ok(SyncOutcome::Conflict {
            task_id: snapshot.task.id.clone(),
        }),
        SyncAction::PullRemote => {
            let remote = remote.ok_or_else(|| anyhow!("pull requires a remote event"))?;
            Ok(SyncOutcome::Pull {
                task: snapshot.task.clone(),
                remote,
            })
        }
        SyncAction::CreateRemote | SyncAction::PushLocal => {
            let draft = task_to_event(&snapshot.task, offset_seconds);
            let saved = if snapshot.external_id.is_empty() || remote.is_none() {
                api.insert_event(calendar_id, &draft).await?
            } else {
                api.update_event(calendar_id, &snapshot.external_id, &draft)
                    .await?
            };
            Ok(SyncOutcome::Clean {
                task_id: snapshot.task.id.clone(),
                external_id: saved.id,
                local_updated: snapshot.task.updated_at,
                remote_updated: saved.updated_unix,
            })
        }
    }
}

pub async fn plan_resolution(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshot: TaskSnapshot,
    strategy: &str,
) -> Result<SyncOutcome> {
    let forced = match strategy {
        "local" => SyncAction::PushLocal,
        "remote" => SyncAction::PullRemote,
        _ => bail!("conflict strategy must be local or remote"),
    };
    let remote = if snapshot.external_id.is_empty() {
        None
    } else {
        api.get_event(calendar_id, &snapshot.external_id).await?
    };
    match forced {
        SyncAction::PullRemote => {
            let Some(remote) = remote else {
                bail!("Google event no longer exists");
            };
            Ok(SyncOutcome::Pull {
                task: snapshot.task,
                remote,
            })
        }
        _ => {
            let draft = task_to_event(&snapshot.task, offset_seconds);
            let saved = if snapshot.external_id.is_empty() || remote.is_none() {
                api.insert_event(calendar_id, &draft).await?
            } else {
                api.update_event(calendar_id, &snapshot.external_id, &draft)
                    .await?
            };
            Ok(SyncOutcome::Clean {
                task_id: snapshot.task.id,
                external_id: saved.id,
                local_updated: snapshot.task.updated_at,
                remote_updated: saved.updated_unix,
            })
        }
    }
}

fn account_config(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT config FROM sync_accounts WHERE provider = 'google'")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(row.get(0)?)
}

pub fn set_account_feedback(
    conn: &Connection,
    status: &str,
    last_error: Option<&str>,
) -> Result<()> {
    set_provider_feedback(conn, GOOGLE_PROVIDER, status, last_error)
}

pub fn set_provider_feedback(
    conn: &Connection,
    provider: &str,
    status: &str,
    last_error: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_accounts SET status = ?2, last_error = ?3 WHERE provider = ?1",
        params![provider, status, last_error],
    )?;
    Ok(())
}

fn set_mapping(
    conn: &Connection,
    provider: &str,
    task_id: &str,
    external_id: &str,
    last_synced: i64,
    status: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_mappings
         SET external_id = ?3, last_synced = ?4, sync_status = ?5
         WHERE task_id = ?1 AND provider = ?2",
        params![task_id, provider, external_id, last_synced, status],
    )?;
    Ok(())
}

fn set_mapping_status(
    conn: &Connection,
    provider: &str,
    task_id: &str,
    status: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE sync_mappings SET sync_status = ?3 WHERE task_id = ?1 AND provider = ?2",
        params![task_id, provider, status],
    )?;
    Ok(())
}

fn mark_mappings(conn: &Connection, provider: &str, status: &str) -> Result<i64> {
    let rows = conn.execute(
        "UPDATE sync_mappings SET sync_status = ?2 WHERE provider = ?1",
        params![provider, status],
    )?;
    Ok(rows as i64)
}

fn all_day_dates(task: &Task, offset_seconds: i32) -> (String, String) {
    let start_unix = task.start_time.unwrap_or(task.created_at);
    let start_date = unix_to_date(start_unix, offset_seconds);
    let end_date = match task.end_time {
        Some(end_unix) => {
            let end_date = unix_to_date(end_unix, offset_seconds);
            if end_date <= start_date {
                next_date(&start_date)
            } else {
                end_date
            }
        }
        None => next_date(&start_date),
    };
    (start_date, end_date)
}

fn unix_to_date(unix: i64, offset_seconds: i32) -> String {
    offset(offset_seconds)
        .timestamp_opt(unix, 0)
        .single()
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

fn next_date(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .and_then(|value| value.checked_add_signed(Duration::days(1)))
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| date.to_string())
}

fn format_unix(unix: i64) -> String {
    Utc.timestamp_opt(unix, 0)
        .single()
        .map(|date| date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

fn offset(seconds: i32) -> FixedOffset {
    FixedOffset::east_opt(seconds).unwrap_or(FixedOffset::east_opt(0).expect("utc offset"))
}

fn moment_json(moment: &EventMoment) -> Value {
    match moment {
        EventMoment::Date(date) => json!({ "date": date }),
        EventMoment::DateTime(date_time) => json!({ "dateTime": date_time, "timeZone": "UTC" }),
    }
}

fn parse_event_bounds(value: &Value) -> (Option<i64>, Option<i64>, bool) {
    let start = value.get("start");
    let end = value.get("end");
    if let Some(date) = start
        .and_then(|item| item.get("date"))
        .and_then(Value::as_str)
    {
        let start_unix = date_to_unix(date);
        let end_unix = end
            .and_then(|item| item.get("date"))
            .and_then(Value::as_str)
            .and_then(date_to_unix);
        return (start_unix, end_unix, true);
    }
    let start_unix = start
        .and_then(|item| item.get("dateTime"))
        .and_then(Value::as_str)
        .and_then(parse_rfc3339);
    let end_unix = end
        .and_then(|item| item.get("dateTime"))
        .and_then(Value::as_str)
        .and_then(parse_rfc3339);
    (start_unix, end_unix, false)
}

fn date_to_unix(date: &str) -> Option<i64> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    Some(Utc.from_utc_datetime(&naive).timestamp())
}

fn parse_rfc3339(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.timestamp())
}

fn form_path(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

async fn request_json(
    method: &str,
    url: &str,
    access_token: &str,
    body: Option<Value>,
) -> Result<Value> {
    let client = reqwest::Client::new();
    let request = client
        .request(method.parse().unwrap_or(reqwest::Method::GET), url)
        .bearer_auth(access_token);
    let request = if let Some(body) = body {
        request.json(&body)
    } else {
        request
    };
    let response = request.send().await?;
    let status = response.status();
    let text = response.text().await?;
    if status.as_u16() == 404 {
        bail!("google api status 404");
    }
    if status.as_u16() == 401 || status.as_u16() == 403 {
        bail!("google api status {}", status.as_u16());
    }
    if !status.is_success() {
        bail!("google api status {}: {text}", status.as_u16());
    }
    Ok(serde_json::from_str(&text)?)
}

fn is_not_found(err: &anyhow::Error) -> bool {
    err.to_string().contains("status 404")
}

fn is_auth_failure(err: &anyhow::Error) -> bool {
    let text = err.to_string();
    text.contains("status 401") || text.contains("status 403") || text.contains("invalid_grant")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sync::SyncRunResult;
    use crate::models::task::{CreateTaskInput, Task};
    use crate::modules::db;
    use crate::modules::sync;
    use chrono::DateTime;
    use rusqlite::{params, Connection};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn sample_task(all_day: bool) -> Task {
        let start = DateTime::parse_from_rfc3339("2026-10-17T09:00:00Z")
            .unwrap()
            .timestamp();
        let end = DateTime::parse_from_rfc3339("2026-10-17T10:00:00Z")
            .unwrap()
            .timestamp();
        Task {
            id: "task-1".to_string(),
            title: "Plan sprint".to_string(),
            description: Some("Notes".to_string()),
            status: "todo".to_string(),
            priority: "medium".to_string(),
            start_time: Some(start),
            end_time: Some(end),
            all_day,
            recurrence: None,
            tags: None,
            created_at: start,
            updated_at: 200,
        }
    }

    #[test]
    fn maps_title_times_and_task_id_onto_a_google_event() {
        let draft = task_to_event(&sample_task(false), 0);
        let json = draft_to_json(&draft);
        assert_eq!(json["summary"], "Plan sprint");
        assert_eq!(json["start"]["dateTime"], "2026-10-17T09:00:00Z");
        assert_eq!(json["end"]["dateTime"], "2026-10-17T10:00:00Z");
        assert_eq!(
            json["extendedProperties"]["private"][TASK_ID_PROPERTY],
            "task-1"
        );
    }

    #[test]
    fn maps_all_day_tasks_to_exclusive_google_dates() {
        let mut task = sample_task(true);
        task.end_time = None;
        let draft = task_to_event(&task, 0);
        assert_eq!(draft.start, EventMoment::Date("2026-10-17".to_string()));
        assert_eq!(draft.end, EventMoment::Date("2026-10-18".to_string()));
    }

    #[test]
    fn parses_a_google_event_back_into_task_fields() {
        let value = json!({
            "id": "evt-1",
            "summary": "Plan sprint",
            "description": "Notes",
            "updated": "2026-10-17T12:00:00Z",
            "start": { "dateTime": "2026-10-17T09:00:00Z" },
            "end": { "dateTime": "2026-10-17T10:00:00Z" },
            "extendedProperties": { "private": { "taskmanagerTaskId": "task-1" } }
        });
        let event = parse_remote_event(&value).unwrap();
        assert_eq!(event.id, "evt-1");
        assert_eq!(event.task_id.as_deref(), Some("task-1"));
        assert!(!event.all_day);
        assert_eq!(event.summary, "Plan sprint");
    }

    #[test]
    fn conflict_keeps_both_sides_when_each_changed_after_sync() {
        assert_eq!(
            decide_sync_action(300, Some(300), 300),
            SyncAction::AlreadySynced
        );
        assert_eq!(
            decide_sync_action(400, Some(300), 300),
            SyncAction::PushLocal
        );
        assert_eq!(
            decide_sync_action(300, Some(450), 300),
            SyncAction::PullRemote
        );
        assert_eq!(
            decide_sync_action(400, Some(450), 300),
            SyncAction::Conflict
        );
        assert_eq!(decide_sync_action(400, None, 0), SyncAction::CreateRemote);
    }

    #[test]
    fn public_config_drops_client_secrets() {
        let config = parse_public_google_config(Some(
            r#"{"client_id":"abc","calendar_id":"work","client_secret":"super-secret"}"#,
        ));
        let stored = serialize_public_google_config(&config);
        assert!(stored.contains("abc"));
        assert!(!stored.contains("super-secret"));
    }

    struct FakeCalendar {
        events: Mutex<HashMap<String, RemoteEvent>>,
    }

    #[async_trait]
    impl CalendarApi for FakeCalendar {
        async fn get_event(
            &self,
            _calendar_id: &str,
            event_id: &str,
        ) -> Result<Option<RemoteEvent>> {
            Ok(self.events.lock().unwrap().get(event_id).cloned())
        }

        async fn insert_event(
            &self,
            _calendar_id: &str,
            draft: &EventDraft,
        ) -> Result<RemoteEvent> {
            let event = remote_from_draft("evt-created", draft, 500);
            self.events
                .lock()
                .unwrap()
                .insert(event.id.clone(), event.clone());
            Ok(event)
        }

        async fn update_event(
            &self,
            _calendar_id: &str,
            event_id: &str,
            draft: &EventDraft,
        ) -> Result<RemoteEvent> {
            let event = remote_from_draft(event_id, draft, 800);
            self.events
                .lock()
                .unwrap()
                .insert(event.id.clone(), event.clone());
            Ok(event)
        }
    }

    fn remote_from_draft(id: &str, draft: &EventDraft, updated: i64) -> RemoteEvent {
        let (start_unix, end_unix, all_day) = match (&draft.start, &draft.end) {
            (EventMoment::DateTime(start), EventMoment::DateTime(end)) => {
                (parse_rfc3339(start), parse_rfc3339(end), false)
            }
            (EventMoment::Date(start), EventMoment::Date(end)) => {
                (date_to_unix(start), date_to_unix(end), true)
            }
            _ => (None, None, false),
        };
        RemoteEvent {
            id: id.to_string(),
            summary: draft.summary.clone(),
            description: draft.description.clone(),
            start_unix,
            end_unix,
            all_day,
            updated_unix: updated,
            task_id: Some(draft.task_id.clone()),
        }
    }

    fn opted_in_task(
        conn: &Connection,
        updated_at: i64,
        external_id: &str,
        last_synced: i64,
    ) -> Task {
        let task = task_module::create(
            conn,
            CreateTaskInput {
                title: "Plan sprint".to_string(),
                description: Some("Notes".to_string()),
                status: None,
                priority: None,
                start_time: Some(1_760_000_000),
                end_time: Some(1_760_003_600),
                all_day: Some(false),
                recurrence: None,
                tags: None,
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE tasks SET updated_at = ?2 WHERE id = ?1",
            params![task.id, updated_at],
        )
        .unwrap();
        sync::set_task_sync(
            conn,
            crate::models::sync::SetTaskSyncInput {
                task_id: task.id.clone(),
                provider: "google".to_string(),
                enabled: true,
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET external_id = ?3, last_synced = ?4, sync_status = 'pending' WHERE task_id = ?1 AND provider = ?2",
            params![task.id, "google", external_id, last_synced],
        )
        .unwrap();
        task_module::get_by_id(conn, &task.id).unwrap().unwrap()
    }

    async fn run_sync(conn: Connection, api: &FakeCalendar) -> (Connection, SyncRunResult) {
        let snapshots = load_google_snapshots(&conn).unwrap();
        let outcomes = compute_outcomes(api, "primary", 0, snapshots)
            .await
            .unwrap();
        let result = apply_outcomes(&conn, &outcomes).unwrap();
        (conn, result)
    }

    #[tokio::test]
    async fn creates_a_google_event_for_a_placeholder_mapping() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in_task(&conn, 100, "", 0);
        let api = FakeCalendar {
            events: Mutex::new(HashMap::new()),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        assert_eq!(api.events.lock().unwrap().len(), 1);
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "synced");
        assert_eq!(state[0].external_id.as_deref(), Some("evt-created"));
    }

    #[tokio::test]
    async fn pushes_local_edits_and_pulls_remote_edits() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in_task(&conn, 400, "evt-1", 300);
        let remote = RemoteEvent {
            id: "evt-1".to_string(),
            summary: "Old".to_string(),
            description: None,
            start_unix: Some(1_760_000_000),
            end_unix: Some(1_760_003_600),
            all_day: false,
            updated_unix: 300,
            task_id: Some(task.id.clone()),
        };
        let api = FakeCalendar {
            events: Mutex::new(HashMap::from([(remote.id.clone(), remote)])),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "Plan sprint"
        );

        conn.execute(
            "UPDATE tasks SET updated_at = 300 WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET last_synced = 300, external_id = 'evt-1' WHERE task_id = ?1",
            params![task.id],
        )
        .unwrap();
        api.events.lock().unwrap().get_mut("evt-1").unwrap().summary = "From Google".to_string();
        api.events
            .lock()
            .unwrap()
            .get_mut("evt-1")
            .unwrap()
            .updated_unix = 900;
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "From Google");
    }

    #[tokio::test]
    async fn marks_conflict_without_overwriting_either_side() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in_task(&conn, 400, "evt-1", 300);
        let remote = RemoteEvent {
            id: "evt-1".to_string(),
            summary: "From Google".to_string(),
            description: None,
            start_unix: Some(1_760_000_000),
            end_unix: Some(1_760_003_600),
            all_day: false,
            updated_unix: 450,
            task_id: Some(task.id.clone()),
        };
        let api = FakeCalendar {
            events: Mutex::new(HashMap::from([(remote.id.clone(), remote)])),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "conflict");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "From Google"
        );
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Plan sprint");
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "conflict");
    }
}
