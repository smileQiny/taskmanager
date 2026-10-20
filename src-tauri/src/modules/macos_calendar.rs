#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

use crate::models::sync::{
    MacosCalendarConnection, MacosCalendarInfo, SaveMacosCalendarSettingsInput, SetTaskSyncInput,
    SyncAccount, SyncRunResult, UpsertSyncAccountInput,
};
use crate::models::task::{CreateTaskInput, Task, UpdateTaskInput};
use crate::modules::google_calendar::{
    apply_provider_outcomes, compute_outcomes, load_provider_snapshots, plan_resolution,
    set_provider_feedback, task_to_event, CalendarApi, RemoteEvent, SyncOutcome, TaskSnapshot,
};
use crate::modules::task as task_module;

pub const MACOS_PROVIDER: &str = "macos";
pub const DIRECTION_BIDIRECTIONAL: &str = "bidirectional";
pub const DIRECTION_LOCAL_TO_REMOTE: &str = "local_to_remote";
pub const DIRECTION_REMOTE_TO_LOCAL: &str = "remote_to_local";
pub const IMPORT_PAST_SECONDS: i64 = 90 * 24 * 60 * 60;
pub const IMPORT_FUTURE_SECONDS: i64 = 180 * 24 * 60 * 60;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const TASK_MARKER: &str = "\n\ntaskmanager-task-id:";

/// EventKit authorization codes from the macOS 14 SDK.
/// `FullAccess` is 3 and is also the deprecated `Authorized` value.
/// `WriteOnly` is 4 and cannot read calendar changes, so it is not enough to sync both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum CalendarAccess {
    NotDetermined,
    Restricted,
    Denied,
    FullAccess,
    WriteOnly,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncGate {
    Disabled,
    Unsupported,
    NeedsAuth,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosCalendarConfig {
    pub calendar_id: String,
    pub calendar_name: String,
    pub sync_direction: String,
}

pub fn platform_supported() -> bool {
    cfg!(target_os = "macos")
}

pub fn live_access() -> CalendarAccess {
    if !platform_supported() {
        return CalendarAccess::Unsupported;
    }
    #[cfg(target_os = "macos")]
    {
        crate::modules::macos_eventkit::current_access()
    }
    #[cfg(not(target_os = "macos"))]
    {
        CalendarAccess::Unsupported
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn access_from_code(code: i64) -> CalendarAccess {
    match code {
        0 => CalendarAccess::NotDetermined,
        1 => CalendarAccess::Restricted,
        2 => CalendarAccess::Denied,
        3 => CalendarAccess::FullAccess,
        4 => CalendarAccess::WriteOnly,
        _ => CalendarAccess::NotDetermined,
    }
}

pub fn permission_name(access: CalendarAccess) -> &'static str {
    match access {
        CalendarAccess::NotDetermined => "not_determined",
        CalendarAccess::Restricted => "restricted",
        CalendarAccess::Denied => "denied",
        CalendarAccess::FullAccess => "full_access",
        CalendarAccess::WriteOnly => "write_only",
        CalendarAccess::Unsupported => "unsupported",
    }
}

pub fn access_message(access: CalendarAccess) -> Option<&'static str> {
    match access {
        CalendarAccess::FullAccess => None,
        CalendarAccess::Unsupported => {
            Some("macOS Calendar sync is only available on macOS.")
        }
        CalendarAccess::NotDetermined => {
            Some("Calendar access has not been granted. Allow full calendar access to sync.")
        }
        CalendarAccess::Restricted => Some("Calendar access is restricted on this Mac."),
        CalendarAccess::Denied => Some(
            "Calendar access was denied. Enable Task Manager in System Settings → Privacy & Security → Calendars.",
        ),
        CalendarAccess::WriteOnly => Some(
            "macOS Calendar sync needs full calendar access. Write-only access cannot read event changes.",
        ),
    }
}

pub fn sync_gate(enabled: bool, access: CalendarAccess) -> SyncGate {
    if !enabled {
        return SyncGate::Disabled;
    }
    match access {
        CalendarAccess::Unsupported => SyncGate::Unsupported,
        CalendarAccess::FullAccess => SyncGate::Ready,
        _ => SyncGate::NeedsAuth,
    }
}

pub fn unsupported_result() -> SyncRunResult {
    SyncRunResult {
        provider: MACOS_PROVIDER.to_string(),
        status: "unsupported".to_string(),
        message: "macOS Calendar sync is only available on macOS.".to_string(),
        affected_tasks: 0,
    }
}

pub fn parse_config(raw: Option<&str>) -> MacosCalendarConfig {
    let value = raw
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}));
    let calendar_id = value
        .get("calendar_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let calendar_name = value
        .get("calendar_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Calendar")
        .to_string();
    let sync_direction =
        normalize_sync_direction(value.get("sync_direction").and_then(Value::as_str)).to_string();
    MacosCalendarConfig {
        calendar_id,
        calendar_name,
        sync_direction,
    }
}

pub fn normalize_sync_direction(raw: Option<&str>) -> &'static str {
    match raw.map(str::trim) {
        Some(DIRECTION_LOCAL_TO_REMOTE) => DIRECTION_LOCAL_TO_REMOTE,
        Some(DIRECTION_REMOTE_TO_LOCAL) => DIRECTION_REMOTE_TO_LOCAL,
        _ => DIRECTION_BIDIRECTIONAL,
    }
}

pub fn import_window(now_unix: i64) -> (i64, i64) {
    (
        now_unix - IMPORT_PAST_SECONDS,
        now_unix + IMPORT_FUTURE_SECONDS,
    )
}

pub fn task_is_sync_eligible(task: &Task) -> bool {
    if task.start_time.is_none() || task.end_time.is_none() {
        return false;
    }
    match task.recurrence.as_deref().map(str::trim) {
        Some(rule) if !rule.is_empty() => false,
        _ => true,
    }
}

pub fn disambiguated_title(title: &str, marker: &str) -> String {
    let plain = format!(" ({marker})");
    if let Some(base) = title.strip_suffix(&plain) {
        return format!("{base} ({marker} 2)");
    }
    let numbered_prefix = format!(" ({marker} ");
    if let Some(index) = title.rfind(&numbered_prefix) {
        let rest = &title[index + numbered_prefix.len()..];
        if let Some(number) = rest.strip_suffix(')') {
            if !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit()) {
                let base = &title[..index];
                let next = number.parse::<u32>().unwrap_or(1) + 1;
                return format!("{base} ({marker} {next})");
            }
        }
    }
    format!("{title}{plain}")
}

pub fn serialize_config(config: &MacosCalendarConfig) -> String {
    json!({
        "calendar_id": config.calendar_id,
        "calendar_name": config.calendar_name,
        "sync_direction": config.sync_direction,
    })
    .to_string()
}

pub fn sanitize_public_config(config: Option<String>) -> Option<String> {
    config.map(|value| serialize_config(&parse_config(Some(&value))))
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn encode_notes(description: Option<&str>, task_id: &str) -> String {
    let body = description.unwrap_or("").trim_end();
    if body.is_empty() {
        format!("taskmanager-task-id:{task_id}")
    } else {
        format!("{body}{TASK_MARKER}{task_id}")
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn decode_notes(notes: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(notes) = notes.map(str::trim).filter(|text| !text.is_empty()) else {
        return (None, None);
    };
    if let Some((body, task_id)) = notes.rsplit_once("taskmanager-task-id:") {
        let task_id = task_id.trim();
        let description = body.trim();
        let description = if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        };
        let task_id = if task_id.is_empty() {
            None
        } else {
            Some(task_id.to_string())
        };
        return (description, task_id);
    }
    (Some(notes.to_string()), None)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn task_url(task_id: &str) -> String {
    format!("taskmanager://task/{task_id}")
}

pub fn connection_view(
    account: &SyncAccount,
    access: CalendarAccess,
    calendars: Vec<MacosCalendarInfo>,
) -> MacosCalendarConnection {
    let config = parse_config(account.config.as_deref());
    let gate = sync_gate(account.enabled, access);
    let status = match gate {
        SyncGate::Disabled => "not_configured",
        SyncGate::Unsupported => "unsupported",
        SyncGate::NeedsAuth => "needs_auth",
        SyncGate::Ready => {
            if account.status == "error" {
                "error"
            } else {
                "authenticated"
            }
        }
    };
    let last_error = if matches!(gate, SyncGate::NeedsAuth | SyncGate::Unsupported) {
        access_message(access).map(str::to_string)
    } else {
        account.last_error.clone()
    };
    MacosCalendarConnection {
        supported: platform_supported(),
        status: status.to_string(),
        connected: matches!(gate, SyncGate::Ready),
        permission: permission_name(access).to_string(),
        calendar_id: config.calendar_id,
        calendar_name: config.calendar_name,
        sync_direction: config.sync_direction,
        calendars,
        last_error,
    }
}

pub fn save_settings(
    conn: &Connection,
    input: &SaveMacosCalendarSettingsInput,
    currently_enabled: bool,
    access: CalendarAccess,
) -> Result<()> {
    let existing = account_config(conn)?;
    let parsed = parse_config(existing.as_deref());
    let calendar_id = input
        .calendar_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(parsed.calendar_id.as_str())
        .to_string();
    let calendar_name = input
        .calendar_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(parsed.calendar_name.as_str())
        .to_string();
    let sync_direction = match input.sync_direction.as_deref() {
        Some(value) => normalize_sync_direction(Some(value)).to_string(),
        None => parsed.sync_direction,
    };
    let enabled = input.enabled.unwrap_or(currently_enabled);
    let config = serialize_config(&MacosCalendarConfig {
        calendar_id,
        calendar_name,
        sync_direction,
    });
    crate::modules::sync::upsert_account(
        conn,
        UpsertSyncAccountInput {
            provider: MACOS_PROVIDER.to_string(),
            enabled,
            config: Some(config),
        },
    )?;
    let (status, message) = feedback_for(enabled, access);
    set_provider_feedback(conn, MACOS_PROVIDER, status, message)
}

pub fn blocked_result(
    conn: &Connection,
    account: &SyncAccount,
    access: CalendarAccess,
) -> Result<Option<SyncRunResult>> {
    match sync_gate(account.enabled, access) {
        SyncGate::Ready => Ok(None),
        SyncGate::Disabled => Ok(Some(SyncRunResult {
            provider: MACOS_PROVIDER.to_string(),
            status: "not_configured".to_string(),
            message: "Provider is disabled. Enable it in Settings before syncing.".to_string(),
            affected_tasks: 0,
        })),
        SyncGate::Unsupported => Ok(Some(unsupported_result())),
        SyncGate::NeedsAuth => {
            let message = access_message(access).unwrap_or("Calendar access is required.");
            let affected = mark_mappings(conn, "needs_auth")?;
            set_provider_feedback(conn, MACOS_PROVIDER, "needs_auth", Some(message))?;
            Ok(Some(SyncRunResult {
                provider: MACOS_PROVIDER.to_string(),
                status: "needs_auth".to_string(),
                message: message.to_string(),
                affected_tasks: affected,
            }))
        }
    }
}

fn feedback_for(enabled: bool, access: CalendarAccess) -> (&'static str, Option<&'static str>) {
    match sync_gate(enabled, access) {
        SyncGate::Disabled => ("not_configured", None),
        SyncGate::Unsupported => (
            "unsupported",
            Some("macOS Calendar sync is only available on macOS."),
        ),
        SyncGate::NeedsAuth => (
            "needs_auth",
            access_message(access).or(Some("Calendar access is required.")),
        ),
        SyncGate::Ready => ("authenticated", None),
    }
}

fn account_config(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT config FROM sync_accounts WHERE provider = 'macos'")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(row.get(0)?)
}

fn mark_mappings(conn: &Connection, status: &str) -> Result<i64> {
    let rows = conn.execute(
        "UPDATE sync_mappings SET sync_status = ?2 WHERE provider = ?1",
        params![MACOS_PROVIDER, status],
    )?;
    Ok(rows as i64)
}

pub fn ensure_timed_mappings(conn: &Connection) -> Result<i64> {
    let mut inserted = 0_i64;
    for task in task_module::get_all(conn)? {
        if !task_is_sync_eligible(&task) {
            continue;
        }
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sync_mappings WHERE task_id = ?1 AND provider = ?2",
            params![task.id, MACOS_PROVIDER],
            |row| row.get(0),
        )?;
        if exists > 0 {
            continue;
        }
        conn.execute(
            "INSERT INTO sync_mappings (id, task_id, provider, external_id, last_synced, sync_status)
             VALUES (?1, ?2, ?3, '', 0, 'pending')",
            params![Uuid::new_v4().to_string(), task.id, MACOS_PROVIDER],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

pub async fn run_macos_sync(
    conn: &Connection,
    api: &dyn CalendarApi,
    calendar_id: &str,
    direction: &str,
    now_unix: i64,
    offset_seconds: i32,
) -> Result<SyncRunResult> {
    let direction = normalize_sync_direction(Some(direction));
    let (window_start, window_end) = import_window(now_unix);
    let listed = api
        .list_events(calendar_id, window_start, window_end)
        .await?;
    ensure_timed_mappings(conn)?;
    adopt_listed_events(conn, &listed)?;

    let mut outcomes = Vec::new();
    if direction == DIRECTION_BIDIRECTIONAL {
        let eligible = load_provider_snapshots(conn, MACOS_PROVIDER)?
            .into_iter()
            .filter(|snapshot| task_is_sync_eligible(&snapshot.task))
            .collect();
        outcomes.extend(compute_outcomes(api, calendar_id, offset_seconds, eligible).await?);
    } else if direction == DIRECTION_LOCAL_TO_REMOTE {
        for snapshot in load_provider_snapshots(conn, MACOS_PROVIDER)? {
            if !task_is_sync_eligible(&snapshot.task) {
                continue;
            }
            outcomes.push(push_snapshot(api, calendar_id, offset_seconds, &snapshot).await?);
        }
    } else {
        for event in &listed {
            let Some(task_id) = task_id_for_external(conn, &event.id)? else {
                continue;
            };
            let Some(task) = task_module::get_by_id(conn, &task_id)? else {
                continue;
            };
            if !task_is_sync_eligible(&task) {
                continue;
            }
            outcomes.push(SyncOutcome::Pull {
                task,
                remote: event.clone(),
            });
        }
    }

    if direction != DIRECTION_LOCAL_TO_REMOTE {
        let mapped = macos_external_ids(conn)?;
        for event in &listed {
            if should_import(conn, event, &mapped)? {
                outcomes.push(stage_import(conn, event)?);
            }
        }
    }

    apply_provider_outcomes(conn, MACOS_PROVIDER, "macOS Calendar", &outcomes)
}

pub async fn resolve_macos_conflict(
    conn: &Connection,
    api: &dyn CalendarApi,
    calendar_id: &str,
    task_id: &str,
    strategy: &str,
    offset_seconds: i32,
) -> Result<SyncRunResult> {
    match strategy {
        "local" | "remote" => {
            let snapshot = macos_snapshot(conn, task_id)?;
            if strategy == "remote" {
                let _remote = require_remote(api, calendar_id, &snapshot.external_id).await?;
            }
            let outcome =
                plan_resolution(api, calendar_id, offset_seconds, snapshot, strategy).await?;
            apply_provider_outcomes(conn, MACOS_PROVIDER, "macOS Calendar", &[outcome])
        }
        "keep_both_rename" => {
            keep_both_rename(conn, api, calendar_id, task_id, offset_seconds).await
        }
        "keep_both_newest" => {
            keep_both_newest(conn, api, calendar_id, task_id, offset_seconds).await
        }
        _ => {
            bail!("conflict strategy must be local, remote, keep_both_rename, or keep_both_newest")
        }
    }
}

async fn keep_both_rename(
    conn: &Connection,
    api: &dyn CalendarApi,
    calendar_id: &str,
    task_id: &str,
    offset_seconds: i32,
) -> Result<SyncRunResult> {
    let snapshot = macos_snapshot(conn, task_id)?;
    let remote = require_remote(api, calendar_id, &snapshot.external_id).await?;
    let renamed = disambiguated_title(&snapshot.task.title, "本地");
    task_module::update(
        conn,
        UpdateTaskInput {
            id: task_id.to_string(),
            title: Some(renamed),
            description: None,
            status: None,
            priority: None,
            start_time: None,
            end_time: None,
            all_day: None,
            recurrence: None,
            tags: None,
        },
    )?;
    let renamed_task = task_module::get_by_id(conn, task_id)?
        .ok_or_else(|| anyhow!("task disappeared during macOS conflict resolution"))?;
    let imported = create_task_from_event(conn, &remote)?;
    enroll_macos(conn, &imported.id)?;
    let saved = api
        .insert_event(calendar_id, &task_to_event(&renamed_task, offset_seconds))
        .await?;
    apply_provider_outcomes(
        conn,
        MACOS_PROVIDER,
        "macOS Calendar",
        &[
            SyncOutcome::Clean {
                task_id: renamed_task.id,
                external_id: saved.id,
                local_updated: renamed_task.updated_at,
                remote_updated: saved.updated_unix,
            },
            SyncOutcome::Clean {
                task_id: imported.id,
                external_id: remote.id,
                local_updated: imported.updated_at,
                remote_updated: remote.updated_unix,
            },
        ],
    )
}

async fn keep_both_newest(
    conn: &Connection,
    api: &dyn CalendarApi,
    calendar_id: &str,
    task_id: &str,
    offset_seconds: i32,
) -> Result<SyncRunResult> {
    let snapshot = macos_snapshot(conn, task_id)?;
    let remote = require_remote(api, calendar_id, &snapshot.external_id).await?;
    let local_is_newer = snapshot.task.updated_at >= remote.updated_unix;
    if local_is_newer {
        let archived_title = disambiguated_title(&event_title(&remote.summary), "较早");
        let archived = task_module::create(
            conn,
            CreateTaskInput {
                title: archived_title,
                description: remote.description.clone(),
                status: Some("todo".to_string()),
                priority: Some("medium".to_string()),
                start_time: remote.start_unix,
                end_time: remote.end_unix,
                all_day: Some(remote.all_day),
                recurrence: None,
                tags: None,
            },
        )?;
        enroll_macos(conn, &archived.id)?;
        let archived_event = api
            .insert_event(calendar_id, &task_to_event(&archived, offset_seconds))
            .await?;
        let saved = api
            .update_event(
                calendar_id,
                &snapshot.external_id,
                &task_to_event(&snapshot.task, offset_seconds),
            )
            .await?;
        apply_provider_outcomes(
            conn,
            MACOS_PROVIDER,
            "macOS Calendar",
            &[
                SyncOutcome::Clean {
                    task_id: snapshot.task.id,
                    external_id: saved.id,
                    local_updated: snapshot.task.updated_at,
                    remote_updated: saved.updated_unix,
                },
                SyncOutcome::Clean {
                    task_id: archived.id,
                    external_id: archived_event.id,
                    local_updated: archived.updated_at,
                    remote_updated: archived_event.updated_unix,
                },
            ],
        )
    } else {
        let archived_title = disambiguated_title(&snapshot.task.title, "较早");
        let archived = task_module::create(
            conn,
            CreateTaskInput {
                title: archived_title,
                description: snapshot.task.description.clone(),
                status: Some(snapshot.task.status.clone()),
                priority: Some(snapshot.task.priority.clone()),
                start_time: snapshot.task.start_time,
                end_time: snapshot.task.end_time,
                all_day: Some(snapshot.task.all_day),
                recurrence: None,
                tags: snapshot.task.tags.clone(),
            },
        )?;
        enroll_macos(conn, &archived.id)?;
        let archived_event = api
            .insert_event(calendar_id, &task_to_event(&archived, offset_seconds))
            .await?;
        apply_provider_outcomes(
            conn,
            MACOS_PROVIDER,
            "macOS Calendar",
            &[
                SyncOutcome::Pull {
                    task: snapshot.task,
                    remote: remote.clone(),
                },
                SyncOutcome::Clean {
                    task_id: archived.id,
                    external_id: archived_event.id,
                    local_updated: archived.updated_at,
                    remote_updated: archived_event.updated_unix,
                },
            ],
        )
    }
}

async fn push_snapshot(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshot: &TaskSnapshot,
) -> Result<SyncOutcome> {
    match push_snapshot_inner(api, calendar_id, offset_seconds, snapshot).await {
        Ok(outcome) => Ok(outcome),
        Err(err) if is_macos_auth_failure(&err) => Err(err),
        Err(err) => Ok(SyncOutcome::Failed {
            task_id: snapshot.task.id.clone(),
            message: err.to_string(),
        }),
    }
}

async fn push_snapshot_inner(
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

async fn require_remote(
    api: &dyn CalendarApi,
    calendar_id: &str,
    external_id: &str,
) -> Result<RemoteEvent> {
    if external_id.is_empty() {
        bail!("macOS Calendar event no longer exists");
    }
    api.get_event(calendar_id, external_id)
        .await?
        .ok_or_else(|| anyhow!("macOS Calendar event no longer exists"))
}

fn macos_snapshot(conn: &Connection, task_id: &str) -> Result<TaskSnapshot> {
    load_provider_snapshots(conn, MACOS_PROVIDER)?
        .into_iter()
        .find(|snapshot| snapshot.task.id == task_id)
        .ok_or_else(|| anyhow!("task is not opted in to macOS Calendar"))
}

fn adopt_listed_events(conn: &Connection, events: &[RemoteEvent]) -> Result<()> {
    for event in events {
        let Some(task_id) = event
            .task_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        if task_module::get_by_id(conn, task_id)?.is_none() {
            continue;
        }
        let external_id: Option<String> = conn
            .query_row(
                "SELECT external_id FROM sync_mappings WHERE task_id = ?1 AND provider = ?2",
                params![task_id, MACOS_PROVIDER],
                |row| row.get(0),
            )
            .optional()?;
        if external_id.as_deref() == Some("") {
            conn.execute(
                "UPDATE sync_mappings SET external_id = ?3 WHERE task_id = ?1 AND provider = ?2",
                params![task_id, MACOS_PROVIDER, event.id],
            )?;
        }
    }
    Ok(())
}

fn task_id_for_external(conn: &Connection, external_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT task_id FROM sync_mappings WHERE provider = ?1 AND external_id = ?2",
            params![MACOS_PROVIDER, external_id],
            |row| row.get(0),
        )
        .optional()?)
}

fn macos_external_ids(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(
        "SELECT external_id FROM sync_mappings WHERE provider = ?1 AND external_id != ''",
    )?;
    let rows = stmt.query_map(params![MACOS_PROVIDER], |row| row.get::<_, String>(0))?;
    let mut ids = HashSet::new();
    for id in rows {
        ids.insert(id?);
    }
    Ok(ids)
}

fn should_import(
    conn: &Connection,
    event: &RemoteEvent,
    mapped_ids: &HashSet<String>,
) -> Result<bool> {
    if mapped_ids.contains(&event.id) {
        return Ok(false);
    }
    if event.start_unix.is_none() || event.end_unix.is_none() {
        return Ok(false);
    }
    if let Some(task_id) = event
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        if task_module::get_by_id(conn, task_id)?.is_some() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn stage_import(conn: &Connection, event: &RemoteEvent) -> Result<SyncOutcome> {
    let task = create_task_from_event(conn, event)?;
    enroll_macos(conn, &task.id)?;
    Ok(SyncOutcome::Clean {
        task_id: task.id,
        external_id: event.id.clone(),
        local_updated: task.updated_at,
        remote_updated: event.updated_unix,
    })
}

fn create_task_from_event(conn: &Connection, event: &RemoteEvent) -> Result<Task> {
    task_module::create(
        conn,
        CreateTaskInput {
            title: event_title(&event.summary),
            description: event.description.clone(),
            status: Some("todo".to_string()),
            priority: Some("medium".to_string()),
            start_time: event.start_unix,
            end_time: event.end_unix,
            all_day: Some(event.all_day),
            recurrence: None,
            tags: None,
        },
    )
}

fn enroll_macos(conn: &Connection, task_id: &str) -> Result<()> {
    crate::modules::sync::set_task_sync(
        conn,
        SetTaskSyncInput {
            task_id: task_id.to_string(),
            provider: MACOS_PROVIDER.to_string(),
            enabled: true,
        },
    )?;
    Ok(())
}

fn event_title(summary: &str) -> String {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        "未命名日程".to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_macos_auth_failure(err: &anyhow::Error) -> bool {
    let text = err.to_string();
    text.contains("status 401") || text.contains("status 403") || text.contains("invalid_grant")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sync::{SaveMacosCalendarSettingsInput, SetTaskSyncInput};
    use crate::models::task::CreateTaskInput;
    use crate::modules::db;
    use crate::modules::google_calendar::{
        apply_provider_outcomes, compute_outcomes, CalendarApi, EventDraft, EventMoment,
        RemoteEvent,
    };
    use crate::modules::sync;
    use crate::modules::task as task_module;
    use async_trait::async_trait;
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[test]
    fn maps_eventkit_authorization_codes_without_treating_write_only_as_syncable() {
        assert_eq!(access_from_code(0), CalendarAccess::NotDetermined);
        assert_eq!(access_from_code(1), CalendarAccess::Restricted);
        assert_eq!(access_from_code(2), CalendarAccess::Denied);
        assert_eq!(access_from_code(3), CalendarAccess::FullAccess);
        assert_eq!(access_from_code(4), CalendarAccess::WriteOnly);
        assert_eq!(sync_gate(true, CalendarAccess::FullAccess), SyncGate::Ready);
        assert_eq!(sync_gate(true, CalendarAccess::Denied), SyncGate::NeedsAuth);
        assert_eq!(
            sync_gate(true, CalendarAccess::WriteOnly),
            SyncGate::NeedsAuth
        );
        assert_eq!(
            sync_gate(true, CalendarAccess::Unsupported),
            SyncGate::Unsupported
        );
        assert_eq!(
            sync_gate(false, CalendarAccess::FullAccess),
            SyncGate::Disabled
        );
        assert!(access_message(CalendarAccess::Denied)
            .unwrap()
            .contains("Privacy"));
        assert!(access_message(CalendarAccess::Unsupported)
            .unwrap()
            .contains("only available on macOS"));
    }

    #[test]
    fn public_config_keeps_calendar_choice_and_drops_secrets() {
        let config = parse_config(Some(
            r#"{"calendar_id":"cal-1","calendar_name":"Home","client_secret":"super-secret"}"#,
        ));
        let stored = serialize_config(&config);
        assert!(stored.contains("cal-1"));
        assert!(stored.contains("Home"));
        assert!(!stored.contains("super-secret"));
    }

    #[test]
    fn notes_round_trip_keeps_the_task_description_and_id() {
        let encoded = encode_notes(Some("Bring the notes"), "task-1");
        let (description, task_id) = decode_notes(Some(&encoded));
        assert_eq!(description.as_deref(), Some("Bring the notes"));
        assert_eq!(task_id.as_deref(), Some("task-1"));
        let (empty_description, id_only) = decode_notes(Some("taskmanager-task-id:task-2"));
        assert_eq!(empty_description, None);
        assert_eq!(id_only.as_deref(), Some("task-2"));
    }

    #[test]
    fn denied_access_marks_mappings_needs_auth_and_does_not_claim_success() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 100, "", 0);
        sync::upsert_account(
            &conn,
            UpsertSyncAccountInput {
                provider: MACOS_PROVIDER.to_string(),
                enabled: true,
                config: Some(r#"{"calendar_name":"Calendar"}"#.to_string()),
            },
        )
        .unwrap();
        let account = macos_account(&conn);
        let blocked = blocked_result(&conn, &account, CalendarAccess::Denied)
            .unwrap()
            .unwrap();
        assert_eq!(blocked.status, "needs_auth");
        assert_ne!(blocked.status, "synced");
        assert!(blocked.message.contains("denied"));
        let states = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(states[0].sync_status, "needs_auth");
    }

    #[test]
    fn unsupported_platform_does_not_rewrite_mappings_as_authorized() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 100, "", 0);
        sync::upsert_account(
            &conn,
            UpsertSyncAccountInput {
                provider: MACOS_PROVIDER.to_string(),
                enabled: true,
                config: None,
            },
        )
        .unwrap();
        let account = macos_account(&conn);
        let blocked = blocked_result(&conn, &account, CalendarAccess::Unsupported)
            .unwrap()
            .unwrap();
        assert_eq!(blocked.status, "unsupported");
        assert!(blocked.message.contains("only available on macOS"));
        assert_eq!(blocked.affected_tasks, 0);
        let states = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(states[0].sync_status, "pending");
        let view = connection_view(&account, CalendarAccess::Unsupported, vec![]);
        assert!(!view.supported || platform_supported());
        assert!(!view.connected);
        if !platform_supported() {
            assert_eq!(view.status, "unsupported");
            assert_eq!(view.permission, "unsupported");
        }
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
            (EventMoment::DateTime(start), EventMoment::DateTime(end)) => (
                chrono::DateTime::parse_from_rfc3339(start)
                    .ok()
                    .map(|date| date.timestamp()),
                chrono::DateTime::parse_from_rfc3339(end)
                    .ok()
                    .map(|date| date.timestamp()),
                false,
            ),
            _ => (None, None, true),
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

    fn opted_in(
        conn: &Connection,
        updated_at: i64,
        external_id: &str,
        last_synced: i64,
    ) -> crate::models::task::Task {
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
            SetTaskSyncInput {
                task_id: task.id.clone(),
                provider: MACOS_PROVIDER.to_string(),
                enabled: true,
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET external_id = ?3, last_synced = ?4 WHERE task_id = ?1 AND provider = ?2",
            params![task.id, MACOS_PROVIDER, external_id, last_synced],
        )
        .unwrap();
        task_module::get_by_id(conn, &task.id).unwrap().unwrap()
    }

    fn macos_account(conn: &Connection) -> SyncAccount {
        sync::list_accounts(conn)
            .unwrap()
            .into_iter()
            .find(|account| account.provider == MACOS_PROVIDER)
            .unwrap()
    }

    async fn run_sync(conn: Connection, api: &FakeCalendar) -> (Connection, SyncRunResult) {
        let snapshots =
            crate::modules::google_calendar::load_provider_snapshots(&conn, MACOS_PROVIDER)
                .unwrap();
        let outcomes = compute_outcomes(api, "cal-1", 0, snapshots).await.unwrap();
        let result =
            apply_provider_outcomes(&conn, MACOS_PROVIDER, "macOS Calendar", &outcomes).unwrap();
        (conn, result)
    }

    #[tokio::test]
    async fn creates_updates_and_pulls_mapped_macos_events() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 100, "", 0);
        let api = FakeCalendar {
            events: Mutex::new(HashMap::new()),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        assert_eq!(result.provider, MACOS_PROVIDER);
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "synced");
        assert_eq!(state[0].external_id.as_deref(), Some("evt-created"));

        conn.execute(
            "UPDATE tasks SET title = 'Local edit', updated_at = 700 WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET last_synced = 500, external_id = 'evt-created' WHERE task_id = ?1 AND provider = 'macos'",
            params![task.id],
        )
        .unwrap();
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        assert_eq!(
            api.events
                .lock()
                .unwrap()
                .get("evt-created")
                .unwrap()
                .summary,
            "Local edit"
        );

        conn.execute(
            "UPDATE tasks SET updated_at = 800 WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET last_synced = 800 WHERE task_id = ?1 AND provider = 'macos'",
            params![task.id],
        )
        .unwrap();
        api.events
            .lock()
            .unwrap()
            .get_mut("evt-created")
            .unwrap()
            .summary = "From Calendar".into();
        api.events
            .lock()
            .unwrap()
            .get_mut("evt-created")
            .unwrap()
            .updated_unix = 900;
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "From Calendar");
    }

    #[tokio::test]
    async fn bilateral_edits_become_a_conflict_without_overwriting_either_side() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        let remote = RemoteEvent {
            id: "evt-1".to_string(),
            summary: "From Calendar".to_string(),
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
        assert_ne!(result.status, "synced");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "From Calendar"
        );
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Plan sprint");
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "conflict");
    }

    #[test]
    fn sync_direction_defaults_round_trips_and_save_keeps_omitted_value() {
        assert_eq!(normalize_sync_direction(None), "bidirectional");
        assert_eq!(normalize_sync_direction(Some("nope")), "bidirectional");
        assert_eq!(
            normalize_sync_direction(Some("local_to_remote")),
            "local_to_remote"
        );
        let parsed = parse_config(Some(
            r#"{"calendar_id":"cal-1","calendar_name":"Home","sync_direction":"remote_to_local","client_secret":"super-secret"}"#,
        ));
        assert_eq!(parsed.sync_direction, "remote_to_local");
        let stored = serialize_config(&parsed);
        assert!(stored.contains("remote_to_local"));
        assert!(!stored.contains("super-secret"));

        let conn = db::init_in_memory().unwrap();
        save_settings(
            &conn,
            &SaveMacosCalendarSettingsInput {
                calendar_id: Some("cal-1".to_string()),
                calendar_name: Some("Home".to_string()),
                enabled: Some(true),
                sync_direction: Some("local_to_remote".to_string()),
            },
            false,
            CalendarAccess::FullAccess,
        )
        .unwrap();
        save_settings(
            &conn,
            &SaveMacosCalendarSettingsInput {
                calendar_id: None,
                calendar_name: Some("Work".to_string()),
                enabled: None,
                sync_direction: None,
            },
            true,
            CalendarAccess::FullAccess,
        )
        .unwrap();
        let raw: String = conn
            .query_row(
                "SELECT config FROM sync_accounts WHERE provider = 'macos'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let kept = parse_config(Some(&raw));
        assert_eq!(kept.sync_direction, "local_to_remote");
        assert_eq!(kept.calendar_name, "Work");
        assert_eq!(kept.calendar_id, "cal-1");
        let (start, end) = import_window(1_700_000_000);
        assert_eq!(start, 1_700_000_000 - 90 * 24 * 60 * 60);
        assert_eq!(end, 1_700_000_000 + 180 * 24 * 60 * 60);
    }

    #[test]
    fn disambiguated_title_adds_and_increments_a_suffix() {
        assert_eq!(disambiguated_title("Meeting", "本地"), "Meeting (本地)");
        assert_eq!(
            disambiguated_title("Meeting (本地)", "本地"),
            "Meeting (本地 2)"
        );
        assert_eq!(
            disambiguated_title("Meeting (本地 2)", "本地"),
            "Meeting (本地 3)"
        );
        assert_eq!(disambiguated_title("Meeting", "较早"), "Meeting (较早)");
    }

    #[test]
    fn ensure_timed_mappings_enrolls_done_tasks_and_skips_untimed_or_recurring() {
        let conn = db::init_in_memory().unwrap();
        let done = timed_task(&conn, "Done", Some("done".to_string()), None);
        let open = timed_task(&conn, "Open", None, None);
        let untimed = task_module::create(
            &conn,
            CreateTaskInput {
                title: "No end".to_string(),
                description: None,
                status: None,
                priority: None,
                start_time: Some(1_760_000_000),
                end_time: None,
                all_day: None,
                recurrence: None,
                tags: None,
            },
        )
        .unwrap();
        let recurring = timed_task(&conn, "Weekly", None, Some("FREQ=DAILY".to_string()));
        sync::set_task_sync(
            &conn,
            SetTaskSyncInput {
                task_id: open.id.clone(),
                provider: "feishu".to_string(),
                enabled: true,
            },
        )
        .unwrap();
        assert_eq!(ensure_timed_mappings(&conn).unwrap(), 2);
        assert_eq!(ensure_timed_mappings(&conn).unwrap(), 0);
        assert_eq!(mapping_count(&conn, &done.id, "macos"), 1);
        assert_eq!(mapping_count(&conn, &open.id, "macos"), 1);
        assert_eq!(mapping_count(&conn, &untimed.id, "macos"), 0);
        assert_eq!(mapping_count(&conn, &recurring.id, "macos"), 0);
        assert_eq!(mapping_count(&conn, &open.id, "feishu"), 1);
        assert!(!task_is_sync_eligible(&untimed));
        assert!(!task_is_sync_eligible(&recurring));
        assert!(task_is_sync_eligible(&done));
    }

    struct ListingCalendar {
        events: Mutex<HashMap<String, RemoteEvent>>,
        listed: Vec<RemoteEvent>,
        list_calls: Mutex<Vec<(String, i64, i64)>>,
        inserts: Mutex<u32>,
        updates: Mutex<u32>,
        next_id: Mutex<u32>,
    }

    #[async_trait]
    impl CalendarApi for ListingCalendar {
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
            *self.inserts.lock().unwrap() += 1;
            let mut next = self.next_id.lock().unwrap();
            *next += 1;
            let id = format!("evt-new-{}", *next);
            let event = remote_from_draft(&id, draft, 500);
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
            *self.updates.lock().unwrap() += 1;
            let event = remote_from_draft(event_id, draft, 800);
            self.events
                .lock()
                .unwrap()
                .insert(event.id.clone(), event.clone());
            Ok(event)
        }

        async fn list_events(
            &self,
            calendar_id: &str,
            start_unix: i64,
            end_unix: i64,
        ) -> Result<Vec<RemoteEvent>> {
            self.list_calls
                .lock()
                .unwrap()
                .push((calendar_id.to_string(), start_unix, end_unix));
            Ok(self.listed.clone())
        }
    }

    fn listing(listed: Vec<RemoteEvent>) -> ListingCalendar {
        let mut events = HashMap::new();
        for event in &listed {
            events.insert(event.id.clone(), event.clone());
        }
        ListingCalendar {
            events: Mutex::new(events),
            listed,
            list_calls: Mutex::new(Vec::new()),
            inserts: Mutex::new(0),
            updates: Mutex::new(0),
            next_id: Mutex::new(0),
        }
    }

    fn bare_event(id: &str, summary: &str, task_id: Option<String>, updated: i64) -> RemoteEvent {
        RemoteEvent {
            id: id.to_string(),
            summary: summary.to_string(),
            description: None,
            start_unix: Some(1_760_000_000),
            end_unix: Some(1_760_003_600),
            all_day: false,
            updated_unix: updated,
            task_id,
        }
    }

    fn timed_task(
        conn: &Connection,
        title: &str,
        status: Option<String>,
        recurrence: Option<String>,
    ) -> crate::models::task::Task {
        task_module::create(
            conn,
            CreateTaskInput {
                title: title.to_string(),
                description: Some("Notes".to_string()),
                status,
                priority: None,
                start_time: Some(1_760_000_000),
                end_time: Some(1_760_003_600),
                all_day: Some(false),
                recurrence,
                tags: None,
            },
        )
        .unwrap()
    }

    fn mapping_count(conn: &Connection, task_id: &str, provider: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM sync_mappings WHERE task_id = ?1 AND provider = ?2",
            params![task_id, provider],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn external_id(conn: &Connection, task_id: &str) -> String {
        conn.query_row(
            "SELECT external_id FROM sync_mappings WHERE task_id = ?1 AND provider = 'macos'",
            params![task_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn titles(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT title FROM tasks ORDER BY title")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    }

    async fn run_direction(
        conn: Connection,
        api: &ListingCalendar,
        direction: &str,
    ) -> (Connection, SyncRunResult) {
        let result = run_macos_sync(&conn, api, "cal-work", direction, 1_700_000_000, 0)
            .await
            .unwrap();
        (conn, result)
    }

    #[tokio::test]
    async fn bidirectional_pushes_timed_tasks_and_imports_in_window_events() {
        let conn = db::init_in_memory().unwrap();
        let local = timed_task(&conn, "Local only", Some("done".to_string()), None);
        let conn = {
            let untimed = task_module::create(
                &conn,
                CreateTaskInput {
                    title: "Missing end".to_string(),
                    description: None,
                    status: None,
                    priority: None,
                    start_time: Some(1_760_000_000),
                    end_time: Some(1_760_003_600),
                    all_day: None,
                    recurrence: None,
                    tags: None,
                },
            )
            .unwrap();
            conn.execute(
                "UPDATE tasks SET end_time = NULL WHERE id = ?1",
                params![untimed.id],
            )
            .unwrap();
            let _ = untimed;
            conn
        };
        let api = listing(vec![bare_event("evt-bare", "From calendar", None, 10)]);
        let (conn, result) = run_direction(conn, &api, "bidirectional").await;
        assert_eq!(result.status, "synced");
        assert!(result.affected_tasks >= 2);
        assert_eq!(*api.inserts.lock().unwrap(), 1);
        assert_eq!(external_id(&conn, &local.id), "evt-new-1");
        assert!(titles(&conn).contains(&"From calendar".to_string()));
        assert_eq!(mapping_count(&conn, &local.id, "macos"), 1);
        let missing = conn
            .query_row(
                "SELECT id FROM tasks WHERE title = 'Missing end'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        assert_eq!(mapping_count(&conn, &missing, "macos"), 0);
        let call = api.list_calls.lock().unwrap()[0].clone();
        assert_eq!(call.0, "cal-work");
        assert_eq!(call.1, 1_700_000_000 - 90 * 24 * 60 * 60);
        assert_eq!(call.2, 1_700_000_000 + 180 * 24 * 60 * 60);
    }

    #[tokio::test]
    async fn local_to_remote_does_not_import_bare_events() {
        let conn = db::init_in_memory().unwrap();
        let local = timed_task(&conn, "Push me", None, None);
        let api = listing(vec![bare_event("evt-bare", "Bare event", None, 10)]);
        let (conn, result) = run_direction(conn, &api, "local_to_remote").await;
        assert_eq!(result.status, "synced");
        assert_eq!(*api.inserts.lock().unwrap(), 1);
        assert_eq!(external_id(&conn, &local.id), "evt-new-1");
        assert_eq!(titles(&conn), vec!["Push me".to_string()]);
    }

    #[tokio::test]
    async fn remote_to_local_does_not_push_local_only_tasks() {
        let conn = db::init_in_memory().unwrap();
        let local = timed_task(&conn, "Stay local", None, None);
        let api = listing(vec![bare_event("evt-bare", "Imported", None, 10)]);
        let (conn, result) = run_direction(conn, &api, "remote_to_local").await;
        assert_eq!(result.status, "synced");
        assert_eq!(*api.inserts.lock().unwrap(), 0);
        assert_eq!(*api.updates.lock().unwrap(), 0);
        assert_eq!(external_id(&conn, &local.id), "");
        assert!(titles(&conn).contains(&"Imported".to_string()));
        let imported = conn
            .query_row("SELECT id FROM tasks WHERE title = 'Imported'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap();
        assert_eq!(external_id(&conn, &imported), "evt-bare");
    }

    #[tokio::test]
    async fn adoption_links_an_event_by_task_id_instead_of_inserting_another() {
        let conn = db::init_in_memory().unwrap();
        let task = timed_task(&conn, "Plan sprint", None, None);
        conn.execute(
            "UPDATE tasks SET updated_at = 0 WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        let event = bare_event("evt-old", "Plan sprint", Some(task.id.clone()), 0);
        let api = listing(vec![event]);
        let (conn, result) = run_direction(conn, &api, "bidirectional").await;
        assert_eq!(result.status, "synced");
        assert_eq!(*api.inserts.lock().unwrap(), 0);
        assert_eq!(external_id(&conn, &task.id), "evt-old");
        assert_eq!(titles(&conn), vec!["Plan sprint".to_string()]);
    }

    #[tokio::test]
    async fn bidirectional_bilateral_edit_stays_a_conflict() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        let remote = bare_event("evt-1", "From Calendar", Some(task.id.clone()), 450);
        let api = listing(vec![remote]);
        let (conn, result) = run_direction(conn, &api, "bidirectional").await;
        assert_eq!(result.status, "conflict");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "From Calendar"
        );
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Plan sprint");
    }

    #[tokio::test]
    async fn conflict_resolutions_keep_local_calendar_or_both_copies() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        let remote = bare_event("evt-1", "From Calendar", Some(task.id.clone()), 450);
        let api = listing(vec![remote]);
        let err = resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "keep_both_rename", 0)
            .await
            .err();
        assert!(err.is_none());
        let renamed = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(renamed.title, "Plan sprint (本地)");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "From Calendar"
        );
        assert!(titles(&conn).contains(&"From Calendar".to_string()));
        assert!(titles(&conn).contains(&"Plan sprint (本地)".to_string()));
        let new_event = api
            .events
            .lock()
            .unwrap()
            .values()
            .find(|event| event.summary == "Plan sprint (本地)")
            .cloned()
            .unwrap();
        assert_ne!(new_event.id, "evt-1");
        assert_eq!(external_id(&conn, &task.id), new_event.id);
        let imported = conn
            .query_row(
                "SELECT id FROM tasks WHERE title = 'From Calendar'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        assert_eq!(external_id(&conn, &imported), "evt-1");

        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 500, "evt-1", 300);
        conn.execute(
            "UPDATE tasks SET title = 'Local title' WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        let remote = bare_event("evt-1", "Calendar title", Some(task.id.clone()), 400);
        let api = listing(vec![remote]);
        resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "keep_both_newest", 0)
            .await
            .unwrap();
        let primary = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(primary.title, "Local title");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "Local title"
        );
        assert!(titles(&conn).contains(&"Calendar title (较早)".to_string()));
        assert_eq!(external_id(&conn, &task.id), "evt-1");

        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        conn.execute(
            "UPDATE tasks SET title = 'Local title' WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        let remote = bare_event("evt-1", "Calendar title", Some(task.id.clone()), 500);
        let api = listing(vec![remote]);
        resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "keep_both_newest", 0)
            .await
            .unwrap();
        let primary = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(primary.title, "Calendar title");
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "Calendar title"
        );
        assert!(titles(&conn).contains(&"Local title (较早)".to_string()));

        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        conn.execute(
            "UPDATE tasks SET title = 'Same time' WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        let remote = bare_event("evt-1", "Older calendar", Some(task.id.clone()), 400);
        let api = listing(vec![remote]);
        resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "keep_both_newest", 0)
            .await
            .unwrap();
        assert_eq!(
            task_module::get_by_id(&conn, &task.id)
                .unwrap()
                .unwrap()
                .title,
            "Same time"
        );
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "Same time"
        );
        assert!(titles(&conn).contains(&"Older calendar (较早)".to_string()));

        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        let remote = bare_event("evt-1", "From Calendar", Some(task.id.clone()), 450);
        let api = listing(vec![remote]);
        resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "local", 0)
            .await
            .unwrap();
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "Plan sprint"
        );
        assert_eq!(titles(&conn), vec!["Plan sprint".to_string()]);

        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300);
        let remote = bare_event("evt-1", "From Calendar", Some(task.id.clone()), 450);
        let api = listing(vec![remote]);
        resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "remote", 0)
            .await
            .unwrap();
        assert_eq!(
            task_module::get_by_id(&conn, &task.id)
                .unwrap()
                .unwrap()
                .title,
            "From Calendar"
        );

        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-missing", 300);
        let api = listing(vec![]);
        let err = resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "keep_both_rename", 0)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no longer exists"));
        assert_eq!(
            task_module::get_by_id(&conn, &task.id)
                .unwrap()
                .unwrap()
                .title,
            "Plan sprint"
        );
        let err = resolve_macos_conflict(&conn, &api, "cal-work", &task.id, "sideways", 0)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("keep_both_newest"));
    }
}
