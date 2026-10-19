use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::modules::google_oauth::{generate_pkce, generate_state};
use crate::modules::token_store::GoogleTokens;

pub const REDIRECT_PORT: u16 = 43821;
pub const REDIRECT_URI: &str = "http://127.0.0.1:43821/callback";
pub const FEISHU_SCOPES: &str = "offline_access calendar:calendar:readonly calendar:calendar.event:read calendar:calendar.event:create calendar:calendar.event:update";
pub const OAUTH_TIMEOUT: Duration = Duration::from_secs(180);

const FEISHU_AUTHORIZE: &str = "https://accounts.feishu.cn/open-apis/authen/v1/authorize";
const FEISHU_TOKEN: &str = "https://accounts.feishu.cn/oauth/v3/token";
const FEISHU_API: &str = "https://open.feishu.cn";
const LARK_AUTHORIZE: &str = "https://accounts.larksuite.com/open-apis/authen/v1/authorize";
const LARK_TOKEN: &str = "https://accounts.larksuite.com/oauth/v3/token";
const LARK_API: &str = "https://open.larksuite.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeishuEndpoints {
    pub authorize_url: &'static str,
    pub token_url: &'static str,
    pub api_base: &'static str,
}

#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub authorize_url: String,
    pub app_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
}

pub fn endpoints_for(domain: &str) -> FeishuEndpoints {
    if normalize_domain(domain) == "lark" {
        FeishuEndpoints {
            authorize_url: LARK_AUTHORIZE,
            token_url: LARK_TOKEN,
            api_base: LARK_API,
        }
    } else {
        FeishuEndpoints {
            authorize_url: FEISHU_AUTHORIZE,
            token_url: FEISHU_TOKEN,
            api_base: FEISHU_API,
        }
    }
}

pub fn normalize_domain(domain: &str) -> String {
    let value = domain.trim();
    if value.eq_ignore_ascii_case("lark") || value.eq_ignore_ascii_case("larksuite") {
        "lark".to_string()
    } else {
        "feishu".to_string()
    }
}

pub fn authorization_url(request: &AuthRequest) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        request.authorize_url,
        form_encode(&request.app_id),
        form_encode(&request.redirect_uri),
        form_encode(FEISHU_SCOPES),
        form_encode(&request.state),
        form_encode(&request.code_challenge),
    )
}

pub fn parse_oauth_callback(request_target: &str, expected_state: &str) -> Result<String> {
    let query = request_target
        .split_once('?')
        .map(|(_, query)| query.split_whitespace().next().unwrap_or(query))
        .unwrap_or("");
    let mut code = None;
    let mut state = None;
    let mut error = None;
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = form_decode(value);
        match key {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            _ => {}
        }
    }
    if let Some(error) = error {
        bail!("Feishu authorization was denied ({error})");
    }
    match state.as_deref() {
        Some(actual) if actual == expected_state => {}
        _ => bail!("OAuth state did not match"),
    }
    code.ok_or_else(|| anyhow!("OAuth callback did not include a code"))
}

pub fn merge_token_response(
    previous: Option<&GoogleTokens>,
    body: &str,
    now: i64,
) -> Result<GoogleTokens> {
    let parsed: Value = serde_json::from_str(body)?;
    let code = json_i64(&parsed, "code").unwrap_or(0);
    if code != 0 {
        let error = parsed
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let description = parsed
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if reauth_code(code) || error.contains("invalid_grant") {
            bail!("feishu token status {code}: invalid_grant {error} {description}");
        }
        bail!("feishu token status {code}: {error} {description}");
    }
    let access_token = parsed
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if access_token.is_empty() {
        bail!("Feishu token response did not include an access token");
    }
    let refresh_token = parsed
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| previous.and_then(|tokens| tokens.refresh_token.clone()));
    let expires_in = json_i64(&parsed, "expires_in").unwrap_or(7200).max(0);
    Ok(GoogleTokens {
        access_token,
        refresh_token,
        expires_at: now.saturating_add(expires_in),
    })
}

pub async fn exchange_authorization_code(
    token_url: &str,
    app_id: &str,
    app_secret: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<GoogleTokens> {
    let body = json!({
        "grant_type": "authorization_code",
        "client_id": app_id,
        "client_secret": app_secret,
        "code": code,
        "redirect_uri": redirect_uri,
        "code_verifier": verifier,
    });
    let response = post_token(token_url, &body).await?;
    merge_token_response(None, &response, chrono::Utc::now().timestamp())
}

pub async fn refresh_access_token(
    token_url: &str,
    app_id: &str,
    app_secret: &str,
    tokens: &GoogleTokens,
) -> Result<GoogleTokens> {
    let Some(refresh_token) = tokens.refresh_token.as_deref() else {
        bail!("feishu token status 20026: invalid_grant Feishu refresh token is missing");
    };
    let body = json!({
        "grant_type": "refresh_token",
        "client_id": app_id,
        "client_secret": app_secret,
        "refresh_token": refresh_token,
    });
    let response = post_token(token_url, &body).await?;
    merge_token_response(Some(tokens), &response, chrono::Utc::now().timestamp())
}

pub async fn interactive_authorization(
    domain: &str,
    app_id: &str,
    app_secret: &str,
    open_browser: bool,
) -> Result<GoogleTokens> {
    let endpoints = endpoints_for(domain);
    let listener = TcpListener::bind(("127.0.0.1", REDIRECT_PORT)).await.map_err(|_| {
        anyhow!(
            "Feishu OAuth needs {REDIRECT_URI}, but port {REDIRECT_PORT} is already in use. Close the other process and connect again."
        )
    })?;
    let pkce = generate_pkce()?;
    let state = generate_state()?;
    let url = authorization_url(&AuthRequest {
        authorize_url: endpoints.authorize_url.to_string(),
        app_id: app_id.to_string(),
        redirect_uri: REDIRECT_URI.to_string(),
        state: state.clone(),
        code_challenge: pkce.challenge.clone(),
    });
    let expected_state = state.clone();
    let accept = tokio::spawn(async move {
        tokio::time::timeout(
            OAUTH_TIMEOUT,
            accept_oauth_callback(listener, expected_state),
        )
        .await
    });
    if open_browser {
        if let Err(err) = open::that(&url) {
            accept.abort();
            bail!("failed to open the system browser: {err}");
        }
    }
    let code = accept
        .await
        .map_err(|err| anyhow!(err))?
        .map_err(|_| anyhow!("Feishu authorization timed out"))??;
    exchange_authorization_code(
        endpoints.token_url,
        app_id,
        app_secret,
        &code,
        &pkce.verifier,
        REDIRECT_URI,
    )
    .await
}

pub async fn accept_oauth_callback(
    listener: TcpListener,
    expected_state: String,
) -> Result<String> {
    let (mut stream, _) = listener.accept().await?;
    let mut buffer = vec![0u8; 8192];
    let read = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let result = parse_oauth_callback(target, &expected_state);
    let (status, body) = match &result {
        Ok(_) => (
            "200 OK",
            "<!doctype html><html><body><p>Task Manager received the Feishu authorization. You can close this tab.</p></body></html>".to_string(),
        ),
        Err(err) => (
            "400 Bad Request",
            format!("<!doctype html><html><body><p>Authorization failed: {err}</p></body></html>"),
        ),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    result
}

fn reauth_code(code: i64) -> bool {
    matches!(code, 20026 | 20037 | 20064 | 20073)
}

fn json_i64(value: &Value, key: &str) -> Option<i64> {
    match value.get(key)? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

async fn post_token(url: &str, body: &Value) -> Result<String> {
    let response = reqwest::Client::new().post(url).json(body).send().await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() && serde_json::from_str::<Value>(&text).is_err() {
        bail!("feishu token status {}: {text}", status.as_u16());
    }
    Ok(text)
}

fn form_encode(value: &str) -> String {
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

fn form_decode(value: &str) -> String {
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                } else {
                    out.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_url_uses_pkce_loopback_and_calendar_scopes() {
        let url = authorization_url(&AuthRequest {
            authorize_url: endpoints_for("feishu").authorize_url.to_string(),
            app_id: "cli_test".to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
            state: "state-1".to_string(),
            code_challenge: "challenge_value".to_string(),
        });

        assert!(url.starts_with("https://accounts.feishu.cn/open-apis/authen/v1/authorize?"));
        assert!(url.contains("client_id=cli_test"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A43821%2Fcallback"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=state-1"));
        assert!(url.contains("code_challenge=challenge_value"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("offline_access"));
        assert!(url.contains("calendar%3Acalendar.event%3Acreate"));
        assert!(url.contains("calendar%3Acalendar.event%3Aupdate"));
        assert!(!url.contains("app_secret"));
        assert!(!url.contains("client_secret"));
        assert_eq!(REDIRECT_URI, "http://127.0.0.1:43821/callback");
    }

    #[test]
    fn lark_domain_uses_larksuite_hosts() {
        let endpoints = endpoints_for("lark");
        assert!(endpoints.authorize_url.contains("accounts.larksuite.com"));
        assert!(endpoints
            .token_url
            .contains("accounts.larksuite.com/oauth/v3/token"));
        assert_eq!(endpoints.api_base, "https://open.larksuite.com");
        assert_eq!(endpoints_for("feishu").api_base, "https://open.feishu.cn");
    }

    #[test]
    fn callback_rejects_a_mismatched_state() {
        let err = parse_oauth_callback("/callback?code=abc&state=other", "expected").unwrap_err();
        assert!(err.to_string().contains("state"));
        let denied =
            parse_oauth_callback("/callback?error=access_denied&state=expected", "expected")
                .unwrap_err();
        assert!(denied.to_string().contains("denied"));
        let code = parse_oauth_callback("/callback?state=expected&code=abc", "expected").unwrap();
        assert_eq!(code, "abc");
    }

    #[test]
    fn token_response_keeps_rotated_refresh_token_and_flags_reauth() {
        let previous = GoogleTokens {
            access_token: "old".to_string(),
            refresh_token: Some("refresh-1".to_string()),
            expires_at: 10,
        };
        let merged = merge_token_response(
            Some(&previous),
            r#"{"code":0,"access_token":"new","expires_in":7200,"refresh_token":"refresh-2","token_type":"Bearer"}"#,
            100,
        )
        .unwrap();
        assert_eq!(merged.access_token, "new");
        assert_eq!(merged.refresh_token.as_deref(), Some("refresh-2"));
        assert_eq!(merged.expires_at, 7300);

        let kept = merge_token_response(
            Some(&previous),
            r#"{"code":0,"access_token":"newer","expires_in":"3600"}"#,
            100,
        )
        .unwrap();
        assert_eq!(kept.refresh_token.as_deref(), Some("refresh-1"));

        let expired = merge_token_response(
            Some(&previous),
            r#"{"code":20037,"error":"invalid_grant","error_description":"refresh token expired"}"#,
            100,
        )
        .unwrap_err();
        assert!(expired.to_string().contains("invalid_grant"));
    }

    #[tokio::test]
    async fn loopback_accepts_matching_state_only() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle =
            tokio::spawn(
                async move { accept_oauth_callback(listener, "good-state".to_string()).await },
            );
        let response = reqwest::Client::new()
            .get(format!(
                "http://127.0.0.1:{port}/callback?code=from-feishu&state=good-state"
            ))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let code = handle.await.unwrap().unwrap();
        assert_eq!(code, "from-feishu");
    }
}
