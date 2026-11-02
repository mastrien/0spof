use crate::auth::MoodleClient;
use crate::crawler::parser;
use crate::error::{Result, SpofError};
use crate::storage::Database;
use reqwest::header::COOKIE;
use serde::{Deserialize, Serialize};

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

    /// Executa a sincronização completa varrendo o dashboard e todas as disciplinas online.
    pub async fn sync_all(&self) -> Result<SyncSummary> {
        let session = self.client.session().ok_or_else(|| {
            SpofError::Authentication("Sessão ausente para sincronização".to_string())
        })?;

        let dashboard_url = format!("{}/my/", self.client.base_url());
        let http = reqwest::Client::new();

        let resp = http
            .get(&dashboard_url)
            .header(COOKIE, session.cookie_header())
            .send()
            .await
            .map_err(|e| SpofError::Network(format!("Falha ao requisitar dashboard: {}", e)))?;

        let dashboard_html = resp
            .text()
            .await
            .map_err(|e| SpofError::Network(format!("Falha ao ler dashboard: {}", e)))?;

        let courses_count = self.sync_from_dashboard_html(&dashboard_html)?;
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
        let client = MoodleClient::new("https://moodle.edu.br".to_string()).unwrap();
        let crawler = CourseCrawler::new(db.clone(), client);

        let dashboard_html = r#"
            <a href="https://moodle.edu.br/course/view.php?id=77" class="coursename">Redes de Computadores</a>
        "#;

        let count = crawler.sync_from_dashboard_html(dashboard_html).unwrap();
        assert_eq!(count, 1);

        let courses = db.get_courses().unwrap();
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].id, 77);

        let course_html = r#"
            <ul class="topics">
                <li id="section-10" class="section">
                    <span class="sectionname">Camada de Aplicação</span>
                    <ul class="section img-text">
                        <li id="module-1" class="activity assign modtype_assign">
                            <a href="/mod/assign/view.php?id=1"><span class="instancename">Exercício DNS</span></a>
                        </li>
                    </ul>
                </li>
            </ul>
        "#;

        let (secs, acts) = crawler.sync_from_course_html(77, course_html).unwrap();
        assert_eq!(secs, 1);
        assert_eq!(acts, 1);

        let saved_acts = db.get_activities_by_section(10).unwrap();
        assert_eq!(saved_acts.len(), 1);
        assert_eq!(saved_acts[0].title, "Exercício DNS");
    }
}
