use crate::auth_window::handle_open_moodle_login_window;
use crate::commands::*;
use crate::dto::{
    CourseDto, EnqueueResultDto, OutboxItemDto, ProofDto, SectionDto, SessionInfoDto,
    SyncSummaryDto, SystemStatusDto,
};
use std::sync::Arc;

#[tauri::command]
pub fn get_system_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SystemStatusDto, String> {
    handle_get_system_status(&state.db)
}

#[tauri::command]
pub fn get_courses(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<CourseDto>, String> {
    handle_get_courses(&state.db)
}

#[tauri::command]
pub fn get_course_details(
    state: tauri::State<'_, Arc<AppState>>,
    course_id: i64,
) -> Result<Vec<SectionDto>, String> {
    handle_get_course_details(&state.db, course_id)
}

#[tauri::command]
pub fn get_outbox_items(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<OutboxItemDto>, String> {
    handle_get_outbox_items(&state.db)
}

#[tauri::command]
pub async fn enqueue_submission(
    state: tauri::State<'_, Arc<AppState>>,
    assign_id: i64,
    file_path: String,
    fetch_tsa: bool,
) -> Result<EnqueueResultDto, String> {
    handle_enqueue_submission(&state.db, assign_id, &file_path, fetch_tsa).await
}

#[tauri::command]
pub fn get_submission_proof(
    state: tauri::State<'_, Arc<AppState>>,
    outbox_id: i64,
) -> Result<ProofDto, String> {
    handle_get_submission_proof(&state.db, outbox_id)
}

#[tauri::command]
pub fn save_session(
    state: tauri::State<'_, Arc<AppState>>,
    base_url: String,
    session_cookie: String,
) -> Result<SessionInfoDto, String> {
    handle_save_session(&state, base_url, session_cookie)
}

#[tauri::command]
pub fn get_session_info(state: tauri::State<'_, Arc<AppState>>) -> SessionInfoDto {
    handle_get_session_info(&state)
}

#[tauri::command]
pub async fn sync_moodle(state: tauri::State<'_, Arc<AppState>>) -> Result<SyncSummaryDto, String> {
    handle_sync_moodle(&state).await
}

#[tauri::command]
pub async fn download_activity_media(
    state: tauri::State<'_, Arc<AppState>>,
    activity_id: i64,
    allow_video: bool,
) -> Result<String, String> {
    handle_download_activity_media(&state, activity_id, allow_video).await
}

#[tauri::command]
pub fn open_target(target: String) -> Result<(), String> {
    handle_open_target(&target)
}

#[tauri::command]
pub fn open_moodle_login_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    base_url: Option<String>,
) -> Result<(), String> {
    handle_open_moodle_login_window(app, (*state).clone(), base_url)
}
