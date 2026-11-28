//! API de alto nível compartilhada pelo desktop (Tauri) e pelo celular (Flutter).
//!
//! Devolve DTOs serializáveis e erros em `String`, para que cada interface apenas os apresente.
//! Nada aqui imprime ou abre janelas.

pub mod dto;
pub mod search;
pub mod sync_report;

use crate::auth::{MoodleClient, MoodleSession};
use crate::crawler::{default_cache_dir, CourseCrawler};
use crate::outbox::{OutboxEngine, SubmissionProof};
use crate::storage::{
    is_material_activity, is_pdf_activity, suggested_course_export_filename, CourseExportSelection,
    Database, ImportSummary,
};
use dto::{
    ActivityDto, CourseDto, CourseExportPreviewDto, EnqueueResultDto, ExportItemPreviewDto,
    ExportSectionPreviewDto, ImportSummaryDto, OutboxItemDto, ProofDto, SectionDto, SyncSummaryDto,
    SystemStatusDto, UpcomingDeadlineDto,
};
use std::path::{Path, PathBuf};

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
    let now = chrono::Utc::now();
    Ok(courses
        .into_iter()
        .map(|course| {
            let sections = database
                .get_sections_by_course(course.id)
                .unwrap_or_default();
            let activities = sections.iter().flat_map(|s| s.activities.iter());
            let activities_count = activities.clone().count();
            let pending: Vec<_> = activities
                .filter(|a| !a.is_submitted)
                .filter_map(|a| a.due_date)
                .filter(|due| *due >= now)
                .collect();
            CourseDto {
                id: course.id,
                fullname: course.fullname,
                shortname: course.shortname,
                sections_count: sections.len(),
                activities_count,
                is_muted: course.is_muted,
                pending_deadlines: pending.len(),
                next_deadline: pending.iter().min().map(|d| d.to_rfc3339()),
                last_synced_at: course.last_synced_at.map(|d| d.to_rfc3339()),
            }
        })
        .collect())
}

pub fn set_course_muted(database: &Database, course_id: i64, is_muted: bool) -> Result<(), String> {
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

fn outbox_item_dtos(
    database: &Database,
    items: Vec<crate::models::OutboxItem>,
) -> Result<Vec<OutboxItemDto>, String> {
    // um único SELECT para todos os prazos (em vez de um por item); erro de leitura é
    // propagado em vez de esconder a folga em silêncio
    let ids: Vec<i64> = items.iter().map(|i| i.assign_id).collect();
    let dues = database.get_activity_due_dates(&ids).map_err(err)?;
    Ok(items
        .into_iter()
        .map(|item| {
            let due = dues.get(&item.assign_id).copied();
            let proof = SubmissionProof::from_outbox_item(&item).with_due_date(due);
            OutboxItemDto {
                id: item.id.unwrap_or(0),
                assign_id: item.assign_id,
                filename: item.filename,
                file_hash: item.file_hash,
                status: item.status.as_str().to_string(),
                retry_count: item.retry_count,
                has_rfc3161_token: item.rfc3161_token.is_some(),
                created_at: item.created_at.to_rfc3339(),
                updated_at: item.updated_at.to_rfc3339(),
                last_attempt_at: item.last_attempt_at.map(|t| t.to_rfc3339()),
                last_error: item.last_error,
                timestamped_at: proof.timestamped_at.map(|t| t.to_rfc3339()),
                due_date: due.map(|d| d.to_rfc3339()),
                margin_seconds: proof.margin_seconds(),
            }
        })
        .collect())
}

/// Itens ainda a enviar (pendentes ou com falha). Usado pelo celular e pelos contadores.
pub fn outbox_items(database: &Database) -> Result<Vec<OutboxItemDto>, String> {
    let items = database.get_pending_outbox_items().map_err(err)?;
    outbox_item_dtos(database, items)
}

/// Fila completa, inclusive itens já entregues (linha do tempo do desktop).
pub fn outbox_history(database: &Database) -> Result<Vec<OutboxItemDto>, String> {
    let items = database.get_all_outbox_items().map_err(err)?;
    outbox_item_dtos(database, items)
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

/// `cache_dir`: onde gravar os materiais baixados; `None` usa o padrão do desktop.
fn crawler_for(
    database: &Database,
    session: MoodleSession,
    cache_dir: Option<PathBuf>,
) -> Result<CourseCrawler, String> {
    let client = MoodleClient::new(session.base_url.clone())
        .map_err(|e| format!("Erro ao criar cliente Moodle: {e}"))?
        .with_session(session);
    let crawler = CourseCrawler::new(database.clone(), client);
    Ok(match cache_dir {
        Some(dir) => crawler.with_cache_dir(dir),
        None => crawler,
    })
}

pub async fn sync_moodle(
    database: &Database,
    session: MoodleSession,
    cache_dir: Option<PathBuf>,
) -> Result<SyncSummaryDto, String> {
    let before = sync_report::snapshot(database).ok();
    let result = crawler_for(database, session, cache_dir)?
        .sync_all()
        .await
        .map_err(|e| format!("Falha na sincronização: {e}"))?;
    let summary = SyncSummaryDto {
        courses_synced: result.courses_synced,
        sections_synced: result.sections_synced,
        activities_synced: result.activities_synced,
        resources_cached: result.resources_cached,
        report_save_error: None,
    };
    let after = sync_report::snapshot(database).ok();
    // a falha ao gravar o resumo não derruba a sync, mas volta em `report_save_error`
    Ok(sync_report::finish_sync(
        database,
        summary,
        before.as_ref(),
        after.as_ref(),
    ))
}

/// Confere que o caminho é um arquivo legível e devolve nome e tamanho (para a tela de entrega).
pub fn file_info(path: &str) -> Result<dto::FileInfoDto, String> {
    let p = Path::new(path.trim());
    let meta = std::fs::metadata(p).map_err(|_| format!("Arquivo não encontrado: {path}"))?;
    if !meta.is_file() {
        return Err(format!("O caminho não é um arquivo: {path}"));
    }
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Nome de arquivo inválido".to_string())?
        .to_string();
    Ok(dto::FileInfoDto {
        path: p.to_string_lossy().to_string(),
        name,
        size_bytes: meta.len(),
    })
}

/// Busca global em disciplinas, atividades e recursos.
pub fn search(
    database: &Database,
    query: &str,
    limit: Option<usize>,
) -> Result<Vec<dto::SearchHitDto>, String> {
    search::search(database, query, limit)
}

/// Novidades da última sincronização (`None` se este aparelho ainda não sincronizou).
pub fn last_sync_report(database: &Database) -> Result<Option<dto::SyncReportDto>, String> {
    sync_report::last_sync_report(database)
}

/// Espaço usado pelo banco e pelos arquivos baixados.
pub fn storage_info(database: &Database) -> Result<dto::StorageInfoDto, String> {
    sync_report::storage_info(database)
}

pub async fn download_activity_media(
    database: &Database,
    session: MoodleSession,
    activity_id: i64,
    allow_video: bool,
    cache_dir: Option<PathBuf>,
) -> Result<String, String> {
    crawler_for(database, session, cache_dir)?
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

fn import_summary_dto(s: ImportSummary) -> ImportSummaryDto {
    ImportSummaryDto {
        courses_imported: s.courses_imported,
        sections_imported: s.sections_imported,
        activities_imported: s.activities_imported,
        outbox_items_imported: s.outbox_items_imported,
        files_imported: s.files_imported,
    }
}

pub fn import_data(database: &Database, json: &str) -> Result<ImportSummaryDto, String> {
    let s = database.import_data_json(json).map_err(err)?;
    Ok(import_summary_dto(s))
}

pub fn course_export_preview(
    database: &Database,
    course_id: i64,
) -> Result<CourseExportPreviewDto, String> {
    let course = database
        .get_course(course_id)
        .map_err(err)?
        .ok_or_else(|| format!("Disciplina #{course_id} não encontrada"))?;
    let sections = database.get_sections_by_course(course_id).map_err(err)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let suggested_filename = suggested_course_export_filename(&course, &today);

    let preview_sections = sections
        .into_iter()
        .filter(|sec| {
            !sec.activities.is_empty()
                || sec.summary.as_deref().is_some_and(|s| !s.trim().is_empty())
        })
        .map(|sec| {
            let items = sec
                .activities
                .into_iter()
                .map(|act| {
                    let is_material = is_material_activity(&act);
                    let is_pdf = is_pdf_activity(&act);
                    let file_meta = act.local_file_path.as_deref().and_then(|p| {
                        let path = Path::new(p);
                        std::fs::metadata(path)
                            .ok()
                            .filter(|m| m.is_file())
                            .map(|m| {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .map(str::to_string);
                                (name, m.len())
                            })
                    });

                    ExportItemPreviewDto {
                        id: act.id,
                        section_id: act.section_id,
                        title: act.title,
                        activity_type: act.activity_type.as_str().to_string(),
                        is_material,
                        selected_by_default: is_material,
                        is_pdf,
                        has_local_file: file_meta.is_some(),
                        file_name: file_meta.as_ref().and_then(|(n, _)| n.clone()),
                        file_size_bytes: file_meta.map(|(_, len)| len),
                        content_mime: act.content_mime,
                        url: act.url,
                        due_date: act.due_date.map(|dt| dt.to_rfc3339()),
                    }
                })
                .collect();

            ExportSectionPreviewDto {
                id: sec.id,
                name: sec.name,
                summary: sec.summary,
                items,
            }
        })
        .collect();

    Ok(CourseExportPreviewDto {
        course_id: course.id,
        fullname: course.fullname,
        shortname: course.shortname,
        suggested_filename,
        sections: preview_sections,
    })
}

pub fn export_course_package(
    database: &Database,
    selection: &CourseExportSelection,
) -> Result<Vec<u8>, String> {
    database.export_course_package(selection).map_err(err)
}

pub fn import_package_bytes(
    database: &Database,
    bytes: &[u8],
    cache_dir: Option<PathBuf>,
) -> Result<ImportSummaryDto, String> {
    let target_dir = cache_dir.unwrap_or_else(default_cache_dir);
    let summary = database
        .import_package_bytes(bytes, &target_dir)
        .map_err(err)?;
    Ok(import_summary_dto(summary))
}

pub fn import_package_file(
    database: &Database,
    path: &Path,
    cache_dir: Option<PathBuf>,
) -> Result<ImportSummaryDto, String> {
    let target_dir = cache_dir.unwrap_or_else(default_cache_dir);
    let summary = database
        .import_package_file(path, &target_dir)
        .map_err(err)?;
    Ok(import_summary_dto(summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Activity, ActivityType, Course, Section};
    use chrono::{Duration, Utc};

    fn activity(id: i64, due: Option<chrono::DateTime<Utc>>, submitted: bool) -> Activity {
        Activity {
            id,
            section_id: 10,
            title: format!("A{id}"),
            activity_type: ActivityType::Assign,
            url: None,
            description: None,
            local_file_path: None,
            content_mime: None,
            due_date: due,
            is_submitted: submitted,
        }
    }

    #[test]
    fn courses_report_pending_deadlines_and_next_one() {
        let db = Database::open_in_memory().unwrap();
        let synced = Utc::now() - Duration::hours(2);
        db.upsert_course(&Course {
            id: 1,
            fullname: "Cálculo".into(),
            shortname: "CALC".into(),
            summary: None,
            last_synced_at: Some(synced),
            is_muted: false,
        })
        .unwrap();
        db.upsert_section(&Section {
            id: 10,
            course_id: 1,
            name: "S".into(),
            summary: None,
            activities: vec![],
        })
        .unwrap();
        let soon = Utc::now() + Duration::days(1);
        let later = Utc::now() + Duration::days(5);
        db.upsert_activity(&activity(1, Some(later), false))
            .unwrap();
        db.upsert_activity(&activity(2, Some(soon), false)).unwrap();
        db.upsert_activity(&activity(3, Some(soon), true)).unwrap(); // entregue
        db.upsert_activity(&activity(4, Some(Utc::now() - Duration::days(1)), false))
            .unwrap(); // passou
        db.upsert_activity(&activity(5, None, false)).unwrap();

        let c = &courses(&db).unwrap()[0];
        assert_eq!(c.activities_count, 5);
        assert_eq!(c.pending_deadlines, 2);
        assert_eq!(c.next_deadline.as_deref(), Some(soon.to_rfc3339().as_str()));
        assert!(c.last_synced_at.is_some());
    }

    #[test]
    fn course_export_preview_marks_materials_and_detects_pdfs() {
        let dir = tempfile::TempDir::new().unwrap();
        let pdf_path = dir.path().join("201_livro.pdf");
        std::fs::write(&pdf_path, b"%PDF-1.4 test bytes").unwrap();

        let db = Database::open_in_memory().unwrap();
        db.upsert_course(&Course {
            id: 5,
            fullname: "Sistemas Operacionais".into(),
            shortname: "SO".into(),
            summary: None,
            last_synced_at: None,
            is_muted: false,
        })
        .unwrap();
        db.upsert_section(&Section {
            id: 50,
            course_id: 5,
            name: "Unidade 1".into(),
            summary: None,
            activities: vec![],
        })
        .unwrap();
        db.upsert_section(&Section {
            id: 51,
            course_id: 5,
            name: "Tópico vazio".into(),
            summary: None,
            activities: vec![],
        })
        .unwrap();
        db.upsert_activity(&Activity {
            id: 201,
            section_id: 50,
            title: "Livro Texto".into(),
            activity_type: ActivityType::Resource,
            url: None,
            description: None,
            local_file_path: Some(pdf_path.to_string_lossy().to_string()),
            content_mime: Some("application/pdf".into()),
            due_date: None,
            is_submitted: false,
        })
        .unwrap();
        let mut assign = activity(203, None, false);
        assign.section_id = 50;
        db.upsert_activity(&assign).unwrap();

        let preview = course_export_preview(&db, 5).unwrap();
        assert_eq!(preview.course_id, 5);
        assert!(preview
            .suggested_filename
            .starts_with("0spof-sistemas-operacionais-"));
        assert!(preview.suggested_filename.ends_with(".zip"));
        // Tópico vazio sem summary é filtrado
        assert_eq!(preview.sections.len(), 1);
        let items = &preview.sections[0].items;
        assert_eq!(items.len(), 2);

        assert!(items[0].is_material);
        assert!(items[0].selected_by_default);
        assert!(items[0].is_pdf);
        assert!(items[0].has_local_file);
        assert_eq!(items[0].file_name.as_deref(), Some("201_livro.pdf"));
        assert_eq!(items[0].file_size_bytes, Some(19));

        assert!(!items[1].is_material);
        assert!(!items[1].selected_by_default);
        assert!(!items[1].is_pdf);
        assert!(!items[1].has_local_file);
    }

    fn seed_course_with_assign(db: &Database, assign_id: i64, due: chrono::DateTime<Utc>) {
        db.upsert_course(&Course {
            id: 1,
            fullname: "C".into(),
            shortname: "C".into(),
            summary: None,
            last_synced_at: None,
            is_muted: false,
        })
        .unwrap();
        db.upsert_section(&Section {
            id: 10,
            course_id: 1,
            name: "S".into(),
            summary: None,
            activities: vec![],
        })
        .unwrap();
        db.upsert_activity(&activity(assign_id, Some(due), false))
            .unwrap();
    }

    #[test]
    fn outbox_history_includes_delivered_with_times_and_margin() {
        let db = Database::open_in_memory().unwrap();
        let due = Utc::now() + Duration::days(2);
        seed_course_with_assign(&db, 77, due);

        let mut pending =
            crate::models::OutboxItem::new(77, "a.pdf".into(), "h1".into(), "/a".into());
        pending.last_error = Some("timeout".into());
        db.insert_outbox_item(&pending).unwrap();
        let mut delivered =
            crate::models::OutboxItem::new(99, "b.pdf".into(), "h2".into(), "/b".into());
        delivered.record_success();
        db.insert_outbox_item(&delivered).unwrap();

        // fila de envio (celular/contadores) continua só com o que falta enviar
        let pending_only = outbox_items(&db).unwrap();
        assert_eq!(pending_only.len(), 1);
        assert_eq!(pending_only[0].last_error.as_deref(), Some("timeout"));

        let all = outbox_history(&db).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].status, "pending");
        assert_eq!(all[1].status, "delivered");
        assert!(all[1].last_attempt_at.is_some());
        assert!(!all[0].created_at.is_empty());
        assert_eq!(all[0].due_date.as_deref(), Some(due.to_rfc3339().as_str()));
        let margin = all[0].margin_seconds.unwrap();
        assert!((margin - 2 * 86_400).abs() < 5);
        assert!(all[1].due_date.is_none() && all[1].margin_seconds.is_none());
    }

    #[test]
    fn submission_proof_report_includes_deadline_margin() {
        let db = Database::open_in_memory().unwrap();
        seed_course_with_assign(&db, 5, Utc::now() + Duration::days(1));
        let id = db
            .insert_outbox_item(&crate::models::OutboxItem::new(
                5,
                "x.pdf".into(),
                "hh".into(),
                "/x".into(),
            ))
            .unwrap();
        let proof = submission_proof(&db, id).unwrap();
        assert!(proof.formatted_report.contains("Folga em relação ao prazo"));
        assert!(proof.formatted_report.contains("antes do prazo"));
    }

    #[test]
    fn file_info_reports_name_and_size_and_rejects_bad_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("trabalho.pdf");
        std::fs::write(&file, b"12345").unwrap();
        let info = file_info(file.to_str().unwrap()).unwrap();
        assert_eq!(info.name, "trabalho.pdf");
        assert_eq!(info.size_bytes, 5);

        assert!(file_info(dir.path().to_str().unwrap())
            .unwrap_err()
            .contains("não é um arquivo"));
        assert!(file_info("/nao/existe.pdf")
            .unwrap_err()
            .contains("não encontrado"));
    }

    #[test]
    fn outbox_listing_propagates_due_date_lookup_errors() {
        let db = Database::open_in_memory().unwrap();
        db.insert_outbox_item(&crate::models::OutboxItem::new(
            1,
            "a".into(),
            "h".into(),
            "/a".into(),
        ))
        .unwrap();
        assert!(outbox_items(&db).is_ok());
        db.conn
            .lock()
            .unwrap()
            .execute("DROP TABLE activities", [])
            .unwrap();
        assert!(outbox_items(&db).is_err());
        assert!(outbox_history(&db).is_err());
    }
}
