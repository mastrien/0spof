pub use crate::crawler::ajax::parse_ajax_enrolled_courses;
pub use crate::crawler::assignment_parser::{
    clean_html, parse_assignment_details, AssignmentDetails,
};
use crate::models::{Activity, ActivityType, Course, Section};
use chrono::Utc;
use scraper::{Html, Selector};

/// Parses the Moodle Dashboard (`/my/` or `/my/courses.php`) HTML to extract enrolled courses.
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
                    let mut title = text.trim().to_string();

                    if is_generic_link_label(&title) {
                        if let Some(aria) = element.value().attr("aria-label") {
                            let clean_aria = aria.trim();
                            if !clean_aria.is_empty() {
                                title = clean_aria.to_string();
                            }
                        } else if let Some(attr_title) = element.value().attr("title") {
                            let clean_title = attr_title.trim();
                            if !clean_title.is_empty() {
                                title = clean_title.to_string();
                            }
                        }
                    }

                    if is_generic_link_label(&title) {
                        title = format!("Disciplina #{}", course_id);
                    }

                    courses.push(Course {
                        id: course_id,
                        fullname: title,
                        shortname: format!("CURSO-{}", course_id),
                        summary: None,
                        last_synced_at: Some(Utc::now()),
                    });
                }
            }
        }
    }

    courses
}

fn is_generic_link_label(label: &str) -> bool {
    label.is_empty()
        || label.eq_ignore_ascii_case("visualizar")
        || label.eq_ignore_ascii_case("acessar")
}

/// Parses a Moodle Course Page (`/course/view.php?id=...`) to extract sections and activities.
pub fn parse_course_sections(html: &str, course_id: i64) -> Vec<Section> {
    let document = Html::parse_document(html);
    let section_selector = Selector::parse("li.section, div.section, .course-section").unwrap();
    let title_selector = Selector::parse(".sectionname, .section-title, h3, h4").unwrap();
    let section_summary_selector = Selector::parse("[data-for='sectioninfo'], .summary").unwrap();
    let li_activity_selector = Selector::parse("li.activity").unwrap();
    let fallback_activity_selector = Selector::parse(".activity-item").unwrap();

    let mut sections = Vec::new();
    let mut seen_section_ids = std::collections::HashSet::new();

    for (section_index, section_element) in document.select(&section_selector).enumerate() {
        let section_id = extract_section_id(&section_element, course_id, section_index);
        if !seen_section_ids.insert(section_id) {
            continue;
        }

        let section_name = section_element
            .value()
            .attr("data-sectionname")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                section_element
                    .select(&title_selector)
                    .next()
                    .map(|t| t.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| format!("Tópico {}", section_index));

        let section_summary = section_element
            .select(&section_summary_selector)
            .next()
            .map(|el| clean_html(&el.inner_html()))
            .filter(|s| !s.is_empty());

        let mut activities = Vec::new();
        let mut seen_activity_ids = std::collections::HashSet::new();

        for activity_element in section_element.select(&li_activity_selector) {
            if let Some(activity) = parse_activity_element(&activity_element, section_id) {
                if seen_activity_ids.insert(activity.id) {
                    activities.push(activity);
                }
            }
        }

        if activities.is_empty() {
            for activity_element in section_element.select(&fallback_activity_selector) {
                if let Some(activity) = parse_activity_element(&activity_element, section_id) {
                    if seen_activity_ids.insert(activity.id) {
                        activities.push(activity);
                    }
                }
            }
        }

        sections.push(Section {
            id: section_id,
            course_id,
            name: section_name,
            summary: section_summary,
            activities,
        });
    }

    if sections.is_empty() {
        let fallback_section_id = course_id * 10000;
        let mut general_activities = Vec::new();
        let mut seen_activity_ids = std::collections::HashSet::new();

        for activity_element in document.select(&li_activity_selector) {
            if let Some(activity) = parse_activity_element(&activity_element, fallback_section_id) {
                if seen_activity_ids.insert(activity.id) {
                    general_activities.push(activity);
                }
            }
        }

        if !general_activities.is_empty() {
            sections.push(Section {
                id: fallback_section_id,
                course_id,
                name: "Geral".to_string(),
                summary: None,
                activities: general_activities,
            });
        }
    }

    sections
}

fn extract_section_id(
    section_element: &scraper::ElementRef,
    course_id: i64,
    section_index: usize,
) -> i64 {
    if let Some(db_id) = section_element
        .value()
        .attr("data-id")
        .and_then(|s| s.parse::<i64>().ok())
    {
        return db_id;
    }

    let relative_num = section_element
        .value()
        .attr("id")
        .and_then(|id_str| id_str.strip_prefix("section-"))
        .and_then(|num_str| num_str.parse::<i64>().ok())
        .unwrap_or((section_index as i64) + 1);

    course_id * 10000 + relative_num
}

fn parse_activity_element(element: &scraper::ElementRef, section_id: i64) -> Option<Activity> {
    let link_selector = Selector::parse("a[href*='/mod/']").unwrap();
    let data_name_selector = Selector::parse("[data-activityname]").unwrap();
    let title_selector = Selector::parse(".instancename, .activityname").unwrap();
    let accesshide_selector = Selector::parse(".accesshide").unwrap();
    let description_selector =
        Selector::parse(".activity-description, .contentafterlink, .activity-altcontent").unwrap();

    let link_element = element.select(&link_selector).next()?;
    let href = link_element.value().attr("href")?;
    let activity_id = extract_id_from_url(href)?;

    let activity_type = extract_mod_type_from_url(href)
        .map(|mod_str| ActivityType::from_moodle_modname(&mod_str))
        .unwrap_or(ActivityType::Other("unknown".to_string()));

    let clean_title = element
        .value()
        .attr("data-activityname")
        .or_else(|| {
            element
                .select(&data_name_selector)
                .next()
                .and_then(|el| el.value().attr("data-activityname"))
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let title_el = element
                .select(&title_selector)
                .next()
                .unwrap_or(link_element);
            let raw_text = title_el.text().collect::<Vec<_>>().join(" ");
            let hidden_text = title_el
                .select(&accesshide_selector)
                .map(|h| h.text().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(" ");

            let stripped = if !hidden_text.trim().is_empty() {
                raw_text
                    .trim_end()
                    .strip_suffix(hidden_text.trim())
                    .unwrap_or(&raw_text)
            } else {
                &raw_text
            };
            let trimmed = stripped.split('\n').next().unwrap_or(stripped).trim();
            if trimmed.is_empty() {
                format!("Atividade {}", activity_id)
            } else {
                trimmed.to_string()
            }
        });

    let description = element
        .select(&description_selector)
        .next()
        .map(|desc_el| clean_html(&desc_el.inner_html()))
        .filter(|s| !s.is_empty());

    Some(Activity {
        id: activity_id,
        section_id,
        title: clean_title,
        activity_type,
        url: Some(href.to_string()),
        description,
        local_file_path: None,
        content_mime: None,
        due_date: None,
        is_submitted: false,
    })
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
