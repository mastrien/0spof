use crate::dto::{
    CourseDto, EnqueueResultDto, OutboxItemDto, ProofDto, SectionDto, SessionInfoDto,
    SyncSummaryDto, SystemStatusDto, UpcomingDeadlineDto,
};
use spof_core::api;
use spof_core::auth::MoodleSession;
use spof_core::storage::Database;
use spof_core::sync::server::ServerHandle;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub session: Arc<Mutex<Option<MoodleSession>>>,
    pub sync_server: Arc<Mutex<Option<ServerHandle>>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            session: Arc::new(Mutex::new(None)),
            sync_server: Arc::new(Mutex::new(None)),
        }
    }
}

pub fn handle_get_system_status(database: &Database) -> Result<SystemStatusDto, String> {
    api::system_status(database)
}

pub fn handle_get_upcoming_deadlines(
    database: &Database,
) -> Result<Vec<UpcomingDeadlineDto>, String> {
    api::upcoming_deadlines(database)
}

pub fn handle_get_courses(database: &Database) -> Result<Vec<CourseDto>, String> {
    api::courses(database)
}

pub fn handle_set_course_muted(
    database: &Database,
    course_id: i64,
    is_muted: bool,
) -> Result<(), String> {
    api::set_course_muted(database, course_id, is_muted)
}

pub fn handle_get_course_details(
    database: &Database,
    course_id: i64,
) -> Result<Vec<SectionDto>, String> {
    api::course_details(database, course_id)
}

pub fn handle_get_last_sync_report(
    database: &Database,
) -> Result<Option<spof_core::api::dto::SyncReportDto>, String> {
    api::last_sync_report(database)
}

pub fn handle_get_storage_info(
    database: &Database,
) -> Result<spof_core::api::dto::StorageInfoDto, String> {
    api::storage_info(database)
}

pub fn handle_search(
    database: &Database,
    query: &str,
    limit: Option<usize>,
) -> Result<Vec<spof_core::api::dto::SearchHitDto>, String> {
    api::search(database, query, limit)
}

pub fn handle_file_info(path: &str) -> Result<spof_core::api::dto::FileInfoDto, String> {
    api::file_info(path)
}

pub fn handle_get_outbox_items(database: &Database) -> Result<Vec<OutboxItemDto>, String> {
    api::outbox_history(database)
}

pub async fn handle_enqueue_submission(
    database: &Database,
    assign_id: i64,
    file_path: &str,
    fetch_tsa: bool,
) -> Result<EnqueueResultDto, String> {
    api::enqueue_submission(database, assign_id, file_path, fetch_tsa).await
}

pub fn handle_get_submission_proof(
    database: &Database,
    outbox_id: i64,
) -> Result<ProofDto, String> {
    api::submission_proof(database, outbox_id)
}

pub fn handle_save_session(
    state: &AppState,
    base_url: String,
    session_cookie: String,
) -> Result<SessionInfoDto, String> {
    let trimmed_url = base_url.trim();
    let trimmed_cookie = session_cookie.trim();

    if trimmed_url.is_empty() || trimmed_cookie.is_empty() {
        return Err("URL base e Cookie de Sessão não podem ser vazios".to_string());
    }

    let moodle_session = MoodleSession::new(trimmed_url.to_string(), trimmed_cookie.to_string());
    let clean_base_url = moodle_session.base_url.clone();
    let mut session_guard = state.session.lock().unwrap();
    *session_guard = Some(moodle_session);

    Ok(SessionInfoDto {
        is_authenticated: true,
        base_url: Some(clean_base_url),
    })
}

pub fn handle_get_session_info(state: &AppState) -> SessionInfoDto {
    let session_guard = state.session.lock().unwrap();
    match session_guard.as_ref() {
        Some(session) => SessionInfoDto {
            is_authenticated: true,
            base_url: Some(session.base_url.clone()),
        },
        None => SessionInfoDto {
            is_authenticated: false,
            base_url: None,
        },
    }
}

pub async fn handle_sync_moodle(state: &AppState) -> Result<SyncSummaryDto, String> {
    let session = current_session(state).ok_or_else(|| {
        "Nenhuma sessão configurada. Configure a conexão nas Configurações.".to_string()
    })?;
    api::sync_moodle(&state.db, session, None).await
}

pub async fn handle_download_activity_media(
    state: &AppState,
    activity_id: i64,
    allow_video: bool,
) -> Result<String, String> {
    let session = current_session(state)
        .ok_or_else(|| "Sessão Moodle não configurada para download.".to_string())?;
    api::download_activity_media(&state.db, session, activity_id, allow_video, None).await
}

fn current_session(state: &AppState) -> Option<MoodleSession> {
    state.session.lock().unwrap().clone()
}

pub fn handle_open_target(target: &str) -> Result<(), String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err("Caminho ou URL vazio".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", trimmed])
            .spawn()
            .map_err(|e| format!("Erro ao abrir destino: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Erro ao abrir destino: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Erro ao abrir destino: {}", e))?;
    }
    Ok(())
}

#[path = "commands_tests.rs"]
#[cfg(test)]
mod tests;
