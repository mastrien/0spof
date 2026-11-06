use crate::models::{Activity, ActivityType, Course, OutboxItem, OutboxStatus, Section};
use crate::storage::Database;
use chrono::Utc;

#[test]
fn test_wipe_all_data_flow() {
    let db = Database::open_in_memory().expect("in-memory db failed");

    let course = Course {
        id: 101,
        fullname: "Algoritmos e Estruturas".to_string(),
        shortname: "A&E".to_string(),
        summary: Some("Ementa da disciplina".to_string()),
        last_synced_at: Some(Utc::now()),
        is_muted: false,
    };
    db.upsert_course(&course).expect("upsert course failed");

    let section = Section {
        id: 201,
        course_id: 101,
        name: "Semana 1".to_string(),
        summary: None,
        activities: Vec::new(),
    };
    db.upsert_section(&section).expect("upsert section failed");

    let activity = Activity {
        id: 301,
        section_id: 201,
        title: "Laboratório 1".to_string(),
        activity_type: ActivityType::Assign,
        url: Some("https://moodle.test/mod/assign/view.php?id=301".to_string()),
        description: Some("<p>Entregar relatório</p>".to_string()),
        local_file_path: None,
        content_mime: None,
        due_date: Some(Utc::now()),
        is_submitted: false,
    };
    db.upsert_activity(&activity)
        .expect("upsert activity failed");

    assert_eq!(db.get_courses().unwrap().len(), 1);
    assert_eq!(db.get_upcoming_deadlines().unwrap().len(), 1);

    db.wipe_all_data().expect("wipe_all_data failed");

    assert!(db.get_courses().unwrap().is_empty());
    assert!(db.get_upcoming_deadlines().unwrap().is_empty());
    assert!(db.get_pending_outbox_items().unwrap().is_empty());
}

#[test]
fn test_export_and_import_backup_flow() {
    let db1 = Database::open_in_memory().expect("db1 failed");

    let course = Course {
        id: 42,
        fullname: "Compiladores".to_string(),
        shortname: "CMP".to_string(),
        summary: None,
        last_synced_at: Some(Utc::now()),
        is_muted: true,
    };
    db1.upsert_course(&course).expect("course insert failed");

    let section = Section {
        id: 420,
        course_id: 42,
        name: "Introdução ao Léxico".to_string(),
        summary: Some("Capítulo 1".to_string()),
        activities: Vec::new(),
    };
    db1.upsert_section(&section).expect("section insert failed");

    let activity = Activity {
        id: 4200,
        section_id: 420,
        title: "Trabalho 1: Analisador Léxico".to_string(),
        activity_type: ActivityType::Assign,
        url: Some("https://moodle.test/assign/4200".to_string()),
        description: Some("Implementar scanner".to_string()),
        local_file_path: Some("/tmp/scanner.rs".to_string()),
        content_mime: Some("text/rust".to_string()),
        due_date: Some(Utc::now()),
        is_submitted: true,
    };
    db1.upsert_activity(&activity)
        .expect("activity insert failed");

    let outbox_item = OutboxItem {
        id: None,
        assign_id: 4200,
        filename: "scanner.rs".to_string(),
        file_hash: "abc123456789".to_string(),
        local_file_path: "/tmp/scanner.rs".to_string(),
        status: OutboxStatus::Pending,
        retry_count: 0,
        last_attempt_at: None,
        last_error: None,
        rfc3161_token: Some(vec![1, 2, 3, 4]),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db1.insert_outbox_item(&outbox_item)
        .expect("outbox insert failed");

    let json_backup = db1.export_data_json().expect("export failed");
    assert!(json_backup.contains("Compiladores"));
    assert!(json_backup.contains("Analisador Léxico"));

    let db2 = Database::open_in_memory().expect("db2 failed");
    let summary = db2
        .import_data_json(&json_backup)
        .expect("import json failed");
    assert_eq!(summary.courses_imported, 1);
    assert_eq!(summary.sections_imported, 1);
    assert_eq!(summary.activities_imported, 1);
    assert_eq!(summary.outbox_items_imported, 1);

    let restored_courses = db2.get_courses().unwrap();
    assert_eq!(restored_courses.len(), 1);
    assert_eq!(restored_courses[0].fullname, "Compiladores");
    assert!(restored_courses[0].is_muted);

    let restored_sections = db2.get_sections_by_course(42).unwrap();
    assert_eq!(restored_sections.len(), 1);
    assert_eq!(restored_sections[0].name, "Introdução ao Léxico");

    let restored_activities = db2.get_activities_by_section(420).unwrap();
    assert_eq!(restored_activities.len(), 1);
    assert_eq!(
        restored_activities[0].title,
        "Trabalho 1: Analisador Léxico"
    );
    assert!(restored_activities[0].is_submitted);
}

#[test]
fn test_import_invalid_json() {
    let db = Database::open_in_memory().expect("db failed");
    let res = db.import_data_json("{ invalid json format }");
    assert!(res.is_err());
}
