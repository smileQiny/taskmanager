mod commands;
mod models;
mod modules;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_MENU_SHOW_MAIN: &str = "show-main";
const TRAY_MENU_QUIT: &str = "quit";

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("taskmanager.db");
            let conn = modules::db::init(&db_path).expect("Failed to initialize database");
            app.manage(AppState {
                db: Mutex::new(conn),
                data_dir: app_dir,
                db_path,
            });
            create_tray_icon(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_metadata,
            commands::task::get_tasks,
            commands::task::get_task,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::delete_task,
            commands::task::get_tasks_by_range,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::pomodoro::create_pomodoro_session,
            commands::pomodoro::list_pomodoro_sessions,
            commands::sync::list_sync_accounts,
            commands::sync::upsert_sync_account,
            commands::sync::sync_provider_now,
            commands::sync::list_task_sync_states,
            commands::sync::set_task_sync,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == MAIN_WINDOW_LABEL => {
            api.prevent_close();
            if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                let _ = window.hide();
            }
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            let _ = show_main_window(app_handle);
        }
        _ => {}
    });
}

fn create_tray_icon(app: &tauri::App) -> tauri::Result<()> {
    let show_main = MenuItem::with_id(app, TRAY_MENU_SHOW_MAIN, "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_MENU_QUIT, "退出 Task Manager", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_main, &quit])?;
    let mut tray_builder = TrayIconBuilder::with_id("taskmanager-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Task Manager")
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            TRAY_MENU_SHOW_MAIN => {
                let _ = show_main_window(app_handle);
            }
            TRAY_MENU_QUIT => app_handle.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if is_primary_tray_click(&event) {
                let _ = show_main_window(tray.app_handle());
            }
        });

    #[cfg(target_os = "macos")]
    {
        tray_builder = tray_builder
            .icon(build_macos_tray_icon())
            .icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(icon) = app.default_window_icon().cloned() {
            tray_builder = tray_builder.icon(icon);
        }
    }

    tray_builder.build(app)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn build_macos_tray_icon() -> Image<'static> {
    const SIZE: u32 = 18;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let point = (x as f32 + 0.5, y as f32 + 0.5);
            let dx = point.0 - 9.0;
            let dy = point.1 - 9.0;
            let radius = (dx * dx + dy * dy).sqrt();
            let circle = (6.25..=7.7).contains(&radius);
            let check_left = distance_to_segment(point, (5.2, 9.1), (7.7, 11.6)) <= 1.1;
            let check_right = distance_to_segment(point, (7.7, 11.6), (13.1, 6.1)) <= 1.1;
            let alpha = if circle || check_left || check_right {
                255
            } else {
                0
            };
            rgba.extend_from_slice(&[0, 0, 0, alpha]);
        }
    }

    Image::new_owned(rgba, SIZE, SIZE)
}

#[cfg(target_os = "macos")]
fn distance_to_segment(point: (f32, f32), start: (f32, f32), end: (f32, f32)) -> f32 {
    let vx = end.0 - start.0;
    let vy = end.1 - start.1;
    let wx = point.0 - start.0;
    let wy = point.1 - start.1;
    let length_squared = vx * vx + vy * vy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        ((wx * vx + wy * vy) / length_squared).clamp(0.0, 1.0)
    };
    let projection = (start.0 + t * vx, start.1 + t * vy);
    let dx = point.0 - projection.0;
    let dy = point.1 - projection.1;
    (dx * dx + dy * dy).sqrt()
}

fn is_primary_tray_click(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}
