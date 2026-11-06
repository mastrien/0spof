use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use scraper::{Html, Selector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentDetails {
    pub assign_id: i64,
    pub title: String,
    pub is_submitted: bool,
    pub due_date: Option<DateTime<Utc>>,
    pub time_remaining: Option<String>,
    pub description_html: Option<String>,
}

/// Parses the assignment details page (`/mod/assign/view.php?id=...`).
pub fn parse_assignment_details(html: &str, assign_id: i64) -> AssignmentDetails {
    let document = Html::parse_document(html);
    let title_selector =
        Selector::parse("h2, .page-header-headings h1, .activity-header h2").unwrap();

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
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("Tarefa #{}", assign_id));

    let mut is_submitted = false;
    let mut due_date = None;
    let mut time_remaining = None;

    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("th, td").unwrap();

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        if cells.len() >= 2 {
            let label = cells[0].text().collect::<Vec<_>>().join(" ").to_lowercase();
            let value = cells[1]
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let value_lower = value.to_lowercase();

            if label.contains("status") && (label.contains("envio") || label.contains("submiss")) {
                if is_positive_submission(&value_lower) {
                    is_submitted = true;
                } else if is_negative_submission(&value_lower) {
                    is_submitted = false;
                }
            } else if label.contains("entrega")
                || label.contains("limite")
                || label.contains("due date")
            {
                if let Some(parsed) = parse_moodle_date(&value) {
                    due_date = Some(parsed);
                }
            } else if (label.contains("tempo restante") || label.contains("time remaining"))
                && !value.is_empty()
            {
                time_remaining = Some(value);
            }
        }
    }

    if !is_submitted {
        let submitted_class =
            Selector::parse(".submissionstatussubmitted, .submissiongraded").unwrap();
        if document.select(&submitted_class).next().is_some() {
            is_submitted = true;
        } else {
            let body_text = document.root_element().text().collect::<Vec<_>>().join(" ");
            if body_text.contains("Submetido para avaliação")
                || body_text.contains("Submitted for grading")
                || body_text.contains("Envio concluído")
            {
                is_submitted = true;
            }
        }
    }

    let intro_selector =
        Selector::parse("#intro, .intro, .activity-description, .generalbox#intro").unwrap();
    let description_html = document
        .select(&intro_selector)
        .next()
        .map(|el| clean_html(&el.inner_html()))
        .filter(|s| !s.is_empty());

    AssignmentDetails {
        assign_id,
        title,
        is_submitted,
        due_date,
        time_remaining,
        description_html,
    }
}

fn is_positive_submission(text: &str) -> bool {
    (text.contains("submetido") || text.contains("submitted") || text.contains("concluído"))
        && !is_negative_submission(text)
}

fn is_negative_submission(text: &str) -> bool {
    text.contains("nenhum")
        || text.contains("no attempt")
        || text.contains("não enviado")
        || text.contains("rascunho")
        || text.contains("draft")
        || text.contains("não submetido")
}

/// Cleans raw HTML while preserving formatting tags and links.
pub fn clean_html(raw: &str) -> String {
    let mut cleaned = raw.trim().to_string();
    if cleaned.is_empty() {
        return String::new();
    }
    cleaned = strip_tag_block(&cleaned, "script");
    cleaned = strip_tag_block(&cleaned, "style");
    cleaned.trim().to_string()
}

fn strip_tag_block(input: &str, tag: &str) -> String {
    let mut result = input.to_string();
    let open_tag = format!("<{}", tag);
    let close_tag = format!("</{}>", tag);

    while let Some(start) = result.to_lowercase().find(&open_tag) {
        if let Some(end_rel) = result[start..].to_lowercase().find(&close_tag) {
            let end = start + end_rel + close_tag.len();
            result.replace_range(start..end, "");
        } else {
            break;
        }
    }
    result
}

/// Parses Moodle date formats (PT-BR, EN, ISO) into `DateTime<Utc>`.
pub fn parse_moodle_date(raw: &str) -> Option<DateTime<Utc>> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(cleaned) {
        return Some(dt.with_timezone(&Utc));
    }

    parse_ptbr_date(cleaned)
}

fn parse_ptbr_date(raw: &str) -> Option<DateTime<Utc>> {
    let lower = raw.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "de" && *s != "-")
        .collect();

    let mut found_day = None;
    let mut found_month = None;
    let mut found_year = None;
    let mut found_hour = None;
    let mut found_min = None;

    for token in tokens {
        if token.contains('/') {
            let parts: Vec<&str> = token.split('/').collect();
            if parts.len() == 3 {
                found_day = parts[0].parse::<u32>().ok();
                found_month = parts[1].parse::<u32>().ok();
                found_year = parts[2].parse::<i32>().ok();
            }
        } else if token.contains(':') {
            let time_parts: Vec<&str> = token.split(':').collect();
            if time_parts.len() >= 2 {
                found_hour = time_parts[0].parse::<u32>().ok();
                found_min = time_parts[1].parse::<u32>().ok();
            }
        } else if let Ok(num) = token.parse::<i32>() {
            if (2000..=2100).contains(&num) {
                found_year = Some(num);
            } else if (1..=31).contains(&num) && found_day.is_none() {
                found_day = Some(num as u32);
            }
        } else if let Some(m) = match_month_name(token) {
            found_month = Some(m);
        }
    }

    let year = found_year?;
    let month = found_month?;
    let day = found_day?;
    let hour = found_hour.unwrap_or(23);
    let min = found_min.unwrap_or(59);

    make_utc_datetime(year, month, day, hour, min)
}

fn make_utc_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
) -> Option<DateTime<Utc>> {
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let naive_dt = date.and_hms_opt(hour, min, 0)?;
    let tz_offset = FixedOffset::west_opt(3 * 3600)?;
    let local_dt = tz_offset.from_local_datetime(&naive_dt).single()?;
    Some(local_dt.with_timezone(&Utc))
}

fn match_month_name(name: &str) -> Option<u32> {
    let n = name.trim_end_matches('.');
    if n.starts_with("jan") {
        Some(1)
    } else if n.starts_with("fev") || n.starts_with("feb") {
        Some(2)
    } else if n.starts_with("mar") {
        Some(3)
    } else if n.starts_with("abr") || n.starts_with("apr") {
        Some(4)
    } else if n.starts_with("mai") || n.starts_with("may") {
        Some(5)
    } else if n.starts_with("jun") {
        Some(6)
    } else if n.starts_with("jul") {
        Some(7)
    } else if n.starts_with("ago") || n.starts_with("aug") {
        Some(8)
    } else if n.starts_with("set") || n.starts_with("sep") {
        Some(9)
    } else if n.starts_with("out") || n.starts_with("oct") {
        Some(10)
    } else if n.starts_with("nov") {
        Some(11)
    } else if n.starts_with("dez") || n.starts_with("dec") {
        Some(12)
    } else {
        None
    }
}

#[path = "assignment_parser_tests.rs"]
#[cfg(test)]
mod tests;
