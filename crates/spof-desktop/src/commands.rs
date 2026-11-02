use serde::{Deserialize, Serialize};
use spof_core::auth::MoodleSession;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemStatusDto {
    pub courses_count: usize,
    pub deadlines_count: usize,
    pub outbox_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CourseDto {
    pub id: i64,
    pub fullname: String,
    pub shortname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboxItemDto {
    pub id: i64,
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub status: String,
    pub retry_count: u32,
    pub has_rfc3161_token: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnqueueResultDto {
    pub outbox_id: i64,
    pub file_hash: String,
    pub has_tsa_token: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofDto {
    pub assign_id: i64,
    pub filename: String,
    pub file_hash: String,
    pub formatted_report: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfoDto {
    pub is_authenticated: bool,
    pub base_url: Option<String>,
}

// Logic handlers decoupled from Tauri macros for direct testing and IPC execution

pub fn handle_get_system_status(db: &Database) -> Result<SystemStatusDto, String> {
    let courses = db.get_courses().map_err(|e| e.to_string())?;
    let deadlines = db.get_upcoming_deadlines().map_err(|e| e.to_string())?;
    let pending = db.get_pending_outbox_items().map_err(|e| e.to_string())?;

    Ok(SystemStatusDto {
        courses_count: courses.len(),
        deadlines_count: deadlines.len(),
        outbox_count: pending.len(),
    })
}

pub fn handle_get_courses(db: &Database) -> Result<Vec<CourseDto>, String> {
    let courses = db.get_courses().map_err(|e| e.to_string())?;
    Ok(courses
        .into_iter()
        .map(|c| CourseDto {
            id: c.id,
            fullname: c.fullname,
            shortname: c.shortname,
        })
        .collect())
}

pub fn handle_get_outbox_items(db: &Database) -> Result<Vec<OutboxItemDto>, String> {
    let items = db.get_pending_outbox_items().map_err(|e| e.to_string())?;
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
    db: &Database,
    assign_id: i64,
    file_path: &str,
    fetch_tsa: bool,
) -> Result<EnqueueResultDto, String> {
    let path = Path::new(file_path);
    let engine = OutboxEngine::new(db.clone());
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

pub fn handle_get_submission_proof(db: &Database, outbox_id: i64) -> Result<ProofDto, String> {
    let engine = OutboxEngine::new(db.clone());
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
    let clean_url = base_url.trim().trim_end_matches('/').to_string();
    let clean_cookie = session_cookie.trim().to_string();

    if clean_url.is_empty() || clean_cookie.is_empty() {
        return Err("A URL e o cookie de sessão não podem ser vazios".to_string());
    }

    let session = MoodleSession::new(clean_url.clone(), clean_cookie);
    let mut lock = state.session.lock().unwrap();
    *lock = Some(session);

    Ok(SessionInfoDto {
        is_authenticated: true,
        base_url: Some(clean_url),
    })
}

pub fn handle_get_session_info(state: &AppState) -> SessionInfoDto {
    let lock = state.session.lock().unwrap();
    if let Some(sess) = &*lock {
        SessionInfoDto {
            is_authenticated: !sess.is_expired(),
            base_url: Some(sess.base_url.clone()),
        }
    } else {
        SessionInfoDto {
            is_authenticated: false,
            base_url: None,
        }
    }
}

// Tauri IPC Command wrappers
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
    assign_id: i64,
    file_path: String,
    fetch_tsa: bool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<EnqueueResultDto, String> {
    handle_enqueue_submission(&state.db, assign_id, &file_path, fetch_tsa).await
}

#[tauri::command]
pub fn get_submission_proof(
    id: i64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ProofDto, String> {
    handle_get_submission_proof(&state.db, id)
}

#[tauri::command]
pub fn save_session(
    base_url: String,
    session_cookie: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SessionInfoDto, String> {
    handle_save_session(&state, base_url, session_cookie)
}

#[tauri::command]
pub fn get_session_info(state: tauri::State<'_, Arc<AppState>>) -> SessionInfoDto {
    handle_get_session_info(&state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spof_core::models::Course;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_desktop_handlers_flow() {
        let db = Database::open_in_memory().unwrap();
        let state = AppState::new(db.clone());

        // 1. Initial status and session info
        let status = handle_get_system_status(&db).unwrap();
        assert_eq!(status.courses_count, 0);

        let sess_info = handle_get_session_info(&state);
        assert!(!sess_info.is_authenticated);
        assert!(sess_info.base_url.is_none());

        // 2. Save session
        let saved_sess = handle_save_session(
            &state,
            "https://moodle.ifsp.edu.br".to_string(),
            "cookie12345".to_string(),
        )
        .unwrap();
        assert!(saved_sess.is_authenticated);
        assert_eq!(
            saved_sess.base_url.as_deref(),
            Some("https://moodle.ifsp.edu.br")
        );

        let sess_after = handle_get_session_info(&state);
        assert!(sess_after.is_authenticated);

        // 3. Add course and verify
        let c = Course {
            id: 99,
            fullname: "Compiladores".to_string(),
            shortname: "COMP".to_string(),
            summary: None,
            last_synced_at: None,
        };
        db.upsert_course(&c).unwrap();

        let courses = handle_get_courses(&db).unwrap();
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].fullname, "Compiladores");

        // 4. Enqueue submission
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"codigo do parser e lexer").unwrap();
        temp.flush().unwrap();

        let enqueue_res =
            handle_enqueue_submission(&db, 9901, temp.path().to_str().unwrap(), false)
                .await
                .unwrap();

        assert!(enqueue_res.outbox_id > 0);
        assert!(!enqueue_res.file_hash.is_empty());

        // 5. Check outbox
        let outbox = handle_get_outbox_items(&db).unwrap();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].assign_id, 9901);

        // 6. Get proof
        let proof = handle_get_submission_proof(&db, enqueue_res.outbox_id).unwrap();
        assert_eq!(proof.assign_id, 9901);
        assert!(proof.formatted_report.contains("COMPROVANTE"));
    }

    #[test]
    fn test_desktop_save_session_validation_error() {
        let db = Database::open_in_memory().unwrap();
        let state = AppState::new(db);
        let res = handle_save_session(&state, "   ".to_string(), "sess".to_string());
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_desktop_enqueue_missing_file_error() {
        let db = Database::open_in_memory().unwrap();
        let res = handle_enqueue_submission(&db, 1, "arquivo_que_nao_existe.xyz", false).await;
        assert!(res.is_err());
    }
}
