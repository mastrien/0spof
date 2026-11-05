use crate::auth::{session::parser::extract_sesskey, MoodleClient};
use crate::crawler::parser;
use crate::error::{Result, SpofError};
use crate::storage::Database;
use reqwest::header::{CONTENT_TYPE, COOKIE};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncSummary {
    pub courses_synced: usize,
    pub sections_synced: usize,
    pub activities_synced: usize,
}

/// Orquestrador de raspagem e sincronização offline de cursos e atividades.
pub struct CourseCrawler {
    db: Database,
    client: MoodleClient,
}

impl CourseCrawler {
    pub fn new(db: Database, client: MoodleClient) -> Self {
        Self { db, client }
    }

    /// Processa o HTML de um Dashboard diretamente e persiste os cursos identificados no banco.
    pub fn sync_from_dashboard_html(&self, html: &str) -> Result<usize> {
        let courses = parser::parse_dashboard_courses(html);
        for course in &courses {
            self.db.upsert_course(course)?;
        }
        Ok(courses.len())
    }

    /// Processa o JSON do webservice AJAX do Moodle 4.x e persiste os cursos identificados no banco.
    pub fn sync_from_ajax_courses_json(&self, json_str: &str) -> Result<usize> {
        let courses = parser::parse_ajax_enrolled_courses(json_str);
        for course in &courses {
            self.db.upsert_course(course)?;
        }
        Ok(courses.len())
    }

    /// Processa o HTML de uma página de curso diretamente e persiste seções e atividades no banco.
    pub fn sync_from_course_html(&self, course_id: i64, html: &str) -> Result<(usize, usize)> {
        let sections = parser::parse_course_sections(html, course_id);
        let mut total_acts = 0;

        for section in &sections {
            self.db.upsert_section(section)?;
            for act in &section.activities {
                self.db.upsert_activity(act)?;
                total_acts += 1;
            }
        }

        Ok((sections.len(), total_acts))
    }

    /// Executa a sincronização completa varrendo o dashboard (AJAX Moodle 4.x + HTML fallback) e todas as disciplinas online.
    pub async fn sync_all(&self) -> Result<SyncSummary> {
        let session = self.client.session().ok_or_else(|| {
            SpofError::Authentication("Sessão ausente para sincronização".to_string())
        })?;

        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| SpofError::Network(format!("Falha ao construir cliente HTTP: {}", e)))?;

        let candidate_endpoints = [
            format!("{}/my/courses.php", self.client.base_url()),
            format!("{}/my/", self.client.base_url()),
            format!("{}/", self.client.base_url()),
        ];

        let mut all_courses = Vec::new();
        let mut redirected_to_login = false;
        let mut successful_responses = 0;
        let mut discovered_sesskey = session.sesskey.clone();

        for endpoint in &candidate_endpoints {
            if let Ok(resp) = http
                .get(endpoint)
                .header(COOKIE, session.cookie_header())
                .send()
                .await
            {
                let final_url = resp.url().as_str().to_string();
                if final_url.contains("/login/") || final_url.contains("login.php") {
                    redirected_to_login = true;
                    continue;
                }
                successful_responses += 1;
                if let Ok(html) = resp.text().await {
                    if discovered_sesskey.is_none() {
                        discovered_sesskey = extract_sesskey(&html);
                    }
                    let mut found = parser::parse_dashboard_courses(&html);
                    all_courses.append(&mut found);
                }
                if !all_courses.is_empty() || discovered_sesskey.is_some() {
                    break;
                }
            }
        }

        if successful_responses == 0 && redirected_to_login {
            return Err(SpofError::Authentication(
                "Sessão inválida ou expirada: o Moodle redirecionou a requisição para a tela de login. Por favor, atualize o cookie MoodleSession.".to_string(),
            ));
        }

        // Moodle 4.x loads enrolled courses dynamically via AJAX using sesskey
        if let Some(ref sesskey) = discovered_sesskey {
            let ajax_url = format!(
                "{}/lib/ajax/service.php?sesskey={}&info=core_course_get_enrolled_courses_by_timeline_classification",
                self.client.base_url(),
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
                    let mut ajax_courses = parser::parse_ajax_enrolled_courses(&ajax_json);
                    all_courses.append(&mut ajax_courses);
                }
            }
        }

        // Deduplicate courses by id
        let mut seen = std::collections::HashSet::new();
        let mut courses_count = 0;
        for course in all_courses {
            if seen.insert(course.id) {
                self.db.upsert_course(&course)?;
                courses_count += 1;
            }
        }

        let courses = self.db.get_courses()?;
        let mut total_sections = 0;
        let mut total_acts = 0;

        for course in courses {
            let course_url = format!(
                "{}/course/view.php?id={}",
                self.client.base_url(),
                course.id
            );
            if let Ok(c_resp) = http
                .get(&course_url)
                .header(COOKIE, session.cookie_header())
                .send()
                .await
            {
                if let Ok(c_html) = c_resp.text().await {
                    if let Ok((secs, acts)) = self.sync_from_course_html(course.id, &c_html) {
                        total_sections += secs;
                        total_acts += acts;
                    }
                }
            }
        }

        Ok(SyncSummary {
            courses_synced: courses_count,
            sections_synced: total_sections,
            activities_synced: total_acts,
        })
    }
}

#[cfg(test)]
mod tests {
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
}
