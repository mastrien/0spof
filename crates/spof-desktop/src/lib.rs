pub mod commands;
pub mod dto;

pub use commands::AppState;
use spof_core::storage::Database;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn try_load_dotenv_session(state: &AppState) {
    let candidates = [Path::new(".env"), Path::new("../../.env")];
    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut base_url = None;
            let mut session_cookie = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((key, val)) = trimmed.split_once('=') {
                    let clean_val = val.trim().trim_matches('"').trim_matches('\'').to_string();
                    match key.trim() {
                        "MOODLE_BASE_URL" if !clean_val.is_empty() => base_url = Some(clean_val),
                        "MOODLE_SESSION_COOKIE" if !clean_val.is_empty() => {
                            session_cookie = Some(clean_val)
                        }
                        _ => {}
                    }
                }
            }
            if let (Some(url), Some(cookie)) = (base_url, session_cookie) {
                let _ = commands::handle_save_session(state, url, cookie);
                break;
            }
        }
    }
}

pub fn run() {
    #[cfg(target_os = "windows")]
    {
        if std::env::var("WEBVIEW2_USER_DATA_FOLDER").is_err() {
            if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
                let data_dir = PathBuf::from(local_appdata)
                    .join("0spof")
                    .join("webview_data");
                let _ = std::fs::create_dir_all(&data_dir);
                std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", data_dir);
            }
        }
    }

    let db_path = if let Ok(custom) = std::env::var("SPOF_DB_PATH") {
        PathBuf::from(custom)
    } else if Path::new("0spof.db").exists() {
        PathBuf::from("0spof.db")
    } else if Path::new("../../0spof.db").exists() {
        PathBuf::from("../../0spof.db")
    } else if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
        let app_dir = PathBuf::from(local_appdata).join("0spof");
        let _ = std::fs::create_dir_all(&app_dir);
        app_dir.join("0spof.db")
    } else {
        PathBuf::from("0spof.db")
    };

    let db =
        Database::open(&db_path).expect("Failed to initialize SQLite database for 0spof desktop");
    let state = AppState::new(db);
    try_load_dotenv_session(&state);
    let app_state = Arc::new(state);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_system_status,
            commands::get_courses,
            commands::get_outbox_items,
            commands::enqueue_submission,
            commands::get_submission_proof,
            commands::save_session,
            commands::get_session_info,
            commands::sync_moodle,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 0spof desktop application");
}
