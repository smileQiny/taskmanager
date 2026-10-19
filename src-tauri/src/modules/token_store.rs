use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const KEY_FILE: &str = "oauth.key";
const SERVICE: &str = "taskmanager";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
}

pub struct TokenStore {
    key: [u8; 32],
    prefer_keyring: bool,
}

impl TokenStore {
    pub fn for_app(data_dir: &Path) -> Result<Self> {
        Ok(Self {
            key: load_or_create_key(data_dir)?,
            prefer_keyring: true,
        })
    }

    #[cfg(test)]
    pub fn for_tests(key: [u8; 32]) -> Self {
        Self {
            key,
            prefer_keyring: false,
        }
    }

    pub fn save_tokens(&self, conn: &Connection, tokens: &GoogleTokens) -> Result<()> {
        self.save_json(conn, "google.tokens", tokens)
    }

    pub fn load_tokens(&self, conn: &Connection) -> Result<Option<GoogleTokens>> {
        self.load_json(conn, "google.tokens")
    }

    pub fn delete_tokens(&self, conn: &Connection) -> Result<()> {
        self.delete(conn, "google.tokens")
    }

    pub fn save_client_secret(&self, conn: &Connection, secret: &str) -> Result<()> {
        self.save_json(conn, "google.client_secret", &secret.to_string())
    }

    pub fn load_client_secret(&self, conn: &Connection) -> Result<Option<String>> {
        self.load_json(conn, "google.client_secret")
    }

    pub fn delete_client_secret(&self, conn: &Connection) -> Result<()> {
        self.delete(conn, "google.client_secret")
    }

    pub fn save_feishu_tokens(&self, conn: &Connection, tokens: &GoogleTokens) -> Result<()> {
        self.save_json(conn, "feishu.tokens", tokens)
    }

    pub fn load_feishu_tokens(&self, conn: &Connection) -> Result<Option<GoogleTokens>> {
        self.load_json(conn, "feishu.tokens")
    }

    pub fn delete_feishu_tokens(&self, conn: &Connection) -> Result<()> {
        self.delete(conn, "feishu.tokens")
    }

    pub fn save_feishu_app_secret(&self, conn: &Connection, secret: &str) -> Result<()> {
        self.save_json(conn, "feishu.app_secret", &secret.to_string())
    }

    pub fn load_feishu_app_secret(&self, conn: &Connection) -> Result<Option<String>> {
        self.load_json(conn, "feishu.app_secret")
    }

    pub fn delete_feishu_app_secret(&self, conn: &Connection) -> Result<()> {
        self.delete(conn, "feishu.app_secret")
    }

    fn save_json<T: Serialize>(&self, conn: &Connection, id: &str, value: &T) -> Result<()> {
        let plaintext = serde_json::to_string(value)?;
        if self.prefer_keyring && save_keyring(id, &plaintext).is_ok() {
            delete_row(conn, id)?;
            return Ok(());
        }
        let (nonce, ciphertext) = encrypt(&self.key, plaintext.as_bytes())?;
        conn.execute(
            "INSERT INTO oauth_secrets (id, nonce, ciphertext) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET nonce = excluded.nonce, ciphertext = excluded.ciphertext",
            params![id, nonce, ciphertext],
        )?;
        Ok(())
    }

    fn load_json<T: for<'de> Deserialize<'de>>(
        &self,
        conn: &Connection,
        id: &str,
    ) -> Result<Option<T>> {
        if self.prefer_keyring {
            if let Some(plaintext) = load_keyring(id)? {
                return Ok(Some(serde_json::from_str(&plaintext)?));
            }
        }
        let Some((nonce, ciphertext)) = load_row(conn, id)? else {
            return Ok(None);
        };
        let plaintext = decrypt(&self.key, &nonce, &ciphertext)?;
        Ok(Some(serde_json::from_slice(&plaintext)?))
    }

    fn delete(&self, conn: &Connection, id: &str) -> Result<()> {
        if self.prefer_keyring {
            let _ = delete_keyring(id);
        }
        delete_row(conn, id)
    }
}

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|err| anyhow!(err))?;
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).map_err(|err| anyhow!(err))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|err| anyhow!(err))?;
    Ok((nonce_bytes.to_vec(), ciphertext))
}

fn decrypt(key: &[u8; 32], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|err| anyhow!(err))?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|err| anyhow!("oauth secret could not be decrypted: {err}"))
}

fn load_or_create_key(data_dir: &Path) -> Result<[u8; 32]> {
    fs::create_dir_all(data_dir)?;
    let path = data_dir.join(KEY_FILE);
    if path.exists() {
        let bytes = fs::read(&path)?;
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).map_err(|err| anyhow!(err))?;
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    file.write_all(&key)?;
    Ok(key)
}

fn save_keyring(account: &str, secret: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, account).map_err(|err| anyhow!(err))?;
    entry.set_password(secret).map_err(|err| anyhow!(err))
}

fn load_keyring(account: &str) -> Result<Option<String>> {
    let entry = match keyring::Entry::new(SERVICE, account) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
    };
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(_) => Ok(None),
    }
}

fn delete_keyring(account: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, account).map_err(|err| anyhow!(err))?;
    entry.delete_credential().map_err(|err| anyhow!(err))
}

fn load_row(conn: &Connection, id: &str) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
    let mut stmt = conn.prepare("SELECT nonce, ciphertext FROM oauth_secrets WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some((row.get(0)?, row.get(1)?)))
}

fn delete_row(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM oauth_secrets WHERE id = ?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db;

    #[test]
    fn encrypted_tokens_round_trip_without_storing_plaintext() {
        let conn = db::init_in_memory().expect("database should initialize");
        let store = TokenStore::for_tests([7u8; 32]);
        let tokens = GoogleTokens {
            access_token: "access-secret".to_string(),
            refresh_token: Some("refresh-secret".to_string()),
            expires_at: 1_700_000_000,
        };

        store
            .save_tokens(&conn, &tokens)
            .expect("tokens should save");
        let stored: String = conn
            .query_row(
                "SELECT ciphertext FROM oauth_secrets WHERE id = 'google.tokens'",
                [],
                |row| {
                    let bytes: Vec<u8> = row.get(0)?;
                    Ok(String::from_utf8_lossy(&bytes).to_string())
                },
            )
            .expect("ciphertext should exist");
        assert!(!stored.contains("access-secret"));
        assert!(!stored.contains("refresh-secret"));

        let loaded = store.load_tokens(&conn).expect("tokens should load");
        assert_eq!(loaded, Some(tokens));

        store.delete_tokens(&conn).expect("tokens should delete");
        assert_eq!(store.load_tokens(&conn).expect("load after delete"), None);
    }

    #[test]
    fn feishu_app_secret_is_encrypted_and_not_returned_as_plaintext() {
        let conn = db::init_in_memory().expect("database should initialize");
        let store = TokenStore::for_tests([9u8; 32]);
        store
            .save_feishu_app_secret(&conn, "super-secret")
            .expect("secret should save");
        let stored: String = conn
            .query_row(
                "SELECT ciphertext FROM oauth_secrets WHERE id = 'feishu.app_secret'",
                [],
                |row| {
                    let bytes: Vec<u8> = row.get(0)?;
                    Ok(String::from_utf8_lossy(&bytes).to_string())
                },
            )
            .expect("ciphertext should exist");
        assert!(!stored.contains("super-secret"));
        assert_eq!(
            store
                .load_feishu_app_secret(&conn)
                .expect("secret should load")
                .as_deref(),
            Some("super-secret")
        );
        store
            .delete_feishu_app_secret(&conn)
            .expect("secret should delete");
        assert_eq!(
            store
                .load_feishu_app_secret(&conn)
                .expect("load after delete"),
            None
        );
    }
}
