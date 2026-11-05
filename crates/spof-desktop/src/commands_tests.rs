use super::*;
use spof_core::models::{Activity, ActivityType, Course, Section};
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

    // 3. Add course, section and activity, then verify
    let course = Course {
        id: 99,
        fullname: "Compiladores".to_string(),
        shortname: "COMP".to_string(),
        summary: None,
        last_synced_at: None,
    };
    db.upsert_course(&course).unwrap();

    let section = Section {
        id: 990,
        course_id: 99,
        name: "Análise Léxica".to_string(),
        summary: Some("Introdução a autômatos".to_string()),
        activities: Vec::new(),
    };
    db.upsert_section(&section).unwrap();

    let activity = Activity {
        id: 9901,
        section_id: 990,
        title: "Implementar Lexer".to_string(),
        activity_type: ActivityType::Assign,
        url: Some("https://moodle.ifsp.edu.br/mod/assign/view.php?id=9901".to_string()),
        description: Some("Entregar arquivo .rs".to_string()),
        local_file_path: None,
        content_mime: None,
        due_date: None,
        is_submitted: false,
    };
    db.upsert_activity(&activity).unwrap();

    let course_details = handle_get_course_details(&db, 99).unwrap();
    assert_eq!(course_details.len(), 1);
    assert_eq!(course_details[0].activities.len(), 1);
    assert_eq!(course_details[0].activities[0].title, "Implementar Lexer");

    let courses = handle_get_courses(&db).unwrap();
    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].activities_count, 1);

    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"codigo do parser e lexer").unwrap();
    temp.flush().unwrap();

    let enqueue_res = handle_enqueue_submission(&db, 9901, temp.path().to_str().unwrap(), false)
        .await
        .unwrap();

    assert!(enqueue_res.outbox_id > 0);
    assert!(!enqueue_res.file_hash.is_empty());

    let outbox = handle_get_outbox_items(&db).unwrap();
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0].assign_id, 9901);

    let proof = handle_get_submission_proof(&db, enqueue_res.outbox_id).unwrap();
    assert_eq!(proof.assign_id, 9901);
    assert!(proof.formatted_report.contains("COMPROVANTE"));
}

#[tokio::test]
async fn test_desktop_enqueue_missing_file_error() {
    let db = Database::open_in_memory().unwrap();
    let res = handle_enqueue_submission(&db, 10, "C:/inexistente/arquivo.zip", false).await;
    assert!(res.is_err());
}

#[test]
fn test_desktop_save_session_validation_error() {
    let db = Database::open_in_memory().unwrap();
    let state = AppState::new(db);
    assert!(handle_save_session(&state, "".to_string(), "cookie".to_string()).is_err());
    assert!(handle_save_session(&state, "http://moodle".to_string(), "   ".to_string()).is_err());
}

#[tokio::test]
async fn test_desktop_sync_without_session_error() {
    let db = Database::open_in_memory().unwrap();
    let state = AppState::new(db);
    let res = handle_sync_moodle(&state).await;
    assert!(res.unwrap_err().contains("Nenhuma sessão configurada"));
}

#[tokio::test]
async fn test_desktop_download_without_session_error() {
    let db = Database::open_in_memory().unwrap();
    let state = AppState::new(db);
    let res = handle_download_activity_media(&state, 1, true).await;
    assert!(res.unwrap_err().contains("Sessão Moodle não configurada"));
}

#[test]
fn test_desktop_open_target_rejects_empty() {
    assert!(handle_open_target("   ").is_err());
}

#[test]
fn test_desktop_course_details_empty_course() {
    let db = Database::open_in_memory().unwrap();
    assert!(handle_get_course_details(&db, 404).unwrap().is_empty());
}
