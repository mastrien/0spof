use super::*;
use crate::models::{Activity, ActivityType, Course, OutboxItem, OutboxStatus, Section};
use chrono::Utc;
use tempfile::NamedTempFile;

#[test]
fn test_in_memory_db_schema() {
    let db = Database::open_in_memory().expect("in-memory db failed");
    let courses = db.get_courses().expect("failed to get courses");
    assert!(courses.is_empty());
}

#[test]
fn test_file_db_schema() {
    let temp = NamedTempFile::new().expect("temp file failed");
    let db = Database::open(temp.path()).expect("file db failed");
    let courses = db.get_courses().expect("failed to get courses");
    assert!(courses.is_empty());
}

#[test]
fn test_courses_crud() {
    let db = Database::open_in_memory().unwrap();
    let course = Course {
        id: 10,
        fullname: "Sistemas Operacionais".to_string(),
        shortname: "SO".to_string(),
        summary: Some("Ementa de SO".to_string()),
        last_synced_at: Some(Utc::now()),
    };

    db.upsert_course(&course).unwrap();
    let fetched = db.get_course(10).unwrap().expect("course not found");
    assert_eq!(fetched.fullname, "Sistemas Operacionais");

    // Update
    let updated_course = Course {
        id: 10,
        fullname: "Sistemas Operacionais Avançados".to_string(),
        shortname: "SO-Adv".to_string(),
        summary: None,
        last_synced_at: None,
    };
    db.upsert_course(&updated_course).unwrap();
    let fetched_updated = db.get_course(10).unwrap().unwrap();
    assert_eq!(fetched_updated.fullname, "Sistemas Operacionais Avançados");
    assert_eq!(fetched_updated.summary, None);
}

#[test]
fn test_sections_and_activities() {
    let db = Database::open_in_memory().unwrap();
    let course = Course {
        id: 1,
        fullname: "Cálculo 1".to_string(),
        shortname: "CALC1".to_string(),
        summary: None,
        last_synced_at: None,
    };
    db.upsert_course(&course).unwrap();

    let section = Section {
        id: 100,
        course_id: 1,
        name: "Limites e Derivadas".to_string(),
        summary: None,
        activities: Vec::new(),
    };
    db.upsert_section(&section).unwrap();

    let activity1 = Activity {
        id: 1001,
        section_id: 100,
        title: "Lista 1 - Limites".to_string(),
        activity_type: ActivityType::Assign,
        url: Some("https://moodle/assign/1".to_string()),
        description: Some("Resolver exercícios 1 a 10".to_string()),
        local_file_path: None,
        content_mime: None,
        due_date: Some(Utc::now()),
        is_submitted: false,
    };
    let activity2 = Activity {
        id: 1002,
        section_id: 100,
        title: "Apostila de Limites".to_string(),
        activity_type: ActivityType::Resource,
        url: None,
        description: None,
        local_file_path: None,
        content_mime: None,
        due_date: None,
        is_submitted: true,
    };

    db.upsert_activity(&activity1).unwrap();
    db.upsert_activity(&activity2).unwrap();

    db.update_activity_local_file(1002, "C:/cache/apostila.pdf", "application/pdf")
        .unwrap();
    let fetched_act2 = db.get_activity(1002).unwrap().unwrap();
    assert_eq!(
        fetched_act2.local_file_path.as_deref(),
        Some("C:/cache/apostila.pdf")
    );
    assert_eq!(
        fetched_act2.content_mime.as_deref(),
        Some("application/pdf")
    );

    let acts = db.get_activities_by_section(100).unwrap();
    assert_eq!(acts.len(), 2);

    let sections = db.get_sections_by_course(1).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].activities.len(), 2);

    let upcoming = db.get_upcoming_deadlines().unwrap();
    assert_eq!(upcoming.len(), 1);
    assert_eq!(upcoming[0].id, 1001);
}

#[test]
fn test_outbox_items_crud() {
    let db = Database::open_in_memory().unwrap();
    let mut item = OutboxItem::new(
        55,
        "relatorio.pdf".to_string(),
        "hash_123456789".to_string(),
        "/caminho/relatorio.pdf".to_string(),
    );

    let id = db.insert_outbox_item(&item).unwrap();
    assert!(id > 0);
    item.id = Some(id);

    let fetched = db.get_outbox_item(id).unwrap().expect("item not found");
    assert_eq!(fetched.filename, "relatorio.pdf");
    assert_eq!(fetched.status, OutboxStatus::Pending);

    let pending = db.get_pending_outbox_items().unwrap();
    assert_eq!(pending.len(), 1);

    // Update item with failure and retry
    item.record_failure("Erro 500 no Moodle".to_string());
    db.update_outbox_item(&item).unwrap();

    let fetched_failed = db.get_outbox_item(id).unwrap().unwrap();
    assert_eq!(fetched_failed.status, OutboxStatus::Failed);
    assert_eq!(fetched_failed.retry_count, 1);
    assert_eq!(
        fetched_failed.last_error.as_deref(),
        Some("Erro 500 no Moodle")
    );

    // Mark delivered
    item.record_success();
    db.update_outbox_item(&item).unwrap();

    let pending_after = db.get_pending_outbox_items().unwrap();
    assert!(pending_after.is_empty());
}
