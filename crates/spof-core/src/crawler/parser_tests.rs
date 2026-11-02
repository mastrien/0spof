use super::*;

#[test]
fn test_extract_id_from_url() {
    assert_eq!(
        extract_id_from_url("https://moodle.edu.br/course/view.php?id=42"),
        Some(42)
    );
    assert_eq!(
        extract_id_from_url("/mod/assign/view.php?id=1055&action=view"),
        Some(1055)
    );
    assert_eq!(extract_id_from_url("https://moodle.edu.br/my/"), None);
}

#[test]
fn test_extract_mod_type_from_url() {
    assert_eq!(
        extract_mod_type_from_url("https://moodle.edu.br/mod/assign/view.php?id=12"),
        Some("assign".to_string())
    );
    assert_eq!(
        extract_mod_type_from_url("/mod/resource/view.php?id=99"),
        Some("resource".to_string())
    );
    assert_eq!(extract_mod_type_from_url("/course/view.php?id=1"), None);
}

#[test]
fn test_parse_dashboard_courses() {
    let html = r#"
        <html>
            <div class="dashboard-card">
                <a href="https://moodle.if.edu.br/course/view.php?id=101" class="coursename">
                    Engenharia de Software
                </a>
            </div>
            <div class="dashboard-card">
                <a href="https://moodle.if.edu.br/course/view.php?id=102" class="coursename">
                    Banco de Dados II
                </a>
            </div>
        </html>
    "#;

    let courses = parse_dashboard_courses(html);
    assert_eq!(courses.len(), 2);
    assert_eq!(courses[0].id, 101);
    assert_eq!(courses[0].fullname, "Engenharia de Software");
    assert_eq!(courses[1].id, 102);
    assert_eq!(courses[1].fullname, "Banco de Dados II");
}

#[test]
fn test_parse_course_sections_and_activities() {
    let html = r#"
        <html>
            <ul class="topics">
                <li id="section-1" class="section">
                    <h3 class="sectionname">Semana 1: Introdução</h3>
                    <ul class="section img-text">
                        <li id="module-501" class="activity assign modtype_assign">
                            <a href="https://moodle/mod/assign/view.php?id=501">
                                <span class="instancename">Trabalho Prático 1</span>
                            </a>
                        </li>
                        <li id="module-502" class="activity resource modtype_resource">
                            <a href="https://moodle/mod/resource/view.php?id=502">
                                <span class="instancename">Slides Aula 1</span>
                            </a>
                        </li>
                    </ul>
                </li>
            </ul>
        </html>
    "#;

    let sections = parse_course_sections(html, 42);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, 1);
    assert_eq!(sections[0].name, "Semana 1: Introdução");
    assert_eq!(sections[0].activities.len(), 2);

    let assign = &sections[0].activities[0];
    assert_eq!(assign.id, 501);
    assert_eq!(assign.title, "Trabalho Prático 1");
    assert_eq!(assign.activity_type, ActivityType::Assign);

    let resource = &sections[0].activities[1];
    assert_eq!(resource.id, 502);
    assert_eq!(resource.title, "Slides Aula 1");
    assert_eq!(resource.activity_type, ActivityType::Resource);
}

#[test]
fn test_parse_assignment_details() {
    let html = r#"
        <html>
            <h2>Projeto Final da Disciplina</h2>
            <div class="submissionstatustable">
                <table>
                    <tr>
                        <td>Status do envio</td>
                        <td>Submetido para avaliação</td>
                    </tr>
                </table>
            </div>
        </html>
    "#;

    let details = parse_assignment_details(html, 88);
    assert_eq!(details.assign_id, 88);
    assert_eq!(details.title, "Projeto Final da Disciplina");
    assert!(details.is_submitted);
}
