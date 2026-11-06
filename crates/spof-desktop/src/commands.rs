use crate::dto::{
    ActivityDto, CourseDto, EnqueueResultDto, OutboxItemDto, ProofDto, SectionDto, SessionInfoDto,
    SyncSummaryDto, SystemStatusDto, UpcomingDeadlineDto,
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

pub fn handle_get_system_status(database: &Database) -> Result<SystemStatusDto, String> {
    let courses = database.get_courses().map_err(|e| e.to_string())?;
    let deadlines = database
        .get_upcoming_deadlines()
        .map_err(|e| e.to_string())?;
    let pending_items = database
        .get_pending_outbox_items()
        .map_err(|e| e.to_string())?;

    let muted_courses_count = courses.iter().filter(|c| c.is_muted).count();
    let active_courses_count = courses.len() - muted_courses_count;

    Ok(SystemStatusDto {
        courses_count: courses.len(),
        active_courses_count,
        muted_courses_count,
        deadlines_count: deadlines.len(),
        outbox_count: pending_items.len(),
    })
}

pub fn handle_get_upcoming_deadlines(
    database: &Database,
) -> Result<Vec<UpcomingDeadlineDto>, String> {
    let deadlines = database
        .get_upcoming_deadlines()
        .map_err(|e| e.to_string())?;
    Ok(deadlines
        .into_iter()
        .map(|act| UpcomingDeadlineDto {
            id: act.id,
            section_id: act.section_id,
            title: act.title,
            activity_type: act.activity_type.as_str().to_string(),
            url: act.url,
            due_date: act.due_date.map(|dt| dt.to_rfc3339()),
            is_submitted: act.is_submitted,
        })
        .collect())
}

pub fn handle_get_courses(database: &Database) -> Result<Vec<CourseDto>, String> {
    let courses = database.get_courses().map_err(|e| e.to_string())?;
    Ok(courses
        .into_iter()
        .map(|course| {
            let sections = database
                .get_sections_by_course(course.id)
                .unwrap_or_default();
            let activities_count = sections
                .iter()
                .map(|section| section.activities.len())
                .sum();
            CourseDto {
                id: course.id,
                fullname: course.fullname,
                shortname: course.shortname,
                sections_count: sections.len(),
                activities_count,
                is_muted: course.is_muted,
            }
        })
        .collect())
}

pub fn handle_set_course_muted(
    database: &Database,
    course_id: i64,
    is_muted: bool,
) -> Result<(), String> {
    database
        .set_course_muted(course_id, is_muted)
        .map_err(|e| e.to_string())
}

pub fn handle_get_course_details(
    database: &Database,
    course_id: i64,
) -> Result<Vec<SectionDto>, String> {
    let sections = database
        .get_sections_by_course(course_id)
        .map_err(|e| e.to_string())?;
    Ok(sections
        .into_iter()
        .map(|sec| SectionDto {
            id: sec.id,
            course_id: sec.course_id,
            name: sec.name,
            summary: sec.summary,
            activities: sec
                .activities
                .into_iter()
                .map(|act| ActivityDto {
                    id: act.id,
                    section_id: act.section_id,
                    title: act.title,
                    activity_type: act.activity_type.as_str().to_string(),
                    url: act.url,
                    description: act.description,
                    local_file_path: act.local_file_path,
                    content_mime: act.content_mime,
                    due_date: act.due_date.map(|dt| dt.to_rfc3339()),
                    is_submitted: act.is_submitted,
                })
                .collect(),
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
        resources_cached: sync_result.resources_cached,
    })
}

pub async fn handle_download_activity_media(
    state: &AppState,
    activity_id: i64,
    allow_video: bool,
) -> Result<String, String> {
    let session_opt = {
        let guard = state.session.lock().unwrap();
        guard.clone()
    };
    let session =
        session_opt.ok_or_else(|| "Sessão Moodle não configurada para download.".to_string())?;
    let client = MoodleClient::new(session.base_url.clone())
        .map_err(|e| e.to_string())?
        .with_session(session);
    let crawler = CourseCrawler::new(state.db.clone(), client);
    crawler
        .download_activity(activity_id, allow_video)
        .await
        .map_err(|e| e.to_string())
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
