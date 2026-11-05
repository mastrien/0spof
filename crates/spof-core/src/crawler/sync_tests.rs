use super::*;

#[test]
fn test_sync_from_html_flow() {
    let db = Database::open_in_memory().unwrap();
    let client = MoodleClient::new("https://moodle.if.edu.br".to_string()).unwrap();
    let crawler = CourseCrawler::new(db.clone(), client);

    let dashboard_html = r#"
        <html>
            <div class="dashboard-card">
                <a href="https://moodle.if.edu.br/course/view.php?id=99" class="coursename">
                    Compiladores
                </a>
            </div>
        </html>
    "#;

    let synced_count = crawler.sync_from_dashboard_html(dashboard_html).unwrap();
    assert_eq!(synced_count, 1);

    let courses = db.get_courses().unwrap();
    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].id, 99);

    let course_html = r#"
        <html>
            <li id="section-10" class="section" data-id="9910">
                <span class="sectionname">Módulo 1</span>
                <ul class="section img-text">
                    <li id="module-1" class="activity assign">
                        <a href="https://moodle.if.edu.br/mod/assign/view.php?id=1"><span class="instancename">Trabalho</span></a>
                    </li>
                </ul>
            </li>
        </html>
    "#;

    let (secs, acts) = crawler.sync_from_course_html(99, course_html).unwrap();
    assert_eq!(secs, 1);
    assert_eq!(acts, 1);

    let sections = db.get_sections_by_course(99).unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "Módulo 1");

    let activities = db.get_activities_by_section(9910).unwrap();
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].title, "Trabalho");
}

#[test]
fn test_sync_from_ajax_json_flow() {
    let db = Database::open_in_memory().unwrap();
    let client = MoodleClient::new("https://moodle2.jcr.ifsp.edu.br".to_string()).unwrap();
    let crawler = CourseCrawler::new(db.clone(), client);

    let ajax_json = r#"[{"error":false,"data":{"courses":[{"id":86,"fullname":"BANCO DE DADOS 1","shortname":"JCRBDD1","summary":""}]}}]"#;
    let synced = crawler.sync_from_ajax_courses_json(ajax_json).unwrap();
    assert_eq!(synced, 1);

    let courses = db.get_courses().unwrap();
    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].id, 86);
    assert_eq!(courses[0].fullname, "BANCO DE DADOS 1");
}

/// Regressão: com sessão expirada, `/my/` redireciona ao login mas a raiz `/` responde 200
/// como visitante. Isso não pode ser tratado como "sincronização bem-sucedida com 0 cursos".
#[test]
fn test_session_rejected_when_only_public_root_answers() {
    assert!(is_session_rejected(true, 1, 0));
    assert!(is_session_rejected(true, 0, 0));
    assert!(!is_session_rejected(false, 1, 0));
    assert!(!is_session_rejected(true, 1, 3));
}

#[tokio::test]
async fn test_sync_all_without_session_fails() {
    let db = Database::open_in_memory().unwrap();
    let client = MoodleClient::new("https://moodle.if.edu.br".to_string()).unwrap();
    let crawler = CourseCrawler::new(db, client);
    let result = crawler.sync_all().await;
    assert!(matches!(result, Err(SpofError::Authentication(_))));
}

#[tokio::test]
async fn test_download_activity_requires_session_and_existing_activity() {
    let db = Database::open_in_memory().unwrap();
    let anonymous_client = MoodleClient::new("https://moodle.if.edu.br".to_string()).unwrap();
    let anonymous_crawler = CourseCrawler::new(db.clone(), anonymous_client);
    let no_session = anonymous_crawler.download_activity(1, false).await;
    assert!(matches!(no_session, Err(SpofError::Authentication(_))));

    let session =
        crate::auth::MoodleSession::new("https://moodle.if.edu.br".to_string(), "abc".to_string());
    let authed_client = MoodleClient::new("https://moodle.if.edu.br".to_string())
        .unwrap()
        .with_session(session);
    let authed_crawler = CourseCrawler::new(db, authed_client)
        .with_cache_dir(std::env::temp_dir().join("0spof_test_cache"));
    let missing = authed_crawler.download_activity(424242, false).await;
    assert!(matches!(missing, Err(SpofError::Validation(_))));
}
