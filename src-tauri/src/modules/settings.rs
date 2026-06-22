use anyhow::{bail, Result};
use rusqlite::{params, Connection};

use crate::models::settings::{AppSettings, UpdateSettingsInput};

const KEY_THEME: &str = "theme";
const KEY_DEFAULT_CALENDAR_VIEW: &str = "default_calendar_view";
const KEY_POMODORO_MINUTES: &str = "pomodoro_minutes";
const KEY_COCKPIT_OPACITY: &str = "cockpit_opacity";

pub fn get(conn: &Connection) -> Result<AppSettings> {
    Ok(AppSettings {
        theme: get_value(conn, KEY_THEME)?.unwrap_or_else(|| "light".to_string()),
        default_calendar_view: get_value(conn, KEY_DEFAULT_CALENDAR_VIEW)?
            .unwrap_or_else(|| "month".to_string()),
        pomodoro_minutes: get_value(conn, KEY_POMODORO_MINUTES)?
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(25),
        cockpit_opacity: get_value(conn, KEY_COCKPIT_OPACITY)?
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(92),
    })
}

pub fn update(conn: &Connection, input: UpdateSettingsInput) -> Result<AppSettings> {
    if let Some(theme) = input.theme {
        validate_theme(&theme)?;
        set_value(conn, KEY_THEME, &theme)?;
    }
    if let Some(view) = input.default_calendar_view {
        validate_calendar_view(&view)?;
        set_value(conn, KEY_DEFAULT_CALENDAR_VIEW, &view)?;
    }
    if let Some(minutes) = input.pomodoro_minutes {
        if !(5..=120).contains(&minutes) {
            bail!("pomodoro_minutes must be between 5 and 120");
        }
        set_value(conn, KEY_POMODORO_MINUTES, &minutes.to_string())?;
    }
    if let Some(opacity) = input.cockpit_opacity {
        if !(60..=100).contains(&opacity) {
            bail!("cockpit_opacity must be between 60 and 100");
        }
        set_value(conn, KEY_COCKPIT_OPACITY, &opacity.to_string())?;
    }
    get(conn)
}

fn get_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
    Ok(rows.next().transpose()?)
}

fn set_value(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn validate_theme(theme: &str) -> Result<()> {
    match theme {
        "light" | "dark" => Ok(()),
        _ => bail!("theme must be light or dark"),
    }
}

fn validate_calendar_view(view: &str) -> Result<()> {
    match view {
        "month" | "week" | "day" => Ok(()),
        _ => bail!("default_calendar_view must be month, week, or day"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::db;

    #[test]
    fn settings_have_defaults_and_can_update() {
        let conn = db::init_in_memory().expect("database should initialize");

        assert_eq!(
            get(&conn).expect("settings should load"),
            AppSettings {
                theme: "light".to_string(),
                default_calendar_view: "month".to_string(),
                pomodoro_minutes: 25,
                cockpit_opacity: 92,
            }
        );

        let updated = update(
            &conn,
            UpdateSettingsInput {
                theme: Some("dark".to_string()),
                default_calendar_view: Some("week".to_string()),
                pomodoro_minutes: Some(45),
                cockpit_opacity: Some(76),
            },
        )
        .expect("settings should update");

        assert_eq!(updated.theme, "dark");
        assert_eq!(updated.default_calendar_view, "week");
        assert_eq!(updated.pomodoro_minutes, 45);
        assert_eq!(updated.cockpit_opacity, 76);
    }

    #[test]
    fn settings_reject_invalid_duration() {
        let conn = db::init_in_memory().expect("database should initialize");
        let err = update(
            &conn,
            UpdateSettingsInput {
                theme: None,
                default_calendar_view: None,
                pomodoro_minutes: Some(1),
                cockpit_opacity: None,
            },
        )
        .expect_err("invalid duration should fail");

        assert!(err.to_string().contains("pomodoro_minutes"));
    }

    #[test]
    fn settings_reject_invalid_cockpit_opacity() {
        let conn = db::init_in_memory().expect("database should initialize");
        let err = update(
            &conn,
            UpdateSettingsInput {
                theme: None,
                default_calendar_view: None,
                pomodoro_minutes: None,
                cockpit_opacity: Some(20),
            },
        )
        .expect_err("invalid cockpit opacity should fail");

        assert!(err.to_string().contains("cockpit_opacity"));
    }
}
