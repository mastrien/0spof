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
    let mut seen_course_ids = std::collections::HashSet::new();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(course_id) = extract_id_from_url(href) {
                if seen_course_ids.insert(course_id) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    let title = text.trim().to_string();

                    if !title.is_empty() && !title.eq_ignore_ascii_case("visualizar") {
                        courses.push(Course {
                            id: course_id,
                            fullname: title.clone(),
                            shortname: format!("CURSO-{}", course_id),
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

    for (section_index, section_element) in document.select(&section_selector).enumerate() {
        let section_id = section_element
            .value()
            .attr("id")
            .and_then(|id_str| id_str.strip_prefix("section-"))
            .and_then(|num_str| num_str.parse::<i64>().ok())
            .unwrap_or_else(|| {
                default_section_id += 1;
                default_section_id
            });

        let section_name = section_element
            .select(&title_selector)
            .next()
            .map(|t| t.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Tópico {}", section_index));

        let mut activities = Vec::new();

        for activity_element in section_element.select(&activity_selector) {
            if let Some(activity) = parse_activity_element(&activity_element, section_id) {
                activities.push(activity);
            }
        }

        sections.push(Section {
            id: section_id,
            course_id,
            name: section_name,
            summary: None,
            activities,
        });
    }

    // Fallback: If no structured sections were found, parse activities globally into a single general section
    if sections.is_empty() {
        let mut general_activities = Vec::new();
        for activity_element in document.select(&activity_selector) {
            if let Some(activity) = parse_activity_element(&activity_element, course_id * 1000) {
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

fn parse_activity_element(element: &scraper::ElementRef, section_id: i64) -> Option<Activity> {
    let link_selector = Selector::parse("a[href*='/mod/']").unwrap();
    let title_selector = Selector::parse(".instancename, .activityname, a").unwrap();

    let link_element = element.select(&link_selector).next()?;
    let href = link_element.value().attr("href")?;
    let activity_id = extract_id_from_url(href)?;

    let activity_type = extract_mod_type_from_url(href)
        .map(|mod_str| ActivityType::from_moodle_modname(&mod_str))
        .unwrap_or(ActivityType::Other("unknown".to_string()));

    let title = element
        .select(&title_selector)
        .next()
        .map(|t| t.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("Atividade {}", activity_id));

    // Clean Moodle accessibility suffix text like " Tarefa" or " Arquivo"
    let clean_title = title
        .split('\n')
        .next()
        .unwrap_or(&title)
        .trim()
        .to_string();

    Some(Activity {
        id: activity_id,
        section_id,
        title: clean_title,
        activity_type,
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
        .map(|heading| {
            heading
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
        })
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

#[path = "parser_tests.rs"]
#[cfg(test)]
mod tests;
