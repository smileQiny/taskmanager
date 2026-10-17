use anyhow::{anyhow, bail, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::modules::token_store::GoogleTokens;

pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
pub const OAUTH_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
}

pub fn generate_pkce() -> Result<PkcePair> {
    let verifier = random_token(32)?;
    Ok(PkcePair {
        challenge: pkce_challenge(&verifier),
        verifier,
    })
}

pub fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn generate_state() -> Result<String> {
    random_token(16)
}

pub fn authorization_url(request: &AuthRequest) -> String {
    format!(
        "{GOOGLE_AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        form_encode(&request.client_id),
        form_encode(&request.redirect_uri),
        form_encode(CALENDAR_SCOPE),
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
        bail!("Google authorization was denied ({error})");
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
    let parsed: TokenResponse = serde_json::from_str(body)?;
    if parsed.access_token.is_empty() {
        bail!("Google token response did not include an access token");
    }
    let refresh_token = parsed
        .refresh_token
        .or_else(|| previous.and_then(|tokens| tokens.refresh_token.clone()));
    let expires_in = parsed.expires_in.unwrap_or(3600).max(0);
    Ok(GoogleTokens {
        access_token: parsed.access_token,
        refresh_token,
        expires_at: now.saturating_add(expires_in),
    })
}

pub async fn exchange_authorization_code(
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<GoogleTokens> {
    let mut form = vec![
        ("client_id", client_id.to_string()),
        ("code", code.to_string()),
        ("code_verifier", verifier.to_string()),
        ("grant_type", "authorization_code".to_string()),
        ("redirect_uri", redirect_uri.to_string()),
    ];
    if let Some(secret) = client_secret.filter(|secret| !secret.is_empty()) {
        form.push(("client_secret", secret.to_string()));
    }
    let body = post_form(GOOGLE_TOKEN_URL, &form).await?;
    merge_token_response(None, &body, chrono::Utc::now().timestamp())
}

pub async fn refresh_access_token(
    client_id: &str,
    client_secret: Option<&str>,
    tokens: &GoogleTokens,
) -> Result<GoogleTokens> {
    let Some(refresh_token) = tokens.refresh_token.as_deref() else {
        bail!("Google refresh token is missing");
    };
    let mut form = vec![
        ("client_id", client_id.to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("grant_type", "refresh_token".to_string()),
    ];
    if let Some(secret) = client_secret.filter(|secret| !secret.is_empty()) {
        form.push(("client_secret", secret.to_string()));
    }
    let body = post_form(GOOGLE_TOKEN_URL, &form).await?;
    merge_token_response(Some(tokens), &body, chrono::Utc::now().timestamp())
}

pub async fn interactive_authorization(
    client_id: &str,
    client_secret: Option<&str>,
    open_browser: bool,
) -> Result<GoogleTokens> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let pkce = generate_pkce()?;
    let state = generate_state()?;
    let url = authorization_url(&AuthRequest {
        client_id: client_id.to_string(),
        redirect_uri: redirect_uri.clone(),
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
        .map_err(|_| anyhow!("Google authorization timed out"))??;
    exchange_authorization_code(
        client_id,
        client_secret,
        &code,
        &pkce.verifier,
        &redirect_uri,
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
            "<!doctype html><html><body><p>Task Manager received the Google authorization. You can close this tab.</p></body></html>".to_string(),
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

async fn post_form(url: &str, form: &[(impl AsRef<str>, String)]) -> Result<String> {
    let pairs: Vec<(String, String)> = form
        .iter()
        .map(|(key, value)| (key.as_ref().to_string(), value.clone()))
        .collect();
    let response = reqwest::Client::new().post(url).form(&pairs).send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        if body.contains("invalid_grant") {
            bail!("google token status {}: invalid_grant", status.as_u16());
        }
        bail!("google token status {}: {body}", status.as_u16());
    }
    Ok(body)
}

fn random_token(byte_len: usize) -> Result<String> {
    let mut bytes = vec![0u8; byte_len];
    getrandom::getrandom(&mut bytes).map_err(|err| anyhow!(err))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
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

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_the_rfc7636_vector() {
        let challenge = pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn authorization_url_includes_pkce_state_and_calendar_scope() {
        let url = authorization_url(&AuthRequest {
            client_id: "client-id".to_string(),
            redirect_uri: "http://127.0.0.1:4317/callback".to_string(),
            state: "state-1".to_string(),
            code_challenge: "challenge_value".to_string(),
        });

        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A4317%2Fcallback"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=state-1"));
        assert!(url.contains("code_challenge=challenge_value"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.events"));
        assert!(!url.contains("client_secret"));
    }

    #[test]
    fn callback_rejects_a_mismatched_state() {
        let err = parse_oauth_callback("/callback?code=abc&state=other", "expected").unwrap_err();
        assert!(err.to_string().contains("state"));
        let code = parse_oauth_callback("/callback?state=expected&code=abc", "expected").unwrap();
        assert_eq!(code, "abc");
    }

    #[test]
    fn token_response_keeps_existing_refresh_token_when_refresh_omits_it() {
        let previous = GoogleTokens {
            access_token: "old".to_string(),
            refresh_token: Some("refresh-1".to_string()),
            expires_at: 10,
        };
        let merged = merge_token_response(
            Some(&previous),
            r#"{"access_token":"new","expires_in":3600,"token_type":"Bearer"}"#,
            100,
        )
        .unwrap();
        assert_eq!(merged.access_token, "new");
        assert_eq!(merged.refresh_token.as_deref(), Some("refresh-1"));
        assert_eq!(merged.expires_at, 3700);
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
                "http://127.0.0.1:{port}/callback?code=from-google&state=good-state"
            ))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let code = handle.await.unwrap().unwrap();
        assert_eq!(code, "from-google");
    }
}
