use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::models::sync::{
    MacosCalendarConnection, MacosCalendarInfo, SaveMacosCalendarSettingsInput, SyncAccount,
    SyncRunResult, UpsertSyncAccountInput,
};
use crate::modules::google_calendar::set_provider_feedback;

pub const MACOS_PROVIDER: &str = "macos";
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
    MacosCalendarConfig {
        calendar_id,
        calendar_name,
    }
}

pub fn serialize_config(config: &MacosCalendarConfig) -> String {
    json!({
        "calendar_id": config.calendar_id,
        "calendar_name": config.calendar_name,
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
    let enabled = input.enabled.unwrap_or(currently_enabled);
    let config = serialize_config(&MacosCalendarConfig {
        calendar_id,
        calendar_name,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sync::SetTaskSyncInput;
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
}
