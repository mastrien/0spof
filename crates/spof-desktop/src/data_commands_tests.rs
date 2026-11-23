use super::*;
use crate::commands::{handle_get_session_info, handle_save_session, AppState};
use chrono::Utc;
use spof_core::models::{Activity, ActivityType, Course, Section};
use spof_core::storage::Database;
use tempfile::TempDir;

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

#[test]
fn test_handle_selective_course_export_preview_and_import() {
    let src_dir = TempDir::new().unwrap();
    let pdf_path = src_dir.path().join("501_slides.pdf");
    std::fs::write(&pdf_path, b"%PDF-1.4\nslides").unwrap();

    let db1 = Database::open_in_memory().unwrap();
    db1.upsert_course(&Course {
        id: 12,
        fullname: "Banco de Dados".to_string(),
        shortname: "BD".to_string(),
        summary: None,
        last_synced_at: Some(Utc::now()),
        is_muted: false,
    })
    .unwrap();
    db1.upsert_section(&Section {
        id: 120,
        course_id: 12,
        name: "Aula 1".to_string(),
        summary: None,
        activities: Vec::new(),
    })
    .unwrap();
    db1.upsert_activity(&Activity {
        id: 501,
        section_id: 120,
        title: "Slides Aula 1".to_string(),
        activity_type: ActivityType::Resource,
        url: Some("https://moodle.test/mod/resource/view.php?id=501".to_string()),
        description: None,
        local_file_path: Some(pdf_path.to_string_lossy().to_string()),
        content_mime: Some("application/pdf".to_string()),
        due_date: None,
        is_submitted: false,
    })
    .unwrap();
    db1.upsert_activity(&Activity {
        id: 502,
        section_id: 120,
        title: "Entrega Prática".to_string(),
        activity_type: ActivityType::Assign,
        url: Some("https://moodle.test/mod/assign/view.php?id=502".to_string()),
        description: None,
        local_file_path: None,
        content_mime: None,
        due_date: Some(Utc::now()),
        is_submitted: false,
    })
    .unwrap();

    let state1 = AppState::new(db1);
    let preview = handle_get_course_export_preview(&state1, 12).unwrap();
    assert_eq!(preview.sections.len(), 1);
    assert_eq!(preview.sections[0].items.len(), 2);
    assert!(preview.sections[0].items[0].selected_by_default);
    assert!(preview.sections[0].items[0].is_pdf);
    assert!(!preview.sections[0].items[1].selected_by_default);

    let zip_bytes = handle_export_course_package(
        &state1,
        CourseExportSelection {
            course_id: 12,
            activity_ids: vec![501],
            include_section_ids: Vec::new(),
            include_files: true,
        },
    )
    .unwrap();

    let dst_dir = TempDir::new().unwrap();
    let db2 = Database::open_in_memory().unwrap();
    let state2 = AppState::new(db2);
    let summary = handle_import_package_bytes_with_cache(
        &state2,
        &zip_bytes,
        Some(dst_dir.path().to_path_buf()),
    )
    .unwrap();

    assert_eq!(summary.courses_imported, 1);
    assert_eq!(summary.activities_imported, 1);
    assert_eq!(summary.files_imported, 1);
    let restored = state2.db.get_activity(501).unwrap().unwrap();
    assert!(restored.local_file_path.is_some());
}
