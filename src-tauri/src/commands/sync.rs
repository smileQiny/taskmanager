use tauri::State;

use crate::models::sync::{
    FeishuConnection, GoogleConnection, MacosCalendarConnection, ResolveSyncConflictInput,
    SaveFeishuSettingsInput, SaveGoogleSettingsInput, SaveMacosCalendarSettingsInput,
    SaveWecomSettingsInput, SetTaskSyncInput, SyncAccount, SyncRunResult, TaskSyncState,
    UpsertSyncAccountInput, WecomConnection,
};
use crate::modules::feishu_calendar::{self, HttpCalendar as FeishuCalendar};
use crate::modules::feishu_oauth::{self, endpoints_for};
use crate::modules::google_calendar::{self, HttpCalendar};
use crate::modules::google_oauth::{interactive_authorization, refresh_access_token};
use crate::modules::macos_calendar;
use crate::modules::sync as sync_module;
use crate::modules::token_store::{GoogleTokens, TokenStore};
use crate::modules::wecom_calendar::{self, HttpCalendar as WecomCalendar};
use crate::AppState;

#[tauri::command]
pub fn list_sync_accounts(state: State<AppState>) -> Result<Vec<SyncAccount>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::list_accounts(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_sync_account(
    state: State<AppState>,
    input: UpsertSyncAccountInput,
) -> Result<SyncAccount, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if input.provider == google_calendar::GOOGLE_PROVIDER {
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let current = sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == google_calendar::GOOGLE_PROVIDER);
        let config = google_calendar::parse_public_google_config(
            input.config.as_deref().or(current
                .as_ref()
                .and_then(|account| account.config.as_deref())),
        );
        google_calendar::save_settings(
            &conn,
            &store,
            &SaveGoogleSettingsInput {
                client_id: config.client_id,
                calendar_id: Some(config.calendar_id),
                client_secret: None,
                enabled: Some(input.enabled),
            },
            current
                .as_ref()
                .map(|account| account.enabled)
                .unwrap_or(false),
        )
        .map_err(|e| e.to_string())?;
        return sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == google_calendar::GOOGLE_PROVIDER)
            .ok_or_else(|| "google account missing".to_string());
    }
    if input.provider == feishu_calendar::FEISHU_PROVIDER {
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let current = sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == feishu_calendar::FEISHU_PROVIDER);
        let config = feishu_calendar::parse_public_config(
            input.config.as_deref().or(current
                .as_ref()
                .and_then(|account| account.config.as_deref())),
        );
        feishu_calendar::save_settings(
            &conn,
            &store,
            &SaveFeishuSettingsInput {
                app_id: config.app_id,
                app_secret: None,
                calendar_id: Some(config.calendar_id),
                domain: Some(config.domain),
                enabled: Some(input.enabled),
            },
            current
                .as_ref()
                .map(|account| account.enabled)
                .unwrap_or(false),
        )
        .map_err(|e| e.to_string())?;
        return sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == feishu_calendar::FEISHU_PROVIDER)
            .ok_or_else(|| "feishu account missing".to_string());
    }
    if input.provider == macos_calendar::MACOS_PROVIDER {
        let current = sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == macos_calendar::MACOS_PROVIDER);
        let config = macos_calendar::parse_config(
            input.config.as_deref().or(current
                .as_ref()
                .and_then(|account| account.config.as_deref())),
        );
        macos_calendar::save_settings(
            &conn,
            &SaveMacosCalendarSettingsInput {
                calendar_id: Some(config.calendar_id),
                calendar_name: Some(config.calendar_name),
                enabled: Some(input.enabled),
            },
            current
                .as_ref()
                .map(|account| account.enabled)
                .unwrap_or(false),
            macos_calendar::live_access(),
        )
        .map_err(|e| e.to_string())?;
        return sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == macos_calendar::MACOS_PROVIDER)
            .ok_or_else(|| "macos account missing".to_string());
    }
    if input.provider == wecom_calendar::WECOM_PROVIDER {
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let current = sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == wecom_calendar::WECOM_PROVIDER);
        let config = wecom_calendar::parse_public_config(
            input.config.as_deref().or(current
                .as_ref()
                .and_then(|account| account.config.as_deref())),
        );
        wecom_calendar::save_settings(
            &conn,
            &store,
            &SaveWecomSettingsInput {
                corp_id: config.corp_id,
                agent_id: config.agent_id,
                secret: None,
                organizer_userid: config.organizer_userid,
                calendar_id: Some(config.calendar_id),
                enabled: Some(input.enabled),
            },
            current
                .as_ref()
                .map(|account| account.enabled)
                .unwrap_or(false),
        )
        .map_err(|e| e.to_string())?;
        return sync_module::list_accounts(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|account| account.provider == wecom_calendar::WECOM_PROVIDER)
            .ok_or_else(|| "wecom account missing".to_string());
    }
    sync_module::upsert_account(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_provider_now(
    state: State<'_, AppState>,
    provider: String,
) -> Result<SyncRunResult, String> {
    if provider == google_calendar::GOOGLE_PROVIDER {
        return sync_google(&state).await;
    }
    if provider == macos_calendar::MACOS_PROVIDER {
        return sync_macos(&state).await;
    }
    if provider == feishu_calendar::FEISHU_PROVIDER {
        return sync_feishu(&state).await;
    }
    if provider == wecom_calendar::WECOM_PROVIDER {
        return sync_wecom(&state).await;
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::sync_now(&conn, provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_task_sync_states(
    state: State<AppState>,
    task_id: String,
) -> Result<Vec<TaskSyncState>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::list_task_sync_states(&conn, &task_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_task_sync(
    state: State<AppState>,
    input: SetTaskSyncInput,
) -> Result<Vec<TaskSyncState>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_module::set_task_sync(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn google_connection_status(state: State<AppState>) -> Result<GoogleConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    let account = google_account(&conn)?;
    google_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_google_settings(
    state: State<AppState>,
    input: SaveGoogleSettingsInput,
) -> Result<GoogleConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    let account = google_account(&conn)?;
    google_calendar::save_settings(&conn, &store, &input, account.enabled)
        .map_err(|e| e.to_string())?;
    let account = google_account(&conn)?;
    google_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_google_calendar(
    state: State<'_, AppState>,
) -> Result<GoogleConnection, String> {
    let (client_id, client_secret) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = google_account(&conn)?;
        let config = google_calendar::parse_public_google_config(account.config.as_deref());
        if config.client_id.is_empty() {
            return Err("Google Calendar requires an OAuth Client ID".to_string());
        }
        let secret = store.load_client_secret(&conn).map_err(|e| e.to_string())?;
        (config.client_id, secret)
    };
    let tokens = interactive_authorization(&client_id, client_secret.as_deref(), true)
        .await
        .map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    store
        .save_tokens(&conn, &tokens)
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sync_accounts SET enabled = 1 WHERE provider = 'google'",
        [],
    )
    .map_err(|e| e.to_string())?;
    google_calendar::set_account_feedback(&conn, "authenticated", None)
        .map_err(|e| e.to_string())?;
    let account = google_account(&conn)?;
    google_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_google_calendar(state: State<AppState>) -> Result<GoogleConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    store.delete_tokens(&conn).map_err(|e| e.to_string())?;
    google_calendar::mark_needs_auth(
        &conn,
        "Google Calendar was disconnected. Connect again to sync.",
    )
    .map_err(|e| e.to_string())?;
    let account = google_account(&conn)?;
    google_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_task_sync_conflict(
    state: State<'_, AppState>,
    input: ResolveSyncConflictInput,
) -> Result<SyncRunResult, String> {
    if input.provider == macos_calendar::MACOS_PROVIDER {
        return resolve_macos(&state, input).await;
    }
    if input.provider == feishu_calendar::FEISHU_PROVIDER {
        return resolve_feishu(&state, input).await;
    }
    if input.provider == wecom_calendar::WECOM_PROVIDER {
        return resolve_wecom(&state, input).await;
    }
    if input.provider != google_calendar::GOOGLE_PROVIDER {
        return Err(format!("{} sync is not implemented", input.provider));
    }
    let (calendar_id, snapshot, access_token) = {
        let prepared = prepare_google_access(&state).await?;
        let PreparedGoogle {
            calendar_id,
            access_token,
            ..
        } = prepared;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let snapshot = google_calendar::load_google_snapshots(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.task.id == input.task_id)
            .ok_or_else(|| "task is not opted in to Google Calendar".to_string())?;
        (calendar_id, snapshot, access_token)
    };
    let api = HttpCalendar::new(access_token);
    let offset = local_offset_seconds();
    let outcome =
        google_calendar::plan_resolution(&api, &calendar_id, offset, snapshot, &input.strategy)
            .await
            .map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    google_calendar::apply_outcomes(&conn, &[outcome]).map_err(|e| e.to_string())
}

struct PreparedGoogle {
    calendar_id: String,
    access_token: String,
}

async fn sync_google(state: &State<'_, AppState>) -> Result<SyncRunResult, String> {
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = google_account(&conn)?;
        let has_tokens = store
            .load_tokens(&conn)
            .map_err(|e| e.to_string())?
            .is_some();
        if let Some(blocked) = google_calendar::blocked_result(&conn, &account, has_tokens)
            .map_err(|e| e.to_string())?
        {
            return Ok(blocked);
        }
    }
    let PreparedGoogle {
        calendar_id,
        access_token,
    } = prepare_google_access(state).await?;
    let snapshots = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        google_calendar::load_google_snapshots(&conn).map_err(|e| e.to_string())?
    };
    let api = HttpCalendar::new(access_token);
    let outcomes =
        google_calendar::compute_outcomes(&api, &calendar_id, local_offset_seconds(), snapshots)
            .await
            .map_err(|err| map_google_auth_error(state, err.to_string()))?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    google_calendar::apply_outcomes(&conn, &outcomes).map_err(|e| e.to_string())
}

async fn prepare_google_access(state: &State<'_, AppState>) -> Result<PreparedGoogle, String> {
    let (client_id, client_secret, calendar_id, tokens) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = google_account(&conn)?;
        let config = google_calendar::parse_public_google_config(account.config.as_deref());
        let secret = store.load_client_secret(&conn).map_err(|e| e.to_string())?;
        let tokens = store.load_tokens(&conn).map_err(|e| e.to_string())?;
        (config.client_id, secret, config.calendar_id, tokens)
    };
    let Some(tokens) = tokens else {
        return Err(
            "Google Calendar sync requires OAuth authorization before live sync.".to_string(),
        );
    };
    let fresh = match ensure_fresh_tokens(&client_id, client_secret.as_deref(), tokens).await {
        Ok(tokens) => tokens,
        Err(err) => return Err(map_google_auth_error(state, err)),
    };
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        store
            .save_tokens(&conn, &fresh)
            .map_err(|e| e.to_string())?;
    }
    Ok(PreparedGoogle {
        calendar_id,
        access_token: fresh.access_token,
    })
}

async fn ensure_fresh_tokens(
    client_id: &str,
    client_secret: Option<&str>,
    tokens: GoogleTokens,
) -> Result<GoogleTokens, String> {
    let now = chrono::Utc::now().timestamp();
    if tokens.expires_at > now + 60 {
        return Ok(tokens);
    }
    refresh_access_token(client_id, client_secret, &tokens)
        .await
        .map_err(|err| err.to_string())
}

fn map_google_auth_error(state: &State<'_, AppState>, message: String) -> String {
    if message.contains("invalid_grant") || message.contains("status 401") {
        if let Ok(conn) = state.db.lock() {
            if let Ok(store) = TokenStore::for_app(&state.data_dir) {
                let _ = store.delete_tokens(&conn);
            }
            let _ = google_calendar::mark_needs_auth(
                &conn,
                "Google authorization expired. Connect Google Calendar again.",
            );
        }
        return "Google authorization expired. Connect Google Calendar again.".to_string();
    }
    message
}

fn google_account(conn: &rusqlite::Connection) -> Result<SyncAccount, String> {
    sync_module::list_accounts(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|account| account.provider == google_calendar::GOOGLE_PROVIDER)
        .ok_or_else(|| "google account missing".to_string())
}

fn local_offset_seconds() -> i32 {
    chrono::Local::now().offset().local_minus_utc()
}

#[tauri::command]
pub fn macos_calendar_status(state: State<AppState>) -> Result<MacosCalendarConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let account = macos_account(&conn)?;
    let access = macos_calendar::live_access();
    let calendars = list_macos_calendars().map_err(|e| e.to_string())?;
    Ok(macos_calendar::connection_view(&account, access, calendars))
}

#[tauri::command]
pub fn save_macos_calendar_settings(
    state: State<AppState>,
    input: SaveMacosCalendarSettingsInput,
) -> Result<MacosCalendarConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let account = macos_account(&conn)?;
    macos_calendar::save_settings(
        &conn,
        &input,
        account.enabled,
        macos_calendar::live_access(),
    )
    .map_err(|e| e.to_string())?;
    let account = macos_account(&conn)?;
    let access = macos_calendar::live_access();
    let calendars = list_macos_calendars().map_err(|e| e.to_string())?;
    Ok(macos_calendar::connection_view(&account, access, calendars))
}

#[tauri::command]
pub async fn request_macos_calendar_access(
    state: State<'_, AppState>,
) -> Result<MacosCalendarConnection, String> {
    if !macos_calendar::platform_supported() {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let account = macos_account(&conn)?;
        return Ok(macos_calendar::connection_view(
            &account,
            macos_calendar::CalendarAccess::Unsupported,
            Vec::new(),
        ));
    }
    #[cfg(target_os = "macos")]
    {
        // Resolve permission before taking the database lock. The prompt can
        // sit open until the user decides, and the connection mutex must stay
        // available to the rest of the app during that wait.
        let access = crate::modules::macos_eventkit::request_full_access()
            .await
            .map_err(|e| e.to_string())?;
        let account = {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let account = macos_account(&conn)?;
            macos_calendar::save_settings(
                &conn,
                &SaveMacosCalendarSettingsInput {
                    calendar_id: None,
                    calendar_name: None,
                    enabled: Some(true),
                },
                account.enabled,
                access,
            )
            .map_err(|e| e.to_string())?;
            macos_account(&conn)?
        };
        let calendars = list_macos_calendars().map_err(|e| e.to_string())?;
        return Ok(macos_calendar::connection_view(&account, access, calendars));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
        Err("macOS Calendar sync is only available on macOS.".to_string())
    }
}

async fn sync_macos(state: &State<'_, AppState>) -> Result<SyncRunResult, String> {
    if !macos_calendar::platform_supported() {
        return Ok(macos_calendar::unsupported_result());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = state;
        return Ok(macos_calendar::unsupported_result());
    }
    #[cfg(target_os = "macos")]
    {
        let (calendar_id, calendar_name, snapshots) = {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let account = macos_account(&conn)?;
            let access = macos_calendar::live_access();
            if let Some(blocked) = macos_calendar::blocked_result(&conn, &account, access)
                .map_err(|e| e.to_string())?
            {
                return Ok(blocked);
            }
            let config = macos_calendar::parse_config(account.config.as_deref());
            let snapshots =
                google_calendar::load_provider_snapshots(&conn, macos_calendar::MACOS_PROVIDER)
                    .map_err(|e| e.to_string())?;
            (config.calendar_id, config.calendar_name, snapshots)
        };
        let api =
            crate::modules::macos_eventkit::EventKitApi::new(calendar_id.clone(), calendar_name);
        let outcomes = google_calendar::compute_outcomes(
            &api,
            &calendar_id,
            local_offset_seconds(),
            snapshots,
        )
        .await
        .map_err(|e| e.to_string())?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        google_calendar::apply_provider_outcomes(
            &conn,
            macos_calendar::MACOS_PROVIDER,
            "macOS Calendar",
            &outcomes,
        )
        .map_err(|e| e.to_string())
    }
}

async fn resolve_macos(
    state: &State<'_, AppState>,
    input: ResolveSyncConflictInput,
) -> Result<SyncRunResult, String> {
    if !macos_calendar::platform_supported() {
        return Err("macOS Calendar sync is only available on macOS.".to_string());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (state, input);
        return Err("macOS Calendar sync is only available on macOS.".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        let (calendar_id, calendar_name, snapshot) = {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let account = macos_account(&conn)?;
            let access = macos_calendar::live_access();
            if macos_calendar::blocked_result(&conn, &account, access)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err(
                    "macOS Calendar access is required before resolving a conflict.".to_string(),
                );
            }
            let config = macos_calendar::parse_config(account.config.as_deref());
            let snapshot =
                google_calendar::load_provider_snapshots(&conn, macos_calendar::MACOS_PROVIDER)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .find(|item| item.task.id == input.task_id)
                    .ok_or_else(|| "task is not opted in to macOS Calendar".to_string())?;
            (config.calendar_id, config.calendar_name, snapshot)
        };
        let api =
            crate::modules::macos_eventkit::EventKitApi::new(calendar_id.clone(), calendar_name);
        let outcome = google_calendar::plan_resolution(
            &api,
            &calendar_id,
            local_offset_seconds(),
            snapshot,
            &input.strategy,
        )
        .await
        .map_err(|e| e.to_string())?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        google_calendar::apply_provider_outcomes(
            &conn,
            macos_calendar::MACOS_PROVIDER,
            "macOS Calendar",
            &[outcome],
        )
        .map_err(|e| e.to_string())
    }
}

fn list_macos_calendars() -> Result<Vec<crate::models::sync::MacosCalendarInfo>, anyhow::Error> {
    #[cfg(target_os = "macos")]
    {
        return crate::modules::macos_eventkit::list_writable_calendars();
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

fn macos_account(conn: &rusqlite::Connection) -> Result<SyncAccount, String> {
    sync_module::list_accounts(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|account| account.provider == macos_calendar::MACOS_PROVIDER)
        .ok_or_else(|| "macos account missing".to_string())
}

#[tauri::command]
pub fn feishu_connection_status(state: State<AppState>) -> Result<FeishuConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    let account = feishu_account(&conn)?;
    feishu_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_feishu_settings(
    state: State<AppState>,
    input: SaveFeishuSettingsInput,
) -> Result<FeishuConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    let account = feishu_account(&conn)?;
    feishu_calendar::save_settings(&conn, &store, &input, account.enabled)
        .map_err(|e| e.to_string())?;
    let account = feishu_account(&conn)?;
    feishu_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_feishu_calendar(
    state: State<'_, AppState>,
) -> Result<FeishuConnection, String> {
    let (app_id, app_secret, domain) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = feishu_account(&conn)?;
        let config = feishu_calendar::parse_public_config(account.config.as_deref());
        if config.app_id.is_empty() {
            return Err("Feishu Calendar requires an OAuth App ID".to_string());
        }
        let secret = store
            .load_feishu_app_secret(&conn)
            .map_err(|e| e.to_string())?
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Feishu Calendar requires an OAuth App Secret".to_string())?;
        (config.app_id, secret, config.domain)
    };
    let tokens = feishu_oauth::interactive_authorization(&domain, &app_id, &app_secret, true)
        .await
        .map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    store
        .save_feishu_tokens(&conn, &tokens)
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sync_accounts SET enabled = 1 WHERE provider = 'feishu'",
        [],
    )
    .map_err(|e| e.to_string())?;
    google_calendar::set_provider_feedback(
        &conn,
        feishu_calendar::FEISHU_PROVIDER,
        "authenticated",
        None,
    )
    .map_err(|e| e.to_string())?;
    let account = feishu_account(&conn)?;
    feishu_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_feishu_calendar(state: State<AppState>) -> Result<FeishuConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    store
        .delete_feishu_tokens(&conn)
        .map_err(|e| e.to_string())?;
    feishu_calendar::mark_needs_auth(
        &conn,
        "Feishu Calendar was disconnected. Connect again to sync.",
    )
    .map_err(|e| e.to_string())?;
    let account = feishu_account(&conn)?;
    feishu_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

async fn sync_feishu(state: &State<'_, AppState>) -> Result<SyncRunResult, String> {
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = feishu_account(&conn)?;
        let has_secret = store
            .load_feishu_app_secret(&conn)
            .map_err(|e| e.to_string())?
            .is_some();
        let has_tokens = store
            .load_feishu_tokens(&conn)
            .map_err(|e| e.to_string())?
            .is_some();
        if let Some(blocked) =
            feishu_calendar::blocked_result(&conn, &account, has_secret, has_tokens)
                .map_err(|e| e.to_string())?
        {
            return Ok(blocked);
        }
    }
    let PreparedFeishu {
        api_base,
        calendar_id,
        access_token,
    } = prepare_feishu_access(state).await?;
    let api = FeishuCalendar::new(access_token, api_base);
    let calendar_id = if calendar_id == "primary" {
        api.resolve_primary()
            .await
            .map_err(|err| map_feishu_auth_error(state, err.to_string()))?
    } else {
        calendar_id
    };
    let snapshots = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        feishu_calendar::load_feishu_snapshots(&conn).map_err(|e| e.to_string())?
    };
    let plans =
        feishu_calendar::compute_plans(&api, &calendar_id, local_offset_seconds(), snapshots)
            .await
            .map_err(|err| map_feishu_auth_error(state, err.to_string()))?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    feishu_calendar::apply_plans(&conn, &plans).map_err(|e| e.to_string())
}

async fn resolve_feishu(
    state: &State<'_, AppState>,
    input: ResolveSyncConflictInput,
) -> Result<SyncRunResult, String> {
    let (calendar_id, api_base, access_token, snapshot) = {
        let prepared = prepare_feishu_access(state).await?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let snapshot = feishu_calendar::load_feishu_snapshots(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.task.id == input.task_id)
            .ok_or_else(|| "task is not opted in to Feishu Calendar".to_string())?;
        (
            prepared.calendar_id,
            prepared.api_base,
            prepared.access_token,
            snapshot,
        )
    };
    let api = FeishuCalendar::new(access_token, api_base);
    let calendar_id = if calendar_id == "primary" {
        api.resolve_primary()
            .await
            .map_err(|err| map_feishu_auth_error(state, err.to_string()))?
    } else {
        calendar_id
    };
    let plan = feishu_calendar::plan_resolution(
        &api,
        &calendar_id,
        local_offset_seconds(),
        snapshot,
        &input.strategy,
    )
    .await
    .map_err(|err| map_feishu_auth_error(state, err.to_string()))?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    feishu_calendar::apply_plans(&conn, &[plan]).map_err(|e| e.to_string())
}

struct PreparedFeishu {
    api_base: String,
    calendar_id: String,
    access_token: String,
}

async fn prepare_feishu_access(state: &State<'_, AppState>) -> Result<PreparedFeishu, String> {
    let (app_id, app_secret, domain, calendar_id, tokens) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = feishu_account(&conn)?;
        let config = feishu_calendar::parse_public_config(account.config.as_deref());
        let secret = store
            .load_feishu_app_secret(&conn)
            .map_err(|e| e.to_string())?;
        let tokens = store.load_feishu_tokens(&conn).map_err(|e| e.to_string())?;
        (
            config.app_id,
            secret,
            config.domain,
            config.calendar_id,
            tokens,
        )
    };
    let Some(tokens) = tokens else {
        return Err(
            "Feishu Calendar sync requires OAuth authorization before live sync.".to_string(),
        );
    };
    let Some(app_secret) = app_secret.filter(|secret| !secret.is_empty()) else {
        return Err(
            "Feishu Calendar sync requires an OAuth App Secret before live sync.".to_string(),
        );
    };
    if app_id.is_empty() {
        return Err("Feishu Calendar sync requires an OAuth App ID before live sync.".to_string());
    }
    let fresh = match ensure_fresh_feishu_tokens(&domain, &app_id, &app_secret, tokens).await {
        Ok(tokens) => tokens,
        Err(err) => return Err(map_feishu_auth_error(state, err)),
    };
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        store
            .save_feishu_tokens(&conn, &fresh)
            .map_err(|e| e.to_string())?;
    }
    Ok(PreparedFeishu {
        api_base: endpoints_for(&domain).api_base.to_string(),
        calendar_id,
        access_token: fresh.access_token,
    })
}

async fn ensure_fresh_feishu_tokens(
    domain: &str,
    app_id: &str,
    app_secret: &str,
    tokens: GoogleTokens,
) -> Result<GoogleTokens, String> {
    let now = chrono::Utc::now().timestamp();
    if tokens.expires_at > now + 60 {
        return Ok(tokens);
    }
    feishu_oauth::refresh_access_token(endpoints_for(domain).token_url, app_id, app_secret, &tokens)
        .await
        .map_err(|err| err.to_string())
}

fn map_feishu_auth_error(state: &State<'_, AppState>, message: String) -> String {
    if message.contains("invalid_grant") || message.contains("status 401") {
        if let Ok(conn) = state.db.lock() {
            if let Ok(store) = TokenStore::for_app(&state.data_dir) {
                let _ = store.delete_feishu_tokens(&conn);
            }
            let _ = feishu_calendar::mark_needs_auth(
                &conn,
                "Feishu authorization expired. Connect Feishu Calendar again.",
            );
        }
        return "Feishu authorization expired. Connect Feishu Calendar again.".to_string();
    }
    message
}

fn feishu_account(conn: &rusqlite::Connection) -> Result<SyncAccount, String> {
    sync_module::list_accounts(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|account| account.provider == feishu_calendar::FEISHU_PROVIDER)
        .ok_or_else(|| "feishu account missing".to_string())
}

#[tauri::command]
pub fn wecom_connection_status(state: State<AppState>) -> Result<WecomConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    let account = wecom_account(&conn)?;
    wecom_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_wecom_settings(
    state: State<AppState>,
    input: SaveWecomSettingsInput,
) -> Result<WecomConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    let account = wecom_account(&conn)?;
    wecom_calendar::save_settings(&conn, &store, &input, account.enabled)
        .map_err(|e| e.to_string())?;
    let account = wecom_account(&conn)?;
    wecom_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_wecom_calendar(state: State<'_, AppState>) -> Result<WecomConnection, String> {
    let (corp_id, secret, agent_id, organizer_userid, calendar_id) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = wecom_account(&conn)?;
        let config = wecom_calendar::parse_public_config(account.config.as_deref());
        if config.corp_id.is_empty() {
            return Err("WeCom Calendar requires a CorpID".to_string());
        }
        let Some(agent_id) = wecom_calendar::parse_agent_id(&config.agent_id) else {
            return Err("WeCom Calendar requires an AgentId".to_string());
        };
        if config.organizer_userid.is_empty() || config.organizer_userid.len() > 64 {
            return Err("WeCom Calendar requires an organizer userid".to_string());
        }
        let secret = store
            .load_wecom_secret(&conn)
            .map_err(|e| e.to_string())?
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "WeCom Calendar requires a corp Secret".to_string())?;
        (
            config.corp_id,
            secret,
            agent_id,
            config.organizer_userid,
            config.calendar_id,
        )
    };
    let tokens = wecom_calendar::request_corp_token(&corp_id, &secret)
        .await
        .map_err(|e| e.to_string())?;
    let resolved_calendar = wecom_calendar::ensure_app_calendar(
        &tokens.access_token,
        Some(agent_id),
        &organizer_userid,
        &calendar_id,
    )
    .await
    .map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    store
        .save_wecom_tokens(&conn, &tokens)
        .map_err(|e| e.to_string())?;
    wecom_calendar::store_calendar_id(&conn, &resolved_calendar).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sync_accounts SET enabled = 1 WHERE provider = 'wecom'",
        [],
    )
    .map_err(|e| e.to_string())?;
    google_calendar::set_provider_feedback(
        &conn,
        wecom_calendar::WECOM_PROVIDER,
        "authenticated",
        None,
    )
    .map_err(|e| e.to_string())?;
    let account = wecom_account(&conn)?;
    wecom_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_wecom_calendar(state: State<AppState>) -> Result<WecomConnection, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
    store
        .delete_wecom_tokens(&conn)
        .map_err(|e| e.to_string())?;
    wecom_calendar::mark_needs_auth(
        &conn,
        "WeCom Calendar was disconnected. Connect again to sync.",
    )
    .map_err(|e| e.to_string())?;
    let account = wecom_account(&conn)?;
    wecom_calendar::connection_view(&conn, &store, &account).map_err(|e| e.to_string())
}

async fn sync_wecom(state: &State<'_, AppState>) -> Result<SyncRunResult, String> {
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = wecom_account(&conn)?;
        let has_secret = store
            .load_wecom_secret(&conn)
            .map_err(|e| e.to_string())?
            .is_some();
        let has_tokens = store
            .load_wecom_tokens(&conn)
            .map_err(|e| e.to_string())?
            .is_some();
        if let Some(blocked) =
            wecom_calendar::blocked_result(&conn, &account, has_secret, has_tokens)
                .map_err(|e| e.to_string())?
        {
            return Ok(blocked);
        }
    }
    let prepared = prepare_wecom_access(state).await?;
    let api = WecomCalendar::new(
        prepared.access_token,
        prepared.agent_id,
        prepared.organizer_userid,
        local_offset_seconds(),
    );
    let snapshots = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        wecom_calendar::load_wecom_snapshots(&conn).map_err(|e| e.to_string())?
    };
    let plans = wecom_calendar::compute_plans(
        &api,
        &prepared.calendar_id,
        local_offset_seconds(),
        snapshots,
    )
    .await
    .map_err(|err| map_wecom_auth_error(state, err.to_string()))?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    wecom_calendar::apply_plans(&conn, &plans).map_err(|e| e.to_string())
}

async fn resolve_wecom(
    state: &State<'_, AppState>,
    input: ResolveSyncConflictInput,
) -> Result<SyncRunResult, String> {
    let (prepared, snapshot) = {
        let prepared = prepare_wecom_access(state).await?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let snapshot = wecom_calendar::load_wecom_snapshots(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|item| item.task.id == input.task_id)
            .ok_or_else(|| "task is not opted in to WeCom Calendar".to_string())?;
        (prepared, snapshot)
    };
    let api = WecomCalendar::new(
        prepared.access_token,
        prepared.agent_id,
        prepared.organizer_userid,
        local_offset_seconds(),
    );
    let plan = wecom_calendar::plan_resolution(
        &api,
        &prepared.calendar_id,
        local_offset_seconds(),
        snapshot,
        &input.strategy,
    )
    .await
    .map_err(|err| map_wecom_auth_error(state, err.to_string()))?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    wecom_calendar::apply_plans(&conn, &[plan]).map_err(|e| e.to_string())
}

struct PreparedWecom {
    calendar_id: String,
    access_token: String,
    agent_id: Option<u64>,
    organizer_userid: String,
}

async fn prepare_wecom_access(state: &State<'_, AppState>) -> Result<PreparedWecom, String> {
    let (corp_id, secret, agent_id, organizer_userid, calendar_id, tokens) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        let account = wecom_account(&conn)?;
        let config = wecom_calendar::parse_public_config(account.config.as_deref());
        let secret = store.load_wecom_secret(&conn).map_err(|e| e.to_string())?;
        let tokens = store.load_wecom_tokens(&conn).map_err(|e| e.to_string())?;
        (
            config.corp_id,
            secret,
            wecom_calendar::parse_agent_id(&config.agent_id),
            config.organizer_userid,
            config.calendar_id,
            tokens,
        )
    };
    if corp_id.is_empty() {
        return Err("WeCom Calendar sync requires a CorpID before live sync.".to_string());
    }
    let Some(secret) = secret.filter(|value| !value.is_empty()) else {
        return Err("WeCom Calendar sync requires a corp Secret before live sync.".to_string());
    };
    if organizer_userid.is_empty() {
        return Err(
            "WeCom Calendar sync requires an organizer userid before live sync.".to_string(),
        );
    }
    if !wecom_calendar::concrete_calendar_id(&calendar_id) {
        return Err(
            "WeCom sync requires an app calendar. Connect WeCom to create one before live sync."
                .to_string(),
        );
    }
    let fresh = match tokens {
        Some(tokens) if tokens.expires_at > chrono::Utc::now().timestamp() + 60 => tokens,
        _ => match wecom_calendar::request_corp_token(&corp_id, &secret).await {
            Ok(tokens) => tokens,
            Err(err) => return Err(map_wecom_auth_error(state, err.to_string())),
        },
    };
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let store = TokenStore::for_app(&state.data_dir).map_err(|e| e.to_string())?;
        store
            .save_wecom_tokens(&conn, &fresh)
            .map_err(|e| e.to_string())?;
    }
    Ok(PreparedWecom {
        calendar_id,
        access_token: fresh.access_token,
        agent_id,
        organizer_userid,
    })
}

fn map_wecom_auth_error(state: &State<'_, AppState>, message: String) -> String {
    if message.contains("status 401") || message.contains("invalid_grant") {
        if let Ok(conn) = state.db.lock() {
            if let Ok(store) = TokenStore::for_app(&state.data_dir) {
                let _ = store.delete_wecom_tokens(&conn);
            }
            let _ = wecom_calendar::mark_needs_auth(
                &conn,
                "WeCom authorization expired. Connect WeCom Calendar again.",
            );
        }
        return "WeCom authorization expired. Connect WeCom Calendar again.".to_string();
    }
    message
}

fn wecom_account(conn: &rusqlite::Connection) -> Result<SyncAccount, String> {
    sync_module::list_accounts(conn)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|account| account.provider == wecom_calendar::WECOM_PROVIDER)
        .ok_or_else(|| "wecom account missing".to_string())
}
