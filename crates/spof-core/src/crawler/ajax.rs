use crate::auth::MoodleSession;
use crate::models::Course;
use chrono::Utc;
use reqwest::header::{CONTENT_TYPE, COOKIE};
use serde_json::{json, Value};

pub(crate) fn is_session_rejected(
    redirected_to_login: bool,
    successful_responses: usize,
    courses_found: usize,
) -> bool {
    redirected_to_login && (successful_responses == 0 || courses_found == 0)
}

pub async fn fetch_ajax_courses(
    http: &reqwest::Client,
    base_url: &str,
    session: &MoodleSession,
    sesskey: &str,
) -> Vec<Course> {
    let ajax_url = format!(
        "{}/lib/ajax/service.php?sesskey={}&info=core_course_get_enrolled_courses_by_timeline_classification",
        base_url,
        sesskey
    );
    let ajax_payload = json!([{
        "index": 0,
        "methodname": "core_course_get_enrolled_courses_by_timeline_classification",
        "args": {
            "offset": 0,
            "limit": 0,
            "classification": "all",
            "sort": "fullname",
            "customfieldname": "",
            "customfieldvalue": ""
        }
    }]);

    if let Ok(ajax_resp) = http
        .post(&ajax_url)
        .header(COOKIE, session.cookie_header())
        .header(CONTENT_TYPE, "application/json")
        .body(ajax_payload.to_string())
        .send()
        .await
    {
        if let Ok(ajax_json) = ajax_resp.text().await {
            return parse_ajax_enrolled_courses(&ajax_json);
        }
    }
    Vec::new()
}

/// Parses the Moodle 4.x AJAX response from `core_course_get_enrolled_courses_by_timeline_classification`.
pub fn parse_ajax_enrolled_courses(json_str: &str) -> Vec<Course> {
    let Ok(parsed_value) = serde_json::from_str::<Value>(json_str) else {
        return Vec::new();
    };

    let mut courses = Vec::new();
    let mut seen_course_ids = std::collections::HashSet::new();

    let items = if let Some(arr) = parsed_value.as_array() {
        arr.as_slice()
    } else {
        std::slice::from_ref(&parsed_value)
    };

    for response_item in items {
        if response_item.get("error").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        if let Some(course_list) = response_item
            .get("data")
            .and_then(|data| data.get("courses"))
            .and_then(Value::as_array)
        {
            for course_obj in course_list {
                let Some(course_id) = course_obj.get("id").and_then(Value::as_i64) else {
                    continue;
                };
                if !seen_course_ids.insert(course_id) {
                    continue;
                }
                let fullname = course_obj
                    .get("fullname")
                    .and_then(Value::as_str)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("Disciplina #{}", course_id));

                let shortname = course_obj
                    .get("shortname")
                    .and_then(Value::as_str)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("CURSO-{}", course_id));

                let summary = course_obj
                    .get("summary")
                    .and_then(Value::as_str)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                courses.push(Course {
                    id: course_id,
                    fullname,
                    shortname,
                    summary,
                    last_synced_at: Some(Utc::now()),
                });
            }
        }
    }

    courses
}
