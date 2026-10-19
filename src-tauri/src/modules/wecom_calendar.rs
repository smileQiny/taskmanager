use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::models::sync::{
    SaveWecomSettingsInput, SyncAccount, SyncRunResult, UpsertSyncAccountInput, WecomConnection,
};
use crate::modules::google_calendar::{
    apply_provider_outcomes, decide_sync_action, load_provider_snapshots, set_provider_feedback,
    task_to_event, CalendarApi, EventDraft, EventMoment, RemoteEvent, SyncAction, SyncOutcome,
    TaskSnapshot,
};
use crate::modules::token_store::{GoogleTokens, TokenStore};

pub const WECOM_PROVIDER: &str = "wecom";
const SECRET_ID: &str = "wecom.secret";
const TOKEN_ID: &str = "wecom.tokens";
const API_ROOT: &str = "https://qyapi.weixin.qq.com";
const SUMMARY_LIMIT: usize = 128;
const DESCRIPTION_LIMIT: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WecomPublicConfig {
    pub corp_id: String,
    pub agent_id: String,
    pub organizer_userid: String,
    pub calendar_id: String,
}

#[derive(Debug, Clone)]
pub struct WecomPlan {
    pub outcome: SyncOutcome,
    pub remote_hash: Option<String>,
}

pub struct HttpCalendar {
    access_token: String,
    agent_id: Option<u64>,
    organizer_userid: String,
    offset_seconds: i32,
}

impl HttpCalendar {
    pub fn new(
        access_token: impl Into<String>,
        agent_id: Option<u64>,
        organizer_userid: impl Into<String>,
        offset_seconds: i32,
    ) -> Self {
        Self {
            access_token: access_token.into(),
            agent_id,
            organizer_userid: organizer_userid.into(),
            offset_seconds,
        }
    }

    async fn find_schedule_id_for_task(
        &self,
        calendar_id: &str,
        task_id: &str,
    ) -> Result<Option<String>> {
        if !concrete_calendar_id(calendar_id) {
            return Ok(None);
        }
        let mut offset = 0_i64;
        loop {
            let page = request_json(
                "POST",
                &format!("{API_ROOT}/cgi-bin/oa/schedule/get_by_calendar"),
                &self.access_token,
                Some(json!({
                    "cal_id": calendar_id,
                    "offset": offset,
                    "limit": 1000,
                })),
            )
            .await?;
            let list = page
                .get("schedule_list")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let count = list.len() as i64;
            if let Some(schedule_id) = schedule_id_for_task(&list, task_id) {
                return Ok(Some(schedule_id));
            }
            if count < 1000 || offset >= 10_000 {
                return Ok(None);
            }
            offset += 1000;
        }
    }
}

#[async_trait]
impl CalendarApi for HttpCalendar {
    async fn get_event(&self, _calendar_id: &str, event_id: &str) -> Result<Option<RemoteEvent>> {
        let value = match request_json(
            "POST",
            &format!("{API_ROOT}/cgi-bin/oa/schedule/get"),
            &self.access_token,
            Some(json!({ "schedule_id_list": [event_id] })),
        )
        .await
        {
            Ok(value) => value,
            Err(err) if is_missing_schedule(&err) => return Ok(None),
            Err(err) => return Err(err),
        };
        let list = value
            .get("schedule_list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(schedule) = list
            .into_iter()
            .find(|item| item.get("schedule_id").and_then(Value::as_str) == Some(event_id))
        else {
            return Ok(None);
        };
        if schedule_is_cancelled(&schedule) {
            return Ok(None);
        }
        parse_remote_schedule(&schedule, self.offset_seconds).map(Some)
    }

    async fn insert_event(&self, calendar_id: &str, draft: &EventDraft) -> Result<RemoteEvent> {
        let existing = match self
            .find_schedule_id_for_task(calendar_id, &draft.task_id)
            .await
        {
            Ok(found) => found,
            Err(err) if is_auth_failure(&err) => return Err(err),
            Err(_) => None,
        };
        if let Some(existing) = existing {
            return self.update_event(calendar_id, &existing, draft).await;
        }
        let created = request_json(
            "POST",
            &format!("{API_ROOT}/cgi-bin/oa/schedule/add"),
            &self.access_token,
            Some(draft_to_wecom_json(
                draft,
                calendar_id,
                &self.organizer_userid,
                self.agent_id,
                self.offset_seconds,
            )),
        )
        .await?;
        let schedule_id = created
            .get("schedule_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("WeCom did not return a schedule_id"))?
            .to_string();
        self.get_event(calendar_id, &schedule_id)
            .await?
            .ok_or_else(|| anyhow!("WeCom created schedule {schedule_id} but did not return it"))
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        draft: &EventDraft,
    ) -> Result<RemoteEvent> {
        let mut body = draft_to_wecom_json(
            draft,
            calendar_id,
            &self.organizer_userid,
            self.agent_id,
            self.offset_seconds,
        );
        body["schedule"]["schedule_id"] = json!(event_id);
        request_json(
            "POST",
            &format!("{API_ROOT}/cgi-bin/oa/schedule/update"),
            &self.access_token,
            Some(body),
        )
        .await?;
        self.get_event(calendar_id, event_id).await?.ok_or_else(|| {
            anyhow!("WeCom updated schedule {event_id} but the schedule is no longer available")
        })
    }
}

pub async fn request_corp_token(corp_id: &str, secret: &str) -> Result<GoogleTokens> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{API_ROOT}/cgi-bin/gettoken"))
        .query(&[("corpid", corp_id), ("corpsecret", secret)])
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    let value = interpret_api_response(status, &text)?;
    parse_access_token(&value, chrono::Utc::now().timestamp())
}

pub async fn ensure_app_calendar(
    access_token: &str,
    agent_id: Option<u64>,
    organizer_userid: &str,
    calendar_id: &str,
) -> Result<String> {
    if concrete_calendar_id(calendar_id) {
        verify_calendar(access_token, calendar_id).await?;
        return Ok(calendar_id.to_string());
    }
    let created = request_json(
        "POST",
        &format!("{API_ROOT}/cgi-bin/oa/calendar/add"),
        access_token,
        Some(build_create_calendar_body(organizer_userid, agent_id)),
    )
    .await?;
    parse_created_calendar_id(&created)
}

pub fn parse_public_config(raw: Option<&str>) -> WecomPublicConfig {
    let value = raw
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}));
    let corp_id = text_field(&value, "corp_id");
    let agent_id = text_field(&value, "agent_id");
    let organizer_userid = text_field(&value, "organizer_userid");
    let calendar_id = value
        .get("calendar_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("primary")
        .to_string();
    WecomPublicConfig {
        corp_id,
        agent_id,
        organizer_userid,
        calendar_id,
    }
}

pub fn serialize_public_config(config: &WecomPublicConfig) -> String {
    json!({
        "corp_id": config.corp_id,
        "agent_id": config.agent_id,
        "organizer_userid": config.organizer_userid,
        "calendar_id": config.calendar_id,
    })
    .to_string()
}

pub fn sanitize_public_config(config: Option<String>) -> Option<String> {
    config.map(|value| serialize_public_config(&parse_public_config(Some(&value))))
}

pub fn parse_agent_id(value: &str) -> Option<u64> {
    let parsed = value.trim().parse::<u64>().ok()?;
    if parsed == 0 {
        None
    } else {
        Some(parsed)
    }
}

pub fn concrete_calendar_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value != "primary" && value != "default"
}

pub fn timezone_hours(offset_seconds: i32) -> i32 {
    let hours = (offset_seconds as f64 / 3600.0).round() as i32;
    hours.clamp(-12, 12)
}

pub fn draft_to_wecom_json(
    draft: &EventDraft,
    calendar_id: &str,
    organizer_userid: &str,
    agent_id: Option<u64>,
    offset_seconds: i32,
) -> Value {
    let all_day = matches!(draft.start, EventMoment::Date(_));
    let start_unix = moment_to_unix(&draft.start, offset_seconds);
    let mut end_unix = moment_to_unix(&draft.end, offset_seconds);
    if end_unix <= start_unix {
        end_unix = start_unix + 3600;
    }
    let mut schedule = json!({
        "admins": [organizer_userid],
        "attendees": [{ "userid": organizer_userid }],
        "summary": truncate_chars(&draft.summary, SUMMARY_LIMIT),
        "description": with_task_marker(draft.description.as_deref(), &draft.task_id, DESCRIPTION_LIMIT),
        "start_time": start_unix,
        "end_time": end_unix,
        "is_whole_day": if all_day { 1 } else { 0 },
        "reminders": {
            "is_remind": 0,
            "is_repeat": 0,
            "timezone": timezone_hours(offset_seconds),
        },
    });
    if concrete_calendar_id(calendar_id) {
        schedule["cal_id"] = json!(calendar_id);
    }
    let mut body = json!({ "schedule": schedule });
    if let Some(agent_id) = agent_id {
        body["agentid"] = json!(agent_id);
    }
    body
}

pub fn build_create_calendar_body(organizer_userid: &str, agent_id: Option<u64>) -> Value {
    let mut body = json!({
        "calendar": {
            "admins": [organizer_userid],
            "set_as_default": 1,
            "summary": "Task Manager",
            "color": "#4C6FFF",
            "description": "Opted-in tasks from Task Manager",
            "shares": [{ "userid": organizer_userid, "permission": 1 }],
        }
    });
    if let Some(agent_id) = agent_id {
        body["agentid"] = json!(agent_id);
    }
    body
}

pub fn parse_remote_schedule(value: &Value, offset_seconds: i32) -> Result<RemoteEvent> {
    let value = value.get("schedule").unwrap_or(value);
    let id = value
        .get("schedule_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("WeCom schedule did not include a schedule_id"))?
        .to_string();
    let summary = value
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let description = value
        .get("description")
        .and_then(Value::as_str)
        .map(strip_task_marker)
        .filter(|text| !text.is_empty());
    let (start_unix, end_unix, all_day) = parse_bounds(value, offset_seconds);
    Ok(RemoteEvent {
        id,
        summary,
        description,
        start_unix,
        end_unix,
        all_day,
        updated_unix: 0,
        task_id: value
            .get("description")
            .and_then(Value::as_str)
            .and_then(task_id_from_description),
    })
}

pub fn parse_created_calendar_id(value: &Value) -> Result<String> {
    let failed_share = value
        .pointer("/fail_result/shares")
        .and_then(Value::as_array)
        .is_some_and(|shares| !shares.is_empty());
    if failed_share {
        bail!(
            "WeCom could not share the calendar with the organizer userid. The userid must be inside the app's visible range."
        );
    }
    value
        .get("cal_id")
        .and_then(Value::as_str)
        .filter(|value| concrete_calendar_id(value))
        .map(str::to_string)
        .ok_or_else(|| anyhow!("WeCom did not return a calendar id"))
}

pub fn parse_access_token(value: &Value, now: i64) -> Result<GoogleTokens> {
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("WeCom did not return an access_token"))?
        .to_string();
    let expires_in = value
        .get("expires_in")
        .and_then(json_i64_value)
        .filter(|value| *value > 0)
        .unwrap_or(7200);
    Ok(GoogleTokens {
        access_token,
        refresh_token: None,
        expires_at: now.saturating_add(expires_in),
    })
}

pub fn schedule_is_cancelled(value: &Value) -> bool {
    json_i64(value, "status") == Some(1)
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

pub fn with_task_marker(description: Option<&str>, task_id: &str, max_chars: usize) -> String {
    let marker = format!("[taskmanager:{task_id}]");
    let marker_chars = marker.chars().count();
    let budget = max_chars.saturating_sub(marker_chars + 2);
    let body: String = description
        .unwrap_or("")
        .chars()
        .take(budget)
        .collect::<String>()
        .trim_end()
        .to_string();
    if body.is_empty() {
        marker
    } else {
        format!("{body}\n\n{marker}")
    }
}

pub fn strip_task_marker(text: &str) -> String {
    let mut lines: Vec<&str> = text
        .lines()
        .filter(|line| task_id_from_description(line).is_none())
        .collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

pub fn task_id_from_description(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("[taskmanager:") else {
            continue;
        };
        let Some(id) = rest.strip_suffix(']') else {
            continue;
        };
        if !id.is_empty() && !id.contains(['[', ']', ' ']) {
            return Some(id.to_string());
        }
    }
    None
}

pub fn interpret_api_response(status: u16, body: &str) -> Result<Value> {
    if status == 401 {
        bail!("wecom api status 401");
    }
    let parsed: Value = if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(body).unwrap_or_else(|_| json!({ "errmsg": body }))
    };
    let code = json_i64(&parsed, "errcode").unwrap_or(0);
    if code == 90460 || code == 90461 {
        bail!("wecom schedule not found");
    }
    if matches!(code, 40001 | 40014 | 41001 | 42001) {
        bail!("wecom api status 401");
    }
    if code == 60020 || code == 301042 {
        bail!(
            "WeCom rejected the request because this computer's IP is not in the app's trusted IP list (企业可信IP). Add the desktop's public IP in the app settings and retry in a minute."
        );
    }
    if !(200..300).contains(&status) || code != 0 {
        let msg = parsed
            .get("errmsg")
            .and_then(Value::as_str)
            .unwrap_or("WeCom request failed");
        bail!("wecom api status {status}: {msg} ({code})");
    }
    Ok(parsed)
}

pub fn credential_flags(conn: &Connection) -> Result<(bool, bool)> {
    Ok((secret_row_present(conn)?, token_row_present(conn)?))
}

pub fn connection_view(
    conn: &Connection,
    store: &TokenStore,
    account: &SyncAccount,
) -> Result<WecomConnection> {
    let config = parse_public_config(account.config.as_deref());
    let has_tokens = store.load_wecom_tokens(conn)?.is_some();
    let has_secret = store.load_wecom_secret(conn)?.is_some();
    Ok(WecomConnection {
        connected: has_tokens && account.status == "authenticated",
        status: account.status.clone(),
        corp_id: config.corp_id,
        agent_id: config.agent_id,
        organizer_userid: config.organizer_userid,
        calendar_id: config.calendar_id,
        has_secret,
        last_error: account.last_error.clone(),
    })
}

pub fn save_settings(
    conn: &Connection,
    store: &TokenStore,
    input: &SaveWecomSettingsInput,
    currently_enabled: bool,
) -> Result<()> {
    let existing = parse_public_config(account_config(conn)?.as_deref());
    let corp_id = input.corp_id.trim().to_string();
    let agent_id = input.agent_id.trim().to_string();
    let organizer_userid = input.organizer_userid.trim().to_string();
    let calendar_id = input
        .calendar_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(existing.calendar_id.as_str())
        .to_string();
    if corp_id != existing.corp_id || agent_id != existing.agent_id {
        store.delete_wecom_tokens(conn)?;
    }
    if let Some(secret) = &input.secret {
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            store.delete_wecom_secret(conn)?;
            store.delete_wecom_tokens(conn)?;
        } else {
            let current = store.load_wecom_secret(conn)?;
            if current.as_deref() != Some(trimmed) {
                store.save_wecom_secret(conn, trimmed)?;
                store.delete_wecom_tokens(conn)?;
            }
        }
    }
    let config = WecomPublicConfig {
        corp_id,
        agent_id,
        organizer_userid,
        calendar_id,
    };
    let enabled = input.enabled.unwrap_or(currently_enabled);
    let has_secret = store.load_wecom_secret(conn)?.is_some();
    let has_tokens = store.load_wecom_tokens(conn)?.is_some();
    let status = account_status(enabled, &config, has_secret, has_tokens);
    crate::modules::sync::upsert_account(
        conn,
        UpsertSyncAccountInput {
            provider: WECOM_PROVIDER.to_string(),
            enabled,
            config: Some(serialize_public_config(&config)),
        },
    )?;
    set_provider_feedback(conn, WECOM_PROVIDER, status, None)
}

pub fn store_calendar_id(conn: &Connection, calendar_id: &str) -> Result<()> {
    let mut config = parse_public_config(account_config(conn)?.as_deref());
    config.calendar_id = calendar_id.trim().to_string();
    let enabled = account_enabled(conn)?;
    crate::modules::sync::upsert_account(
        conn,
        UpsertSyncAccountInput {
            provider: WECOM_PROVIDER.to_string(),
            enabled,
            config: Some(serialize_public_config(&config)),
        },
    )?;
    Ok(())
}

pub fn blocked_result(
    conn: &Connection,
    account: &SyncAccount,
    has_secret: bool,
    has_tokens: bool,
) -> Result<Option<SyncRunResult>> {
    if !account.enabled {
        return Ok(Some(SyncRunResult {
            provider: WECOM_PROVIDER.to_string(),
            status: "not_configured".to_string(),
            message: "Provider is disabled. Enable it in Settings before syncing.".to_string(),
            affected_tasks: 0,
        }));
    }
    let config = parse_public_config(account.config.as_deref());
    let Some(message) = auth_block_message(&config, has_secret, has_tokens) else {
        return Ok(None);
    };
    let affected = mark_mappings(conn, "needs_auth")?;
    set_provider_feedback(conn, WECOM_PROVIDER, "needs_auth", Some(message))?;
    Ok(Some(SyncRunResult {
        provider: WECOM_PROVIDER.to_string(),
        status: "needs_auth".to_string(),
        message: message.to_string(),
        affected_tasks: affected,
    }))
}

pub fn mark_needs_auth(conn: &Connection, message: &str) -> Result<SyncRunResult> {
    let affected = mark_mappings(conn, "needs_auth")?;
    set_provider_feedback(conn, WECOM_PROVIDER, "needs_auth", Some(message))?;
    Ok(SyncRunResult {
        provider: WECOM_PROVIDER.to_string(),
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
) -> Result<Vec<WecomPlan>> {
    let mut plans = Vec::new();
    for snapshot in snapshots {
        match plan_for_snapshot(api, calendar_id, offset_seconds, &snapshot).await {
            Ok(plan) => plans.push(plan),
            Err(err) if is_auth_failure(&err) => return Err(err),
            Err(err) => plans.push(WecomPlan {
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

pub fn apply_plans(conn: &Connection, plans: &[WecomPlan]) -> Result<SyncRunResult> {
    let outcomes: Vec<SyncOutcome> = plans.iter().map(|plan| plan.outcome.clone()).collect();
    let result = apply_provider_outcomes(conn, WECOM_PROVIDER, "WeCom Calendar", &outcomes)?;
    for plan in plans {
        let Some(hash) = &plan.remote_hash else {
            continue;
        };
        conn.execute(
            "UPDATE sync_mappings SET remote_hash = ?3 WHERE task_id = ?1 AND provider = ?2",
            params![task_id_of(&plan.outcome), WECOM_PROVIDER, hash],
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
) -> Result<WecomPlan> {
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
                bail!("WeCom schedule no longer exists");
            };
            let remote_hash = Some(fingerprint(&remote));
            Ok(WecomPlan {
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
            Ok(WecomPlan {
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

pub fn load_wecom_snapshots(conn: &Connection) -> Result<Vec<TaskSnapshot>> {
    load_provider_snapshots(conn, WECOM_PROVIDER)
}

fn account_status(
    enabled: bool,
    config: &WecomPublicConfig,
    has_secret: bool,
    has_tokens: bool,
) -> &'static str {
    if !enabled {
        "not_configured"
    } else if auth_block_message(config, has_secret, has_tokens).is_none() {
        "authenticated"
    } else {
        "needs_auth"
    }
}

fn auth_block_message(
    config: &WecomPublicConfig,
    has_secret: bool,
    has_tokens: bool,
) -> Option<&'static str> {
    if config.corp_id.is_empty() {
        Some("WeCom sync requires a CorpID before live sync.")
    } else if parse_agent_id(&config.agent_id).is_none() {
        Some("WeCom sync requires an AgentId before live sync.")
    } else if config.organizer_userid.is_empty() || config.organizer_userid.len() > 64 {
        Some("WeCom sync requires an organizer userid before live sync.")
    } else if !has_secret {
        Some("WeCom sync requires a corp Secret before live sync.")
    } else if !has_tokens {
        Some("WeCom sync requires a verified corp access token before live sync.")
    } else if !concrete_calendar_id(&config.calendar_id) {
        Some("WeCom sync requires an app calendar. Connect WeCom to create one before live sync.")
    } else {
        None
    }
}

async fn plan_for_snapshot(
    api: &dyn CalendarApi,
    calendar_id: &str,
    offset_seconds: i32,
    snapshot: &TaskSnapshot,
) -> Result<WecomPlan> {
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
            let event =
                remote.ok_or_else(|| anyhow!("already synced requires a remote schedule"))?;
            Ok(WecomPlan {
                remote_hash: Some(fingerprint(&event)),
                outcome: SyncOutcome::Clean {
                    task_id: snapshot.task.id.clone(),
                    external_id: snapshot.external_id.clone(),
                    local_updated: snapshot.last_synced,
                    remote_updated: snapshot.last_synced,
                },
            })
        }
        SyncAction::Conflict => Ok(WecomPlan {
            outcome: SyncOutcome::Conflict {
                task_id: snapshot.task.id.clone(),
            },
            remote_hash: None,
        }),
        SyncAction::PullRemote => {
            let event = remote.ok_or_else(|| anyhow!("pull requires a remote schedule"))?;
            let remote_hash = Some(fingerprint(&event));
            Ok(WecomPlan {
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
            Ok(WecomPlan {
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
    let mut stmt = conn.prepare("SELECT config FROM sync_accounts WHERE provider = 'wecom'")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(row.get(0)?)
}

fn account_enabled(conn: &Connection) -> Result<bool> {
    let enabled: Option<i64> = conn
        .query_row(
            "SELECT enabled FROM sync_accounts WHERE provider = 'wecom'",
            [],
            |row| row.get(0),
        )
        .ok();
    Ok(enabled.unwrap_or(0) != 0)
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
        params![WECOM_PROVIDER, status],
    )?;
    Ok(rows as i64)
}

fn text_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn moment_to_unix(moment: &EventMoment, offset_seconds: i32) -> i64 {
    match moment {
        EventMoment::DateTime(value) => DateTime::parse_from_rfc3339(value)
            .map(|date| date.timestamp())
            .unwrap_or(0),
        EventMoment::Date(value) => date_midnight_unix(value, offset_seconds).unwrap_or(0),
    }
}

fn date_midnight_unix(date: &str, offset_seconds: i32) -> Option<i64> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    let offset = fixed_offset(timezone_hours(offset_seconds));
    offset
        .from_local_datetime(&naive)
        .single()
        .map(|value| value.timestamp())
}

fn parse_bounds(value: &Value, offset_seconds: i32) -> (Option<i64>, Option<i64>, bool) {
    let start = json_i64(value, "start_time");
    let end = json_i64(value, "end_time");
    let all_day = json_i64(value, "is_whole_day") == Some(1);
    if !all_day {
        return (start, end, false);
    }
    let hours = value
        .pointer("/reminders/timezone")
        .and_then(json_i64_value)
        .map(|value| value as i32)
        .unwrap_or_else(|| timezone_hours(offset_seconds))
        .clamp(-12, 12);
    let start_unix = start.and_then(|unix| utc_date_unix(&unix_to_offset_date(unix, hours)));
    let end_unix = end.and_then(|unix| utc_date_unix(&unix_to_offset_date(unix, hours)));
    (start_unix, end_unix, true)
}

fn unix_to_offset_date(unix: i64, hours: i32) -> String {
    fixed_offset(hours)
        .timestamp_opt(unix, 0)
        .single()
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

fn utc_date_unix(date: &str) -> Option<i64> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    Some(naive.and_utc().timestamp())
}

fn fixed_offset(hours: i32) -> FixedOffset {
    FixedOffset::east_opt(hours * 3600).unwrap_or(FixedOffset::east_opt(0).expect("utc offset"))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn schedule_id_for_task(list: &[Value], task_id: &str) -> Option<String> {
    list.iter().find_map(|item| {
        if schedule_is_cancelled(item) {
            return None;
        }
        let description = item.get("description").and_then(Value::as_str)?;
        if task_id_from_description(description).as_deref() != Some(task_id) {
            return None;
        }
        item.get("schedule_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
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

async fn verify_calendar(access_token: &str, calendar_id: &str) -> Result<()> {
    let value = request_json(
        "POST",
        &format!("{API_ROOT}/cgi-bin/oa/calendar/get"),
        access_token,
        Some(json!({ "cal_id_list": [calendar_id] })),
    )
    .await?;
    let found = value
        .get("calendar_list")
        .and_then(Value::as_array)
        .is_some_and(|list| {
            list.iter()
                .any(|item| item.get("cal_id").and_then(Value::as_str) == Some(calendar_id))
        });
    if !found {
        bail!(
            "WeCom calendar was not found. Check the calendar ID or connect again to create an app calendar."
        );
    }
    Ok(())
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
        .query(&[("access_token", access_token)]);
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

fn is_missing_schedule(err: &anyhow::Error) -> bool {
    err.to_string().contains("wecom schedule not found")
}

fn is_auth_failure(err: &anyhow::Error) -> bool {
    let text = err.to_string();
    text.contains("status 401") || text.contains("invalid_grant")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sync::SetTaskSyncInput;
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
    fn public_config_drops_secret_and_tokens() {
        let config = parse_public_config(Some(
            r#"{"corp_id":"ww123","agent_id":"1000002","organizer_userid":"zhangsan","calendar_id":"wc-cal","secret":"super-secret","corpsecret":"also-secret","access_token":"token-secret"}"#,
        ));
        let stored = serialize_public_config(&config);
        assert!(stored.contains("ww123"));
        assert!(stored.contains("1000002"));
        assert!(stored.contains("zhangsan"));
        assert!(stored.contains("wc-cal"));
        assert!(!stored.contains("super-secret"));
        assert!(!stored.contains("also-secret"));
        assert!(!stored.contains("token-secret"));
    }

    #[test]
    fn maps_timed_task_onto_wecom_schedule_json() {
        let draft = task_to_event(&sample_task(false), 0);
        let json = draft_to_wecom_json(&draft, "wc-cal", "zhangsan", Some(1000002), 0);
        let start = DateTime::parse_from_rfc3339("2026-10-17T09:00:00Z")
            .unwrap()
            .timestamp();
        assert_eq!(json["schedule"]["summary"], "Plan sprint");
        assert_eq!(json["schedule"]["start_time"], start);
        assert_eq!(json["schedule"]["end_time"], start + 3600);
        assert_eq!(json["schedule"]["is_whole_day"], 0);
        assert_eq!(json["schedule"]["attendees"][0]["userid"], "zhangsan");
        assert_eq!(json["schedule"]["admins"][0], "zhangsan");
        assert_eq!(json["schedule"]["cal_id"], "wc-cal");
        assert_eq!(json["schedule"]["reminders"]["is_repeat"], 0);
        assert_eq!(json["schedule"]["reminders"]["timezone"], 0);
        assert_eq!(json["agentid"], 1000002);
        assert!(json.get("secret").is_none());
        assert!(json["schedule"]["description"]
            .as_str()
            .unwrap()
            .contains("[taskmanager:task-1]"));
        let mut schedule = json["schedule"].clone();
        schedule["schedule_id"] = json!("sch-1");
        let event = parse_remote_schedule(&schedule, 0).unwrap();
        assert_eq!(event.description.as_deref(), Some("Notes"));
        assert_eq!(event.task_id.as_deref(), Some("task-1"));
        assert!(!event.all_day);
    }

    #[test]
    fn maps_all_day_tasks_to_exclusive_offset_midnights() {
        let mut task = sample_task(true);
        task.end_time = None;
        let draft = task_to_event(&task, 8 * 3600);
        let json = draft_to_wecom_json(&draft, "primary", "zhangsan", None, 8 * 3600);
        assert_eq!(json["schedule"]["is_whole_day"], 1);
        assert!(json["schedule"].get("cal_id").is_none());
        assert!(json.get("agentid").is_none());
        let start = json["schedule"]["start_time"].as_i64().unwrap();
        let end = json["schedule"]["end_time"].as_i64().unwrap();
        assert_eq!(end - start, 86_400);
        let zone = FixedOffset::east_opt(8 * 3600).unwrap();
        let start_local = zone.timestamp_opt(start, 0).unwrap();
        assert_eq!(
            start_local.format("%Y-%m-%d %H:%M").to_string(),
            "2026-10-17 00:00"
        );
        let mut schedule = json["schedule"].clone();
        schedule["schedule_id"] = json!("sch-day");
        let parsed = parse_remote_schedule(&schedule, 8 * 3600).unwrap();
        assert!(parsed.all_day);
        assert_eq!(parsed.start_unix, utc_date_unix("2026-10-17"));
        assert_eq!(parsed.end_unix, utc_date_unix("2026-10-18"));
    }

    #[test]
    fn truncates_summary_and_description_to_wecom_limits() {
        let mut task = sample_task(false);
        task.title = "标".repeat(200);
        task.description = Some("记".repeat(1200));
        let draft = task_to_event(&task, 0);
        let json = draft_to_wecom_json(&draft, "wc-cal", "zhangsan", None, 0);
        assert_eq!(
            json["schedule"]["summary"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            128
        );
        assert!(
            json["schedule"]["description"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= 1000
        );
        assert!(
            task_id_from_description(json["schedule"]["description"].as_str().unwrap()).is_some()
        );
    }

    #[test]
    fn api_errors_distinguish_auth_missing_schedules_and_trusted_ip() {
        let ok = interpret_api_response(
            200,
            r#"{"errcode":0,"errmsg":"ok","access_token":"abc","expires_in":7200}"#,
        )
        .unwrap();
        let tokens = parse_access_token(&ok, 1_000).unwrap();
        assert_eq!(tokens.access_token, "abc");
        assert_eq!(tokens.expires_at, 8_200);
        assert!(tokens.refresh_token.is_none());
        let expired = interpret_api_response(200, r#"{"errcode":42001,"errmsg":"token expired"}"#)
            .unwrap_err();
        assert!(expired.to_string().contains("status 401"));
        let missing =
            interpret_api_response(200, r#"{"errcode":90460,"errmsg":"deleted"}"#).unwrap_err();
        assert!(missing.to_string().contains("wecom schedule not found"));
        let blocked = interpret_api_response(200, r#"{"errcode":60020,"errmsg":"not allow ip"}"#)
            .unwrap_err();
        assert!(blocked.to_string().contains("企业可信IP"));
        assert!(!blocked.to_string().contains("status 401"));
        let bad_corp =
            interpret_api_response(200, r#"{"errcode":40013,"errmsg":"invalid corpid"}"#)
                .unwrap_err();
        assert!(bad_corp.to_string().contains("invalid corpid"));
        assert!(!bad_corp.to_string().contains("status 401"));
    }

    #[test]
    fn calendar_create_body_shares_the_organizer_and_rejects_failed_shares() {
        let body = build_create_calendar_body("zhangsan", Some(1000002));
        assert_eq!(body["calendar"]["summary"], "Task Manager");
        assert_eq!(body["calendar"]["admins"][0], "zhangsan");
        assert_eq!(body["calendar"]["shares"][0]["userid"], "zhangsan");
        assert_eq!(body["calendar"]["set_as_default"], 1);
        assert!(body.get("secret").is_none());
        assert_eq!(
            parse_created_calendar_id(&json!({"errcode":0,"cal_id":"wc-cal"})).unwrap(),
            "wc-cal"
        );
        let failed = parse_created_calendar_id(&json!({
            "errcode": 0,
            "cal_id": "wc-cal",
            "fail_result": { "shares": [{ "userid": "zhangsan", "errcode": 81013 }] }
        }))
        .unwrap_err();
        assert!(failed.to_string().contains("visible range"));
    }

    #[test]
    fn cancelled_schedules_and_fingerprint_revisions_match_feishu_rules() {
        let value = json!({
            "schedule_id": "sch-1",
            "summary": "Plan sprint",
            "description": "Notes\n\n[taskmanager:task-1]",
            "start_time": 100,
            "end_time": 200,
            "status": 0,
            "is_whole_day": 0
        });
        assert!(!schedule_is_cancelled(&value));
        let event = parse_remote_schedule(&value, 0).unwrap();
        let same = fingerprint(&event);
        assert_eq!(remote_revision(300, &same, &event), 300);
        assert_eq!(remote_revision(300, "", &event), 300);
        let mut edited = event.clone();
        edited.summary = "Changed in WeCom".to_string();
        assert_eq!(remote_revision(300, &same, &edited), 301);
        let cancelled = json!({ "schedule_id": "sch-1", "status": 1 });
        assert!(schedule_is_cancelled(&cancelled));
    }

    #[test]
    fn listed_account_strips_secrets_and_missing_credentials_do_not_succeed() {
        let conn = db::init_in_memory().unwrap();
        let task = task_module::create(
            &conn,
            CreateTaskInput {
                title: "WeCom task".to_string(),
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
                provider: "wecom".to_string(),
                enabled: true,
                config: Some(
                    r#"{"corp_id":"ww123","agent_id":"1000002","organizer_userid":"zhangsan","calendar_id":"primary","secret":"super-secret","access_token":"token-secret"}"#
                        .to_string(),
                ),
            },
        )
        .unwrap();
        sync::set_task_sync(
            &conn,
            SetTaskSyncInput {
                task_id: task.id.clone(),
                provider: "wecom".to_string(),
                enabled: true,
            },
        )
        .unwrap();
        let account = sync::list_accounts(&conn)
            .unwrap()
            .into_iter()
            .find(|item| item.provider == "wecom")
            .unwrap();
        let config = account.config.clone().unwrap();
        assert!(config.contains("ww123"));
        assert!(!config.contains("super-secret"));
        assert!(!config.contains("token-secret"));
        let result = sync::sync_now(&conn, "wecom".to_string()).unwrap();
        assert_eq!(result.status, "needs_auth");
        assert!(result.message.contains("Secret"));
        assert_ne!(result.status, "synced");
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "needs_auth");
    }

    #[test]
    fn saved_token_without_an_app_calendar_does_not_report_synced() {
        let conn = db::init_in_memory().unwrap();
        let store = TokenStore::for_tests([4u8; 32]);
        save_settings(
            &conn,
            &store,
            &SaveWecomSettingsInput {
                corp_id: "ww123".to_string(),
                agent_id: "1000002".to_string(),
                secret: Some("super-secret".to_string()),
                organizer_userid: "zhangsan".to_string(),
                calendar_id: Some("primary".to_string()),
                enabled: Some(true),
            },
            false,
        )
        .unwrap();
        store
            .save_wecom_tokens(
                &conn,
                &GoogleTokens {
                    access_token: "access-secret".to_string(),
                    refresh_token: None,
                    expires_at: 1_800_000_000,
                },
            )
            .unwrap();
        let result = sync::sync_now(&conn, "wecom".to_string()).unwrap();
        assert_eq!(result.status, "needs_auth");
        assert!(result.message.contains("calendar"));
        assert_ne!(result.status, "synced");
        let stored: String = conn
            .query_row(
                "SELECT ciphertext FROM oauth_secrets WHERE id = 'wecom.secret'",
                [],
                |row| {
                    let bytes: Vec<u8> = row.get(0)?;
                    Ok(String::from_utf8_lossy(&bytes).to_string())
                },
            )
            .unwrap();
        assert!(!stored.contains("super-secret"));
    }

    #[test]
    fn complete_credentials_wait_for_the_async_command() {
        let conn = db::init_in_memory().unwrap();
        let store = TokenStore::for_tests([5u8; 32]);
        save_settings(
            &conn,
            &store,
            &SaveWecomSettingsInput {
                corp_id: "ww123".to_string(),
                agent_id: "1000002".to_string(),
                secret: Some("super-secret".to_string()),
                organizer_userid: "zhangsan".to_string(),
                calendar_id: Some("wc-cal".to_string()),
                enabled: Some(true),
            },
            false,
        )
        .unwrap();
        store
            .save_wecom_tokens(
                &conn,
                &GoogleTokens {
                    access_token: "access-secret".to_string(),
                    refresh_token: None,
                    expires_at: 1_800_000_000,
                },
            )
            .unwrap();
        let error = sync::sync_now(&conn, "wecom".to_string()).unwrap_err();
        assert!(error.to_string().contains("async"));
        let view = connection_view(
            &conn,
            &store,
            &sync::list_accounts(&conn)
                .unwrap()
                .into_iter()
                .find(|item| item.provider == "wecom")
                .unwrap(),
        )
        .unwrap();
        assert!(view.has_secret);
        assert!(!view.corp_id.is_empty());
        assert!(!format!("{view:?}").contains("super-secret"));
        assert!(!format!("{view:?}").contains("access-secret"));
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
            let event = remote_from_draft("sch-created", draft);
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
                (utc_date_unix(start), utc_date_unix(end), true)
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
            SetTaskSyncInput {
                task_id: task.id.clone(),
                provider: "wecom".to_string(),
                enabled: true,
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET external_id = ?3, last_synced = ?4, remote_hash = ?5, sync_status = 'pending' WHERE task_id = ?1 AND provider = ?2",
            params![task.id, "wecom", external_id, last_synced, remote_hash],
        )
        .unwrap();
        task_module::get_by_id(conn, &task.id).unwrap().unwrap()
    }

    async fn run_sync(conn: Connection, api: &FakeCalendar) -> (Connection, SyncRunResult) {
        let snapshots = load_wecom_snapshots(&conn).unwrap();
        let plans = compute_plans(api, "wc-cal", 0, snapshots).await.unwrap();
        let result = apply_plans(&conn, &plans).unwrap();
        (conn, result)
    }

    fn stored_hash(conn: &Connection, task_id: &str) -> String {
        conn.query_row(
            "SELECT remote_hash FROM sync_mappings WHERE task_id = ?1 AND provider = 'wecom'",
            params![task_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn creates_a_wecom_schedule_for_a_placeholder_mapping() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 100, "", 0, "");
        let api = FakeCalendar {
            events: Mutex::new(HashMap::new()),
        };
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        assert!(result.message.contains("WeCom"));
        let state = sync::list_task_sync_states(&conn, &task.id).unwrap();
        assert_eq!(state[0].sync_status, "synced");
        assert_eq!(state[0].external_id.as_deref(), Some("sch-created"));
        assert!(!stored_hash(&conn, &task.id).is_empty());
    }

    #[tokio::test]
    async fn pushes_local_edits_and_pulls_remote_edits() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "sch-1", 300, "");
        let remote = RemoteEvent {
            id: "sch-1".to_string(),
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
            api.events.lock().unwrap().get("sch-1").unwrap().summary,
            "Plan sprint"
        );

        conn.execute(
            "UPDATE tasks SET updated_at = 300 WHERE id = ?1",
            params![task.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE sync_mappings SET last_synced = 300, remote_hash = 'stale-hash' WHERE task_id = ?1",
            params![task.id],
        )
        .unwrap();
        api.events.lock().unwrap().get_mut("sch-1").unwrap().summary = "From WeCom".to_string();
        let (conn, result) = run_sync(conn, &api).await;
        assert_eq!(result.status, "synced");
        let updated = task_module::get_by_id(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.title, "From WeCom");
    }

    #[tokio::test]
    async fn marks_conflict_without_overwriting_either_side() {
        let conn = db::init_in_memory().unwrap();
        let task = opted_in(&conn, 400, "sch-1", 300, "stale-hash");
        let remote = RemoteEvent {
            id: "sch-1".to_string(),
            summary: "From WeCom".to_string(),
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
            api.events.lock().unwrap().get("sch-1").unwrap().summary,
            "From WeCom"
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
        let task = opted_in(&conn, 300, "sch-1", 300, "");
        let remote = RemoteEvent {
            id: "sch-1".to_string(),
            summary: "From WeCom".to_string(),
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
    fn schedule_lookup_ignores_cancelled_rows() {
        let list = vec![
            json!({
                "schedule_id": "old",
                "description": "[taskmanager:task-1]",
                "status": 1
            }),
            json!({
                "schedule_id": "live",
                "description": "Notes\n\n[taskmanager:task-1]",
                "status": 0
            }),
        ];
        assert_eq!(
            schedule_id_for_task(&list, "task-1").as_deref(),
            Some("live")
        );
    }
}
