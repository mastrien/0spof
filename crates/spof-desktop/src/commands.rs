use crate::dto::{
    CourseDto, EnqueueResultDto, OutboxItemDto, ProofDto, SessionInfoDto, SyncSummaryDto,
    SystemStatusDto,
};
use spof_core::auth::{MoodleClient, MoodleSession};
use spof_core::crawler::CourseCrawler;
use spof_core::outbox::OutboxEngine;
use spof_core::storage::Database;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub session: Arc<Mutex<Option<MoodleSession>>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            session: Arc::new(Mutex::new(None)),
        }
    }
}

// Logic handlers decoupled from Tauri macros for direct testing and IPC execution

pub fn handle_get_system_status(database: &Database) -> Result<SystemStatusDto, String> {
    let courses = database.get_courses().map_err(|e| e.to_string())?;
    let deadlines = database
        .get_upcoming_deadlines()
        .map_err(|e| e.to_string())?;
    let pending_items = database
        .get_pending_outbox_items()
        .map_err(|e| e.to_string())?;

    Ok(SystemStatusDto {
        courses_count: courses.len(),
        deadlines_count: deadlines.len(),
        outbox_count: pending_items.len(),
    })
}

pub fn handle_get_courses(database: &Database) -> Result<Vec<CourseDto>, String> {
    let courses = database.get_courses().map_err(|e| e.to_string())?;
    Ok(courses
        .into_iter()
        .map(|course| CourseDto {
            id: course.id,
            fullname: course.fullname,
            shortname: course.shortname,
        })
        .collect())
}

pub fn handle_get_outbox_items(database: &Database) -> Result<Vec<OutboxItemDto>, String> {
    let items = database
        .get_pending_outbox_items()
        .map_err(|e| e.to_string())?;
    Ok(items
        .into_iter()
        .map(|item| OutboxItemDto {
            id: item.id.unwrap_or(0),
            assign_id: item.assign_id,
            filename: item.filename,
            file_hash: item.file_hash,
            status: item.status.as_str().to_string(),
            retry_count: item.retry_count,
            has_rfc3161_token: item.rfc3161_token.is_some(),
        })
        .collect())
}

pub async fn handle_enqueue_submission(
    database: &Database,
    assign_id: i64,
    file_path: &str,
    fetch_tsa: bool,
) -> Result<EnqueueResultDto, String> {
    let path = Path::new(file_path);
    let engine = OutboxEngine::new(database.clone());
    let item = engine
        .enqueue_file(assign_id, path, fetch_tsa)
        .await
        .map_err(|e| e.to_string())?;

    Ok(EnqueueResultDto {
        outbox_id: item.id.unwrap_or(0),
        file_hash: item.file_hash,
        has_tsa_token: item.rfc3161_token.is_some(),
    })
}

pub fn handle_get_submission_proof(
    database: &Database,
    outbox_id: i64,
) -> Result<ProofDto, String> {
    let engine = OutboxEngine::new(database.clone());
    let proof = engine
        .get_submission_proof(outbox_id)
        .map_err(|e| e.to_string())?;

    let formatted_report = proof.formatted_report();

    Ok(ProofDto {
        assign_id: proof.assign_id,
        filename: proof.filename,
        file_hash: proof.file_hash,
        formatted_report,
    })
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
    let mut session_guard = state.session.lock().unwrap();
    *session_guard = Some(moodle_session);

    Ok(SessionInfoDto {
        is_authenticated: true,
        base_url: Some(trimmed_url.to_string()),
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
    let session_opt = {
        let guard = state.session.lock().unwrap();
        guard.clone()
    };

    let session = session_opt.ok_or_else(|| {
        "Nenhuma sessão configurada. Configure a conexão nas Configurações.".to_string()
    })?;

    let client = MoodleClient::new(session.base_url.clone())
        .map_err(|e| format!("Erro ao criar cliente Moodle: {}", e))?
        .with_session(session);

    let crawler = CourseCrawler::new(state.db.clone(), client);
    let sync_result = crawler
        .sync_all()
        .await
        .map_err(|e| format!("Falha na sincronização: {}", e))?;

    Ok(SyncSummaryDto {
        courses_synced: sync_result.courses_synced,
        sections_synced: sync_result.sections_synced,
        activities_synced: sync_result.activities_synced,
    })
}

// Tauri IPC Command Bindings

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

#[path = "commands_tests.rs"]
#[cfg(test)]
mod tests;
