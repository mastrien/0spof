//! API de alto nível compartilhada pelo desktop (Tauri) e pelo celular (Flutter).
//!
//! Devolve DTOs serializáveis e erros em `String`, para que cada interface apenas os apresente.
//! Nada aqui imprime ou abre janelas.

pub mod dto;

use crate::auth::{MoodleClient, MoodleSession};
use crate::crawler::CourseCrawler;
use crate::outbox::OutboxEngine;
use crate::storage::Database;
use dto::{
    ActivityDto, CourseDto, EnqueueResultDto, OutboxItemDto, ProofDto, SectionDto, SyncSummaryDto,
    SystemStatusDto, UpcomingDeadlineDto,
};
use std::path::Path;

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

pub fn system_status(database: &Database) -> Result<SystemStatusDto, String> {
    let courses = database.get_courses().map_err(err)?;
    let deadlines = database.get_upcoming_deadlines().map_err(err)?;
    let pending_items = database.get_pending_outbox_items().map_err(err)?;

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

pub fn upcoming_deadlines(database: &Database) -> Result<Vec<UpcomingDeadlineDto>, String> {
    let deadlines = database.get_upcoming_deadlines().map_err(err)?;
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

pub fn courses(database: &Database) -> Result<Vec<CourseDto>, String> {
    let courses = database.get_courses().map_err(err)?;
    Ok(courses
        .into_iter()
        .map(|course| {
            let sections = database
                .get_sections_by_course(course.id)
                .unwrap_or_default();
            let activities_count = sections.iter().map(|s| s.activities.len()).sum();
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

pub fn set_course_muted(
    database: &Database,
    course_id: i64,
    is_muted: bool,
) -> Result<(), String> {
    database.set_course_muted(course_id, is_muted).map_err(err)
}

pub fn course_details(database: &Database, course_id: i64) -> Result<Vec<SectionDto>, String> {
    let sections = database.get_sections_by_course(course_id).map_err(err)?;
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

pub fn outbox_items(database: &Database) -> Result<Vec<OutboxItemDto>, String> {
    let items = database.get_pending_outbox_items().map_err(err)?;
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

pub async fn enqueue_submission(
    database: &Database,
    assign_id: i64,
    file_path: &str,
    fetch_tsa: bool,
) -> Result<EnqueueResultDto, String> {
    let engine = OutboxEngine::new(database.clone());
    let item = engine
        .enqueue_file(assign_id, Path::new(file_path), fetch_tsa)
        .await
        .map_err(err)?;

    Ok(EnqueueResultDto {
        outbox_id: item.id.unwrap_or(0),
        file_hash: item.file_hash,
        has_tsa_token: item.rfc3161_token.is_some(),
    })
}

pub fn submission_proof(database: &Database, outbox_id: i64) -> Result<ProofDto, String> {
    let engine = OutboxEngine::new(database.clone());
    let proof = engine.get_submission_proof(outbox_id).map_err(err)?;
    let formatted_report = proof.formatted_report();
    Ok(ProofDto {
        assign_id: proof.assign_id,
        filename: proof.filename,
        file_hash: proof.file_hash,
        formatted_report,
    })
}

fn crawler_for(database: &Database, session: MoodleSession) -> Result<CourseCrawler, String> {
    let client = MoodleClient::new(session.base_url.clone())
        .map_err(|e| format!("Erro ao criar cliente Moodle: {e}"))?
        .with_session(session);
    Ok(CourseCrawler::new(database.clone(), client))
}

pub async fn sync_moodle(
    database: &Database,
    session: MoodleSession,
) -> Result<SyncSummaryDto, String> {
    let result = crawler_for(database, session)?
        .sync_all()
        .await
        .map_err(|e| format!("Falha na sincronização: {e}"))?;
    Ok(SyncSummaryDto {
        courses_synced: result.courses_synced,
        sections_synced: result.sections_synced,
        activities_synced: result.activities_synced,
        resources_cached: result.resources_cached,
    })
}

pub async fn download_activity_media(
    database: &Database,
    session: MoodleSession,
    activity_id: i64,
    allow_video: bool,
) -> Result<String, String> {
    crawler_for(database, session)?
        .download_activity(activity_id, allow_video)
        .await
        .map_err(err)
}

pub fn wipe_all_data(database: &Database) -> Result<(), String> {
    database.wipe_all_data().map_err(err)
}

pub fn export_data(database: &Database) -> Result<String, String> {
    database.export_data_json().map_err(err)
}

pub fn import_data(database: &Database, json: &str) -> Result<dto::ImportSummaryDto, String> {
    let s = database.import_data_json(json).map_err(err)?;
    Ok(dto::ImportSummaryDto {
        courses_imported: s.courses_imported,
        sections_imported: s.sections_imported,
        activities_imported: s.activities_imported,
        outbox_items_imported: s.outbox_items_imported,
    })
}
