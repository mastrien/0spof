use crate::models::{Activity, ActivityType, Course, Section};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentDetails {
    pub assign_id: i64,
    pub title: String,
    pub is_submitted: bool,
    pub due_date: Option<DateTime<Utc>>,
    pub time_remaining: Option<String>,
}

/// Parses the Moodle Dashboard (`/my/`) HTML to extract enrolled courses.
pub fn parse_dashboard_courses(html: &str) -> Vec<Course> {
    let document = Html::parse_document(html);
    let link_selector = Selector::parse("a[href*='/course/view.php?id=']").unwrap();

    let mut courses = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(id) = extract_id_from_url(href) {
                if seen_ids.insert(id) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    let title = text.trim().to_string();

                    if !title.is_empty() && !title.eq_ignore_ascii_case("visualizar") {
                        courses.push(Course {
                            id,
                            fullname: title.clone(),
                            shortname: format!("CURSO-{}", id),
                            summary: None,
                            last_synced_at: Some(Utc::now()),
                        });
                    }
                }
            }
        }
    }

    courses
}

/// Parses a Moodle Course Page (`/course/view.php?id=...`) to extract sections and activities.
pub fn parse_course_sections(html: &str, course_id: i64) -> Vec<Section> {
    let document = Html::parse_document(html);
    let section_selector = Selector::parse("li.section, div.section, .course-section").unwrap();
    let title_selector = Selector::parse(".sectionname, .section-title, h3, h4").unwrap();
    let activity_selector = Selector::parse("li.activity, .activity-item").unwrap();

    let mut sections = Vec::new();
    let mut default_section_id = course_id * 1000;

    for (idx, sec_elem) in document.select(&section_selector).enumerate() {
        let sec_id = sec_elem
            .value()
            .attr("id")
            .and_then(|id_str| id_str.strip_prefix("section-"))
            .and_then(|num| num.parse::<i64>().ok())
            .unwrap_or_else(|| {
                default_section_id += 1;
                default_section_id
            });

        let sec_name = sec_elem
            .select(&title_selector)
            .next()
            .map(|t| t.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Tópico {}", idx));

        let mut activities = Vec::new();

        for act_elem in sec_elem.select(&activity_selector) {
            if let Some(activity) = parse_activity_element(&act_elem, sec_id) {
                activities.push(activity);
            }
        }

        sections.push(Section {
            id: sec_id,
            course_id,
            name: sec_name,
            summary: None,
            activities,
        });
    }

    // Fallback: If no structured sections were found, parse activities globally into a single general section
    if sections.is_empty() {
        let mut general_activities = Vec::new();
        for act_elem in document.select(&activity_selector) {
            if let Some(activity) = parse_activity_element(&act_elem, course_id * 1000) {
                general_activities.push(activity);
            }
        }

        if !general_activities.is_empty() {
            sections.push(Section {
                id: course_id * 1000,
                course_id,
                name: "Geral".to_string(),
                summary: None,
                activities: general_activities,
            });
        }
    }

    sections
}

fn parse_activity_element(elem: &scraper::ElementRef, section_id: i64) -> Option<Activity> {
    let link_selector = Selector::parse("a[href*='/mod/']").unwrap();
    let title_selector = Selector::parse(".instancename, .activityname, a").unwrap();

    let link_elem = elem.select(&link_selector).next()?;
    let href = link_elem.value().attr("href")?;
    let act_id = extract_id_from_url(href)?;

    let act_type = extract_mod_type_from_url(href)
        .map(|t| ActivityType::from_moodle_modname(&t))
        .unwrap_or(ActivityType::Other("unknown".to_string()));

    let title = elem
        .select(&title_selector)
        .next()
        .map(|t| t.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("Atividade {}", act_id));

    // Clean Moodle accessibility suffix text like " Tarefa" or " Arquivo"
    let clean_title = title
        .split('\n')
        .next()
        .unwrap_or(&title)
        .trim()
        .to_string();

    Some(Activity {
        id: act_id,
        section_id,
        title: clean_title,
        activity_type: act_type,
        url: Some(href.to_string()),
        due_date: None,
        is_submitted: false,
    })
}

/// Parses the assignment details page (`/mod/assign/view.php?id=...`).
pub fn parse_assignment_details(html: &str, assign_id: i64) -> AssignmentDetails {
    let document = Html::parse_document(html);
    let title_selector = Selector::parse("h2, .page-header-headings h1").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .map(|h| h.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .unwrap_or_else(|| format!("Tarefa #{}", assign_id));

    let body_text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let is_submitted = body_text.contains("Submetido para avaliação")
        || body_text.contains("Submitted for grading")
        || body_text.contains("Envio concluído");

    AssignmentDetails {
        assign_id,
        title,
        is_submitted,
        due_date: None,
        time_remaining: None,
    }
}

fn extract_id_from_url(url: &str) -> Option<i64> {
    let query_start = url.find('?')?;
    let query = &url[query_start + 1..];

    for pair in query.split('&') {
        let mut parts = pair.split('=');
        if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
            if key == "id" {
                return val.parse::<i64>().ok();
            }
        }
    }
    None
}

fn extract_mod_type_from_url(url: &str) -> Option<String> {
    if let Some(pos) = url.find("/mod/") {
        let after = &url[pos + 5..];
        if let Some(slash_pos) = after.find('/') {
            return Some(after[..slash_pos].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
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
}
