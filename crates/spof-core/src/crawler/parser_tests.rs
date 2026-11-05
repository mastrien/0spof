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
fn test_parse_ajax_enrolled_courses() {
    let json_payload = r#"[
        {
            "error": false,
            "data": {
                "courses": [
                    {
                        "id": 86,
                        "fullname": "[20251.3.JCR.TCN.ADS] BANCO DE DADOS 1",
                        "shortname": "JCRBDD1",
                        "summary": "Disciplina de bancos relacionais"
                    },
                    {
                        "id": 87,
                        "fullname": "[20251.3.JCR.TCN.ADS] DESENVOLVIMENTO WEB 1",
                        "shortname": "JCRWEB1",
                        "summary": ""
                    }
                ]
            }
        }
    ]"#;

    let courses = parse_ajax_enrolled_courses(json_payload);
    assert_eq!(courses.len(), 2);
    assert_eq!(courses[0].id, 86);
    assert_eq!(
        courses[0].fullname,
        "[20251.3.JCR.TCN.ADS] BANCO DE DADOS 1"
    );
    assert_eq!(courses[0].shortname, "JCRBDD1");
    assert_eq!(
        courses[0].summary.as_deref(),
        Some("Disciplina de bancos relacionais")
    );
    assert_eq!(courses[1].id, 87);
    assert_eq!(courses[1].summary, None);
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
    assert_eq!(sections[0].id, 420001);
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
fn test_parse_moodle4_course_sections_no_duplicate_activities() {
    let html = r#"
        <html>
            <li id="section-1" class="section course-section main" data-id="1650" data-sectionname="Aula 01 - Proposições">
                <h3 class="sectionname"><a href="/course/section.php?id=1650">Aula 01 - Proposições</a></h3>
                <ul class="section">
                    <li class="activity activity-wrapper assign modtype_assign" id="module-2968" data-id="2968">
                        <div class="activity-item focus-control" data-activityname="Lista de Exercícios - Fundamentos de Lógica">
                            <div class="activityname">
                                <a href="https://moodle2.jcr.ifsp.edu.br/mod/assign/view.php?id=2968">
                                    <span class="instancename">Lista de Exercícios - Fundamentos de Lógica <span class="accesshide"> Tarefa</span></span>
                                </a>
                            </div>
                        </div>
                    </li>
                </ul>
            </li>
        </html>
    "#;

    let sections = parse_course_sections(html, 86);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, 1650);
    assert_eq!(sections[0].name, "Aula 01 - Proposições");
    assert_eq!(sections[0].activities.len(), 1);
    assert_eq!(sections[0].activities[0].id, 2968);
    assert_eq!(
        sections[0].activities[0].title,
        "Lista de Exercícios - Fundamentos de Lógica"
    );
    assert_eq!(
        sections[0].activities[0].activity_type,
        ActivityType::Assign
    );
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
