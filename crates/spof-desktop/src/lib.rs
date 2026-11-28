pub mod auth_window;
pub mod commands;
pub use spof_core::api::dto;
pub mod data_commands;

pub mod ipc;
pub mod sync_commands;

pub use commands::AppState;
use spof_core::storage::Database;
use std::path::Path;
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
                let data_dir = std::path::PathBuf::from(local_appdata)
                    .join("0spof")
                    .join("webview_data");
                let _ = std::fs::create_dir_all(&data_dir);
                std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", data_dir);
            }
        }
    }

    let db_path = spof_core::paths::resolve_db_path_and_prepare();

    let db =
        Database::open(&db_path).expect("Failed to initialize SQLite database for 0spof desktop");
    let state = AppState::new(db);
    try_load_dotenv_session(&state);
    let _ = sync_commands::handle_start_sync_server(&state);
    let app_state = Arc::new(state);

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            ipc::get_system_status,
            ipc::get_upcoming_deadlines,
            ipc::get_courses,
            ipc::set_course_muted,
            ipc::get_course_details,
            ipc::get_outbox_items,
            ipc::enqueue_submission,
            ipc::get_submission_proof,
            ipc::save_session,
            ipc::get_session_info,
            ipc::sync_moodle,
            ipc::download_activity_media,
            ipc::open_target,
            ipc::open_moodle_login_window,
            ipc::logout,
            ipc::wipe_all_data,
            ipc::export_data,
            ipc::import_data,
            ipc::get_course_export_preview,
            ipc::export_course_package,
            ipc::import_package,
            ipc::get_app_version,
            ipc::get_last_sync_report,
            ipc::get_storage_info,
            ipc::search_local,
            ipc::file_info,
            ipc::check_for_updates,
            ipc::install_update,
            ipc::get_sync_info,
            ipc::start_sync_server,
            ipc::stop_sync_server,
            ipc::rotate_sync_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 0spof desktop application");
}
