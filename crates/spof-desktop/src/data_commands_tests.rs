use super::*;
use crate::commands::{handle_get_session_info, handle_save_session, AppState};
use chrono::Utc;
use spof_core::models::{Course, Section};
use spof_core::storage::Database;

#[test]
fn test_handle_logout_clears_session() {
    let db = Database::open_in_memory().unwrap();
    let state = AppState::new(db);

    let saved = handle_save_session(
        &state,
        "https://moodle.exemplo.br".to_string(),
        "cookie123".to_string(),
    )
    .unwrap();
    assert!(saved.is_authenticated);

    let current = handle_get_session_info(&state);
    assert!(current.is_authenticated);

    let logged_out = handle_logout(&state);
    assert!(!logged_out.is_authenticated);
    assert_eq!(logged_out.base_url, None);

    let post_logout = handle_get_session_info(&state);
    assert!(!post_logout.is_authenticated);
}

#[test]
fn test_handle_wipe_all_data_clears_db() {
    let db = Database::open_in_memory().unwrap();
    let course = Course {
        id: 77,
        fullname: "Sistemas Distribuídos".to_string(),
        shortname: "SD".to_string(),
        summary: None,
        last_synced_at: Some(Utc::now()),
        is_muted: false,
    };
    db.upsert_course(&course).unwrap();
    assert_eq!(db.get_courses().unwrap().len(), 1);

    let state = AppState::new(db);
    let wipe_res = handle_wipe_all_data(&state);
    assert!(wipe_res.is_ok());

    assert!(state.db.get_courses().unwrap().is_empty());
}

#[test]
fn test_handle_export_and_import_data() {
    let db = Database::open_in_memory().unwrap();
    let course = Course {
        id: 88,
        fullname: "Engenharia de Software".to_string(),
        shortname: "ES".to_string(),
        summary: None,
        last_synced_at: Some(Utc::now()),
        is_muted: false,
    };
    db.upsert_course(&course).unwrap();
    let section = Section {
        id: 880,
        course_id: 88,
        name: "Módulo 1".to_string(),
        summary: None,
        activities: Vec::new(),
    };
    db.upsert_section(&section).unwrap();

    let state1 = AppState::new(db);
    let export_res = handle_export_data(&state1);
    assert!(export_res.is_ok());
    let json_content = export_res.unwrap();
    assert!(json_content.contains("Engenharia de Software"));

    let db2 = Database::open_in_memory().unwrap();
    let state2 = AppState::new(db2);
    let import_res = handle_import_data(&state2, json_content);
    assert!(import_res.is_ok());
    let summary = import_res.unwrap();
    assert_eq!(summary.courses_imported, 1);
    assert_eq!(summary.sections_imported, 1);

    let restored = state2.db.get_courses().unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].fullname, "Engenharia de Software");
}

#[test]
fn test_handle_get_app_version() {
    let version = super::handle_get_app_version();
    assert!(version.starts_with('v'));
    assert!(spof_core::update::parse_semver_tuple(&version).is_some());
}
