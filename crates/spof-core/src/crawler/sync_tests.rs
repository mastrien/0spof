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
