use tauri::State;

use crate::models::sync::{
    GoogleConnection, ResolveSyncConflictInput, SaveGoogleSettingsInput, SetTaskSyncInput,
    SyncAccount, SyncRunResult, TaskSyncState, UpsertSyncAccountInput,
};
use crate::modules::google_calendar::{self, HttpCalendar};
use crate::modules::google_oauth::{interactive_authorization, refresh_access_token};
use crate::modules::sync as sync_module;
use crate::modules::token_store::{GoogleTokens, TokenStore};
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
    sync_module::upsert_account(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_provider_now(
    state: State<'_, AppState>,
    provider: String,
) -> Result<SyncRunResult, String> {
    if provider != google_calendar::GOOGLE_PROVIDER {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        return sync_module::sync_now(&conn, provider).map_err(|e| e.to_string());
    }
    sync_google(&state).await
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
