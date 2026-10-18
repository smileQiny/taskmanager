use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::models::sync::{
    FeishuConnection, SaveFeishuSettingsInput, SyncAccount, SyncRunResult, UpsertSyncAccountInput,
};
use crate::modules::feishu_oauth::{normalize_domain, REDIRECT_URI};
use crate::modules::google_calendar::{
    apply_provider_outcomes, decide_sync_action, load_provider_snapshots, set_provider_feedback,
    task_to_event, CalendarApi, EventDraft, EventMoment, RemoteEvent, SyncAction, SyncOutcome,
    TaskSnapshot,
};
use crate::modules::token_store::TokenStore;

pub const FEISHU_PROVIDER: &str = "feishu";
const SECRET_ID: &str = "feishu.app_secret";
const TOKEN_ID: &str = "feishu.tokens";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeishuPublicConfig {
    pub app_id: String,
    pub calendar_id: String,
    pub domain: String,
}

#[derive(Debug, Clone)]
pub struct FeishuPlan {
    pub outcome: SyncOutcome,
    pub remote_hash: Option<String>,
}

pub struct HttpCalendar {
    access_token: String,
    api_base: String,
}

impl HttpCalendar {
    pub fn new(access_token: impl Into<String>, api_base: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            api_base: api_base.into().trim_end_matches('/').to_string(),
        }
    }

    pub async fn resolve_primary(&self) -> Result<String> {
        let url = format!("{}/open-apis/calendar/v4/calendars/primary", self.api_base);
        let data = request_json("POST", &url, &self.access_token, Some(json!({}))).await?;
        parse_primary_calendar_id(&data)
    }
}

#[async_trait]
impl CalendarApi for HttpCalendar {
    async fn get_event(&self, calendar_id: &str, event_id: &str) -> Result<Option<RemoteEvent>> {
        let url = format!(
            "{}/open-apis/calendar/v4/calendars/{}/events/{}",
            self.api_base,
            form_path(calendar_id),
            form_path(event_id)
        );
        match request_json("GET", &url, &self.access_token, None).await {
            Ok(data) => {
                let event = data.get("event").unwrap_or(&data);
                if event_is_cancelled(event) {
                    return Ok(None);
                }
                parse_remote_event(event).map(Some)
            }
            Err(err) if is_missing_event(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn insert_event(&self, calendar_id: &str, draft: &EventDraft) -> Result<RemoteEvent> {
        let url = format!(
            "{}/open-apis/calendar/v4/calendars/{}/events?idempotency_key={}",
            self.api_base,
            form_path(calendar_id),
            form_path(&idempotency_key(&draft.task_id))
        );
        let data = request_json(
            "POST",
            &url,
            &self.access_token,
            Some(draft_to_feishu_json(draft)),
        )
        .await?;
        parse_remote_event(data.get("event").unwrap_or(&data))
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        draft: &EventDraft,
    ) -> Result<RemoteEvent> {
        let url = format!(
            "{}/open-apis/calendar/v4/calendars/{}/events/{}",
            self.api_base,
            form_path(calendar_id),
            form_path(event_id)
        );
        let data = request_json(
            "PATCH",
            &url,
            &self.access_token,
            Some(draft_to_feishu_json(draft)),
        )
        .await?;
        parse_remote_event(data.get("event").unwrap_or(&data))
    }
}

pub fn parse_public_config(raw: Option<&str>) -> FeishuPublicConfig {
    let value = raw
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}));
    let app_id = value
        .get("app_id")
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
    let domain = normalize_domain(
        value
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or("feishu"),
    );
    FeishuPublicConfig {
        app_id,
        calendar_id,
        domain,
    }
}

pub fn serialize_public_config(config: &FeishuPublicConfig) -> String {
    json!({
        "app_id": config.app_id,
        "calendar_id": config.calendar_id,
        "domain": config.domain,
    })
    .to_string()
}

pub fn sanitize_public_config(config: Option<String>) -> Option<String> {
    config.map(|value| serialize_public_config(&parse_public_config(Some(&value))))
}

pub fn draft_to_feishu_json(draft: &EventDraft) -> Value {
    let (start_time, end_time) = match (&draft.start, &draft.end) {
        (EventMoment::Date(start), EventMoment::Date(end)) => (
            json!({ "date": start, "timezone": "UTC" }),
            json!({ "date": end, "timezone": "UTC" }),
        ),
        (EventMoment::DateTime(start), EventMoment::DateTime(end)) => (
            json!({ "timestamp": unix_string(start), "timezone": "UTC" }),
            json!({ "timestamp": unix_string(end), "timezone": "UTC" }),
        ),
        _ => (
            json!({ "timestamp": "0", "timezone": "UTC" }),
            json!({ "timestamp": "3600", "timezone": "UTC" }),
        ),
    };
    json!({
        "summary": draft.summary,
        "description": draft.description.clone().unwrap_or_default(),
        "need_notification": false,
        "start_time": start_time,
        "end_time": end_time,
        "vchat": { "vc_type": "no_meeting" },
    })
}

pub fn parse_remote_event(value: &Value) -> Result<RemoteEvent> {
    let value = value.get("event").unwrap_or(value);
    let id = value
        .get("event_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Feishu event did not include an event_id"))?
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
    let (start_unix, end_unix, all_day) = parse_bounds(value);
    let updated_unix = value
        .get("create_time")
        .and_then(json_i64_value)
        .unwrap_or(0);
    Ok(RemoteEvent {
        id,
        summary,
        description,
        start_unix,
        end_unix,
        all_day,
        updated_unix,
        task_id: None,
    })
}

pub fn parse_primary_calendar_id(data: &Value) -> Result<String> {
    let calendars = data
        .get("calendars")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Feishu did not return a primary calendar"))?;
    let primary = calendars
        .iter()
        .find(|item| item.pointer("/calendar/type").and_then(Value::as_str) == Some("primary"));
    let chosen = primary.or_else(|| calendars.first());
    chosen
        .and_then(|item| item.pointer("/calendar/calendar_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Feishu did not return a primary calendar"))
}

pub fn event_is_cancelled(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("cancelled")
}

pub fn fingerprint(event: &RemoteEvent) -> String {
    let payload = format!(
        "{}\n{}\n{}\n{}\n{}",
        event.summary,
        event.description.clone().unwrap_or_default(),
        event.start_unix.unwrap_or_default(),
        event.end_unix.unwrap_or_default(),
        u8::from(event.all_day),
    );
    let digest = Sha256::digest(payload.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn remote_revision(last_synced: i64, stored_hash: &str, remote: &RemoteEvent) -> i64 {
    if stored_hash.is_empty() || stored_hash == fingerprint(remote) {
        last_synced
    } else {
        last_synced.saturating_add(1)
    }
}

pub fn idempotency_key(task_id: &str) -> String {
    let mut key = if task_id.len() >= 32 {
        task_id.to_string()
    } else {
        format!("taskmanager-{task_id}")
    };
    if key.len() < 32 {
        key = format!("{key:-<32}");
    }
    if key.len() > 128 {
        key.truncate(128);
    }
    key
}

pub fn interpret_api_response(status: u16, body: &str) -> Result<Value> {
    if status == 401 {
        bail!("feishu api status 401");
    }
    let parsed: Value = if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(body).unwrap_or_else(|_| json!({ "msg": body }))
    };
    let code = json_i64(&parsed, "code").unwrap_or(0);
    if code == 193001 || code == 193003 || (status == 404 && code == 0) {
        bail!("feishu event not found");
    }
    if matches!(code, 99991663 | 99991664 | 99991668 | 99991661) {
        bail!("feishu api status 401");
    }
    if !(200..300).contains(&status) || code != 0 {
        let msg = parsed
            .get("msg")
            .or_else(|| parsed.get("error_description"))
            .and_then(Value::as_str)
            .unwrap_or("Feishu calendar request failed");
        bail!("feishu api status {status}: {msg}");
    }
    Ok(parsed.get("data").cloned().unwrap_or(json!({})))
}

pub fn credential_flags(conn: &Connection) -> Result<(bool, bool)> {
    Ok((secret_row_present(conn)?, token_row_present(conn)?))
}

pub fn connection_view(
    conn: &Connection,
    store: &TokenStore,
    account: &SyncAccount,
) -> Result<FeishuConnection> {
    let config = parse_public_config(account.config.as_deref());
    let has_tokens = store.load_feishu_tokens(conn)?.is_some();
    let has_app_secret = store.load_feishu_app_secret(conn)?.is_some();
    Ok(FeishuConnection {
        connected: has_tokens && account.status == "authenticated",
        status: account.status.clone(),
        app_id: config.app_id,
        calendar_id: config.calendar_id,
        domain: config.domain,
        has_app_secret,
        redirect_uri: REDIRECT_URI.to_string(),
        last_error: account.last_error.clone(),
    })
}

pub fn save_settings(
    conn: &Connection,
    store: &TokenStore,
    input: &SaveFeishuSettingsInput,
    currently_enabled: bool,
) -> Result<()> {
    let existing = parse_public_config(account_config(conn)?.as_deref());
    let app_id = input.app_id.trim().to_string();
    let calendar_id = input
        .calendar_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.calendar_id.as_str())
        .to_string();
    let domain = input
        .domain
        .as_deref()
        .map(normalize_domain)
        .unwrap_or_else(|| existing.domain.clone());
    if app_id != existing.app_id || domain != existing.domain {
        store.delete_feishu_tokens(conn)?;
    }
    if let Some(secret) = &input.app_secret {
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            store.delete_feishu_app_secret(conn)?;
            store.delete_feishu_tokens(conn)?;
        } else {
            let current = store.load_feishu_app_secret(conn)?;
            if current.as_deref() != Some(trimmed) {
                store.save_feishu_app_secret(conn, trimmed)?;
                store.delete_feishu_tokens(conn)?;
            }
        }
    }
    let enabled = input.enabled.unwrap_or(currently_enabled);
    let has_secret = store.load_feishu_app_secret(conn)?.is_some();
    let has_tokens = store.load_feishu_tokens(conn)?.is_some();
    let status = if !enabled {
        "not_configured"
    } else if app_id.is_empty() || !has_secret || !has_tokens {
        "needs_auth"
    } else {
        "authenticated"
    };
    let config = serialize_public_config(&FeishuPublicConfig {
        app_id,
        calendar_id,
        domain,
    });
    crate::modules::sync::upsert_account(
        conn,
        UpsertSyncAccountInput {
            provider: FEISHU_PROVIDER.to_string(),
            enabled,
            config: Some(config),
        },
    )?;
    set_provider_feedback(conn, FEISHU_PROVIDER, status, None)
}

pub fn blocked_result(
    conn: &Connection,
    account: &SyncAccount,
    has_secret: bool,
    has_tokens: bool,
) -> Result<Option<SyncRunResult>> {
    if !account.enabled {
        return Ok(Some(SyncRunResult {
            provider: FEISHU_PROVIDER.to_string(),
            status: "not_configured".to_string(),
            message: "Provider is disabled. Enable it in Settings before syncing.".to_string(),
            affected_tasks: 0,
        }));
    }
    let config = parse_public_config(account.config.as_deref());
    if config.app_id.is_empty() || !has_secret || !has_tokens {
        let affected = mark_mappings(conn, "needs_auth")?;
        let message = if config.app_id.is_empty() {
            "Feishu sync requires OAuth App ID before live sync."
        } else if !has_secret {
            "Feishu sync requires OAuth App Secret before live sync."
        } else {
            "Feishu sync requires OAuth authorization before live sync."
        };
        set_provider_feedback(conn, FEISHU_PROVIDER, "needs_auth", Some(message))?;
        return Ok(Some(SyncRunResult {
            provider: FEISHU_PROVIDER.to_string(),
            status: "needs_auth".to_string(),
            message: message.to_string(),
            affected_tasks: affected,
        }));
    }
    Ok(None)
}

pub fn mark_needs_auth(conn: &Connection, message: &str) -> Result<SyncRunResult> {
    let affected = mark_mappings(conn, "needs_auth")?;
    set_provider_feedback(conn, FEISHU_PROVIDER, "needs_auth", Some(message))?;
    Ok(SyncRunResult {
        provider: FEISHU_PROVIDER.to_string(),
        status: "needs_auth".to_string(),
        message: message.to_string(),
        affected_tasks: affected,
    })
}

pub async fn compute_plans(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshots: Vec<TaskSnapshot>,
) -> Result<Vec<FeishuPlan>> {
    let mut plans = Vec::new();
    for snapshot in snapshots {
        match plan_for_snapshot(api, calendar_id, offset_seconds, &snapshot).await {
            Ok(plan) => plans.push(plan),
            Err(err) if is_auth_failure(&err) => return Err(err),
            Err(err) => plans.push(FeishuPlan {
                outcome: SyncOutcome::Failed {
                    task_id: snapshot.task.id,
                    message: err.to_string(),
                },
                remote_hash: None,
            }),
        }
    }
    Ok(plans)
}

pub fn apply_plans(conn: &Connection, plans: &[FeishuPlan]) -> Result<SyncRunResult> {
    let outcomes: Vec<SyncOutcome> = plans.iter().map(|plan| plan.outcome.clone()).collect();
    let result = apply_provider_outcomes(conn, FEISHU_PROVIDER, "Feishu Calendar", &outcomes)?;
    for plan in plans {
        let Some(hash) = &plan.remote_hash else {
            continue;
        };
        conn.execute(
            "UPDATE sync_mappings SET remote_hash = ?3 WHERE task_id = ?1 AND provider = ?2",
            params![task_id_of(&plan.outcome), FEISHU_PROVIDER, hash],
        )?;
    }
    Ok(result)
}

pub async fn plan_resolution(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshot: TaskSnapshot,
    strategy: &str,
) -> Result<FeishuPlan> {
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
                bail!("Feishu event no longer exists");
            };
            let remote_hash = Some(fingerprint(&remote));
            Ok(FeishuPlan {
                outcome: SyncOutcome::Pull {
                    task: snapshot.task,
                    remote,
                },
                remote_hash,
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
            let remote_hash = Some(fingerprint(&saved));
            Ok(FeishuPlan {
                outcome: SyncOutcome::Clean {
                    task_id: snapshot.task.id,
                    external_id: saved.id,
                    local_updated: snapshot.task.updated_at,
                    remote_updated: snapshot.task.updated_at,
                },
                remote_hash,
            })
        }
    }
}

pub fn load_feishu_snapshots(conn: &Connection) -> Result<Vec<TaskSnapshot>> {
    load_provider_snapshots(conn, FEISHU_PROVIDER)
}

async fn plan_for_snapshot(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshot: &TaskSnapshot,
) -> Result<FeishuPlan> {
    let remote = if snapshot.external_id.is_empty() {
        None
    } else {
        api.get_event(calendar_id, &snapshot.external_id).await?
    };
    let action = match &remote {
        None => SyncAction::CreateRemote,
        Some(event) => {
            let updated = remote_revision(snapshot.last_synced, &snapshot.remote_hash, event);
            decide_sync_action(
                snapshot.task.updated_at,
                Some(updated),
                snapshot.last_synced,
            )
        }
    };
    match action {
        SyncAction::AlreadySynced => {
            let event = remote.ok_or_else(|| anyhow!("already synced requires a remote event"))?;
            Ok(FeishuPlan {
                remote_hash: Some(fingerprint(&event)),
                outcome: SyncOutcome::Clean {
                    task_id: snapshot.task.id.clone(),
                    external_id: snapshot.external_id.clone(),
                    local_updated: snapshot.last_synced,
                    remote_updated: snapshot.last_synced,
                },
            })
        }
        SyncAction::Conflict => Ok(FeishuPlan {
            outcome: SyncOutcome::Conflict {
                task_id: snapshot.task.id.clone(),
            },
            remote_hash: None,
        }),
        SyncAction::PullRemote => {
            let event = remote.ok_or_else(|| anyhow!("pull requires a remote event"))?;
            let remote_hash = Some(fingerprint(&event));
            Ok(FeishuPlan {
                outcome: SyncOutcome::Pull {
                    task: snapshot.task.clone(),
                    remote: event,
                },
                remote_hash,
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
            let remote_hash = Some(fingerprint(&saved));
            Ok(FeishuPlan {
                outcome: SyncOutcome::Clean {
                    task_id: snapshot.task.id.clone(),
                    external_id: saved.id,
                    local_updated: snapshot.task.updated_at,
                    remote_updated: snapshot.task.updated_at,
                },
                remote_hash,
            })
        }
    }
}

fn task_id_of(outcome: &SyncOutcome) -> &str {
    match outcome {
        SyncOutcome::Clean { task_id, .. }
        | SyncOutcome::Conflict { task_id }
        | SyncOutcome::Failed { task_id, .. } => task_id,
        SyncOutcome::Pull { task, .. } => &task.id,
    }
}

fn account_config(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT config FROM sync_accounts WHERE provider = 'feishu'")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(row.get(0)?)
}

fn secret_row_present(conn: &Connection) -> Result<bool> {
    row_present(conn, SECRET_ID)
}

fn token_row_present(conn: &Connection) -> Result<bool> {
    row_present(conn, TOKEN_ID)
}

fn row_present(conn: &Connection, id: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM oauth_secrets WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn mark_mappings(conn: &Connection, status: &str) -> Result<i64> {
    let rows = conn.execute(
        "UPDATE sync_mappings SET sync_status = ?2 WHERE provider = ?1",
        params![FEISHU_PROVIDER, status],
    )?;
    Ok(rows as i64)
}

fn parse_bounds(value: &Value) -> (Option<i64>, Option<i64>, bool) {
    let start = value.get("start_time");
    let end = value.get("end_time");
    if let Some(date) = start
        .and_then(|item| item.get("date"))
        .and_then(Value::as_str)
    {
        if !date.is_empty() {
            let start_unix = date_to_unix(date);
            let end_unix = end
                .and_then(|item| item.get("date"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .and_then(date_to_unix);
            return (start_unix, end_unix, true);
        }
    }
    let start_unix = start
        .and_then(|item| item.get("timestamp"))
        .and_then(json_i64_value);
    let end_unix = end
        .and_then(|item| item.get("timestamp"))
        .and_then(json_i64_value);
    (start_unix, end_unix, false)
}

fn date_to_unix(date: &str) -> Option<i64> {
    let date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    Some(naive.and_utc().timestamp())
}

fn unix_string(rfc3339: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map(|value| value.timestamp().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn json_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(json_i64_value)
}

fn json_i64_value(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
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
    let status = response.status().as_u16();
    let text = response.text().await?;
    interpret_api_response(status, &text)
}

fn is_missing_event(err: &anyhow::Error) -> bool {
    err.to_string().contains("feishu event not found")
}

fn is_auth_failure(err: &anyhow::Error) -> bool {
    let text = err.to_string();
    text.contains("status 401") || text.contains("invalid_grant")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::CreateTaskInput;
    use crate::modules::db;
    use crate::modules::sync;
    use crate::modules::task as task_module;
    use chrono::DateTime;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn sample_task(all_day: bool) -> crate::models::task::Task {
        let start = DateTime::parse_from_rfc3339("2026-10-17T09:00:00Z")
            .unwrap()
            .timestamp();
        let end = DateTime::parse_from_rfc3339("2026-10-17T10:00:00Z")
            .unwrap()
            .timestamp();
        crate::models::task::Task {
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
    fn public_config_drops_app_secret_and_tokens() {
        let config = parse_public_config(Some(
            r#"{"app_id":"cli_test","calendar_id":"work","domain":"lark","app_secret":"super-secret","access_token":"token-secret"}"#,
        ));
        let stored = serialize_public_config(&config);
        assert!(stored.contains("cli_test"));
        assert!(stored.contains("work"));
        assert!(stored.contains("lark"));
        assert!(!stored.contains("super-secret"));
        assert!(!stored.contains("token-secret"));
    }

    #[test]
    fn maps_task_times_onto_feishu_event_json() {
        let draft = task_to_event(&sample_task(false), 0);
        let json = draft_to_feishu_json(&draft);
        let start = DateTime::parse_from_rfc3339("2026-10-17T09:00:00Z")
            .unwrap()
            .timestamp()
            .to_string();
        let end = DateTime::parse_from_rfc3339("2026-10-17T10:00:00Z")
            .unwrap()
            .timestamp()
            .to_string();
        assert_eq!(json["summary"], "Plan sprint");
        assert_eq!(json["description"], "Notes");
        assert_eq!(json["start_time"]["timestamp"], start);
        assert_eq!(json["end_time"]["timestamp"], end);
        assert_eq!(json["start_time"]["timezone"], "UTC");
        assert_eq!(json["vchat"]["vc_type"], "no_meeting");
        assert_eq!(json["need_notification"], false);
        assert!(json.get("app_secret").is_none());
        assert!(idempotency_key("task-1").len() >= 32);
    }

    #[test]
    fn maps_all_day_tasks_to_exclusive_utc_dates() {
        let mut task = sample_task(true);
        task.end_time = None;
        let draft = task_to_event(&task, 0);
        let json = draft_to_feishu_json(&draft);
        assert_eq!(json["start_time"]["date"], "2026-10-17");
        assert_eq!(json["end_time"]["date"], "2026-10-18");
    }

    #[test]
    fn parses_feishu_event_and_primary_calendar() {
        let start = DateTime::parse_from_rfc3339("2026-10-17T09:00:00Z")
            .unwrap()
            .timestamp();
        let value = json!({
            "event_id": "evt-1",
            "summary": "Plan sprint",
            "description": "Notes",
            "start_time": { "timestamp": start.to_string(), "timezone": "UTC" },
            "end_time": { "timestamp": (start + 3600).to_string(), "timezone": "UTC" },
            "status": "confirmed",
            "create_time": "1602504000"
        });
        let event = parse_remote_event(&value).unwrap();
        assert_eq!(event.id, "evt-1");
        assert_eq!(event.summary, "Plan sprint");
        assert_eq!(event.start_unix, Some(start));
        assert!(!event.all_day);
        assert!(!event_is_cancelled(&value));

        let all_day = json!({
            "event_id": "evt-day",
            "summary": "Offsite",
            "start_time": { "date": "2026-10-17" },
            "end_time": { "date": "2026-10-18" },
            "status": "cancelled"
        });
        let parsed = parse_remote_event(&all_day).unwrap();
        assert!(parsed.all_day);
        assert!(event_is_cancelled(&all_day));

        let primary = parse_primary_calendar_id(&json!({
            "calendars": [
                { "calendar": { "calendar_id": "shared", "type": "shared" } },
                { "calendar": { "calendar_id": "primary-cal", "type": "primary" } }
            ]
        }))
        .unwrap();
        assert_eq!(primary, "primary-cal");
    }

    #[test]
    fn api_errors_distinguish_missing_events_from_auth_and_config_failures() {
        let ok = interpret_api_response(200, r#"{"code":0,"data":{"event":{"event_id":"evt"}}}"#)
            .unwrap();
        assert_eq!(ok["event"]["event_id"], "evt");
        let missing =
            interpret_api_response(400, r#"{"code":193001,"msg":"event not found"}"#).unwrap_err();
        assert!(missing.to_string().contains("feishu event not found"));
        let expired =
            interpret_api_response(200, r#"{"code":99991663,"msg":"invalid token"}"#).unwrap_err();
        assert!(expired.to_string().contains("status 401"));
        let bad_calendar =
            interpret_api_response(400, r#"{"code":191001,"msg":"invalid calendar_id"}"#)
                .unwrap_err();
        assert!(bad_calendar.to_string().contains("invalid calendar_id"));
        assert!(!bad_calendar.to_string().contains("status 401"));
        let missing_calendar =
            interpret_api_response(404, r#"{"code":191000,"msg":"calendar not found"}"#)
                .unwrap_err();
        assert!(!missing_calendar
            .to_string()
            .contains("feishu event not found"));
    }

    #[test]
    fn fingerprint_detects_remote_edits_without_an_update_timestamp() {
        let event = parse_remote_event(&json!({
            "event_id": "evt-1",
            "summary": "Plan sprint",
            "description": "Notes",
            "start_time": { "timestamp": "100" },
            "end_time": { "timestamp": "200" }
        }))
        .unwrap();
        let same = fingerprint(&event);
        assert_eq!(remote_revision(300, &same, &event), 300);
        assert_eq!(remote_revision(300, "", &event), 300);
        let mut edited = event.clone();
        edited.summary = "Changed in Feishu".to_string();
        assert_eq!(remote_revision(300, &same, &edited), 301);
    }

    #[test]
    fn listed_account_config_omits_secrets_and_missing_credentials_do_not_succeed() {
        let conn = db::init_in_memory().unwrap();
        let task = task_module::create(
            &conn,
            CreateTaskInput {
                title: "Feishu task".to_string(),
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
        .unwrap();
        sync::upsert_account(
            &conn,
            UpsertSyncAccountInput {
                provider: "feishu".to_string(),
                enabled: true,
                config: Some(
                    r#"{"app_id":"cli_test","app_secret":"super-secret","calendar_id":"primary"}"#
                        .to_string(),
                ),
            },
        )
        .unwrap();
        sync::set_task_sync(
            &conn,
            crate::models::sync::SetTaskSyncInput {
                task_id: task.id.clone(),
                provider: "feishu".to_string(),
                enabled: true,
            },
        )
        .unwrap();
        let account = sync::list_accounts(&conn)
            .unwrap()
            .into_iter()
            .find(|item| item.provider == "feishu")
            .unwrap();
        let config = account.config.clone().unwrap();
        assert!(config.contains("cli_test"));
        assert!(!config.contains("super-secret"));

        let result = sync::sync_now(&conn, "feishu".to_string()).unwrap();
        assert_eq!(result.status, "needs_auth");
        assert!(result.message.contains("OAuth"));
        assert_ne!(result.status, "synced");
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "needs_auth");
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
            let event = remote_from_draft("evt-created", draft);
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
            let event = remote_from_draft(event_id, draft);
            self.events
                .lock()
                .unwrap()
                .insert(event.id.clone(), event.clone());
            Ok(event)
        }
    }

    fn remote_from_draft(id: &str, draft: &EventDraft) -> RemoteEvent {
        let (start_unix, end_unix, all_day) = match (&draft.start, &draft.end) {
            (EventMoment::DateTime(start), EventMoment::DateTime(end)) => (
                DateTime::parse_from_rfc3339(start)
                    .ok()
                    .map(|value| value.timestamp()),
                DateTime::parse_from_rfc3339(end)
                    .ok()
                    .map(|value| value.timestamp()),
                false,
            ),
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
            updated_unix: 0,
            task_id: Some(draft.task_id.clone()),
        }
    }

    fn opted_in(
        conn: &Connection,
        updated_at: i64,
        external_id: &str,
        last_synced: i64,
        remote_hash: &str,
    ) -> crate::models::task::Task {
        let task = task_module::create(
            conn,
            CreateTaskInput {
                title: "Plan sprint".to_string(),
                description: Some("Notes".to_string()),
                status: None,
                priority: None,
                start_time: Some(1_760_686_800),
                end_time: Some(1_760_690_400),
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
                provider: "feishu".to_string(),
                enabled: true,
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET external_id = ?3, last_synced = ?4, remote_hash = ?5, sync_status = 'pending' WHERE task_id = ?1 AND provider = ?2",
            params![task.id, "feishu", external_id, last_synced, remote_hash],
        )
        .unwrap();
        task_module::get_by_id(conn, &task.id).unwrap().unwrap()
    }

    async fn run_sync(conn: Connection, api: &FakeCalendar) -> (Connection, SyncRunResult) {
        let snapshots = load_feishu_snapshots(&conn).unwrap();
        let plans = compute_plans(api, "primary", 0, snapshots).await.unwrap();
        let result = apply_plans(&conn, &plans).unwrap();
        (conn, result)
    }

    fn stored_hash(conn: &Connection, task_id: &str) -> String {
        conn.query_row(
            "SELECT remote_hash FROM sync_mappings WHERE task_id = ?1 AND provider = 'feishu'",
            params![task_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn creates_a_feishu_event_for_a_placeholder_mapping() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 100, "", 0, "");
        let api = FakeCalendar {
            events: Mutex::new(HashMap::new()),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        assert!(result.message.contains("Feishu"));
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "synced");
        assert_eq!(state[0].external_id.as_deref(), Some("evt-created"));
        assert!(!stored_hash(&conn, &task.id).is_empty());
    }

    #[tokio::test]
    async fn pushes_local_edits_and_pulls_remote_edits() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300, "");
        let remote = RemoteEvent {
            id: "evt-1".to_string(),
            summary: "Old".to_string(),
            description: Some("Notes".to_string()),
            start_unix: Some(1_760_686_800),
            end_unix: Some(1_760_690_400),
            all_day: false,
            updated_unix: 0,
            task_id: None,
        };
        let hash = fingerprint(&remote);
        conn.execute(
            "UPDATE sync_mappings SET remote_hash = ?2 WHERE task_id = ?1",
            params![task.id, hash],
        )
        .unwrap();
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
        let current_hash = stored_hash(&conn, &task.id);
        conn.execute(
            "UPDATE sync_mappings SET last_synced = 300, remote_hash = 'stale-hash' WHERE task_id = ?1",
            params![task.id],
        )
        .unwrap();
        api.events.lock().unwrap().get_mut("evt-1").unwrap().summary = "From Feishu".to_string();
        let _ = current_hash;
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "From Feishu");
    }

    #[tokio::test]
    async fn marks_conflict_without_overwriting_either_side() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "evt-1", 300, "stale-hash");
        let remote = RemoteEvent {
            id: "evt-1".to_string(),
            summary: "From Feishu".to_string(),
            description: None,
            start_unix: Some(1_760_686_800),
            end_unix: Some(1_760_690_400),
            all_day: false,
            updated_unix: 0,
            task_id: None,
        };
        let api = FakeCalendar {
            events: Mutex::new(HashMap::from([(remote.id.clone(), remote)])),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "conflict");
        assert!(result.message.contains("conflict"));
        assert_eq!(
            api.events.lock().unwrap().get("evt-1").unwrap().summary,
            "From Feishu"
        );
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Plan sprint");
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "conflict");
        assert_eq!(stored_hash(&conn, &task.id), "stale-hash");
    }

    #[tokio::test]
    async fn empty_fingerprint_does_not_pull_until_a_later_remote_edit() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 300, "evt-1", 300, "");
        let remote = RemoteEvent {
            id: "evt-1".to_string(),
            summary: "From Feishu".to_string(),
            description: None,
            start_unix: Some(1_760_686_800),
            end_unix: Some(1_760_690_400),
            all_day: false,
            updated_unix: 0,
            task_id: None,
        };
        let api = FakeCalendar {
            events: Mutex::new(HashMap::from([(remote.id.clone(), remote)])),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "Plan sprint");
        assert!(!stored_hash(&conn, &task.id).is_empty());
    }

    #[test]
    fn endpoints_follow_the_saved_domain() {
        assert!(crate::modules::feishu_oauth::endpoints_for("feishu")
            .api_base
            .contains("feishu"));
        assert!(crate::modules::feishu_oauth::endpoints_for("lark")
            .api_base
            .contains("larksuite"));
    }
}
