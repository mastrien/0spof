use crate::auth::{session::parser::extract_sesskey, MoodleClient, MoodleSession};
use crate::crawler::ajax::{fetch_ajax_courses, is_session_rejected};
use crate::crawler::{assignment_parser, parser, resource};
use crate::error::{Result, SpofError};
use crate::models::{Activity, ActivityType};
use crate::storage::Database;
use reqwest::header::COOKIE;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncSummary {
    pub courses_synced: usize,
    pub sections_synced: usize,
    pub activities_synced: usize,
    #[serde(default)]
    pub resources_cached: usize,
}

fn default_cache_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("SPOF_CACHE_DIR") {
        return PathBuf::from(custom);
    }
    if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_appdata).join("0spof").join("resources");
    }
    PathBuf::from("0spof_cache")
}

/// Orquestrador de raspagem e sincronização offline de cursos, atividades e recursos.
pub struct CourseCrawler {
    db: Database,
    client: MoodleClient,
    cache_dir: PathBuf,
}

impl CourseCrawler {
    pub fn new(db: Database, client: MoodleClient) -> Self {
        Self {
            db,
            client,
            cache_dir: default_cache_dir(),
        }
    }

    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = cache_dir;
        self
    }

    pub fn sync_from_dashboard_html(&self, html: &str) -> Result<usize> {
        let courses = parser::parse_dashboard_courses(html);
        for course in &courses {
            self.db.upsert_course(course)?;
        }
        Ok(courses.len())
    }

    pub fn sync_from_ajax_courses_json(&self, json_str: &str) -> Result<usize> {
        let courses = parser::parse_ajax_enrolled_courses(json_str);
        for course in &courses {
            self.db.upsert_course(course)?;
        }
        Ok(courses.len())
    }

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

    pub async fn download_activity(&self, activity_id: i64, allow_video: bool) -> Result<String> {
        let session = self
            .client
            .session()
            .ok_or_else(|| SpofError::Authentication("Sessão ausente para download".to_string()))?;
        let activity = self.db.get_activity(activity_id)?.ok_or_else(|| {
            SpofError::Validation(format!("Atividade #{} não encontrada", activity_id))
        })?;
        let url = activity.url.as_deref().ok_or_else(|| {
            SpofError::Validation("Atividade não possui URL associada".to_string())
        })?;

        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| SpofError::Network(format!("Falha ao construir cliente HTTP: {}", e)))?;

        if let Some((path, mime)) = resource::download_activity_resource(
            &http,
            session,
            activity.id,
            &activity.title,
            url,
            &self.cache_dir,
            allow_video,
        )
        .await?
        {
            let path_str = path.to_string_lossy().to_string();
            self.db
                .update_activity_local_file(activity.id, &path_str, &mime)?;
            Ok(path_str)
        } else {
            Err(SpofError::Validation(
                "Não foi possível baixar o conteúdo deste recurso".to_string(),
            ))
        }
    }

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
                    let mut courses = parser::parse_dashboard_courses(&html);
                    all_courses.append(&mut courses);
                }
            }
        }

        if is_session_rejected(redirected_to_login, successful_responses, all_courses.len()) {
            return Err(SpofError::Authentication(
                "Sessão do Moodle rejeitada ou expirada".to_string(),
            ));
        }

        if let Some(sesskey) = discovered_sesskey {
            let mut ajax_courses =
                fetch_ajax_courses(&http, self.client.base_url(), session, &sesskey).await;
            all_courses.append(&mut ajax_courses);
        }

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
        let mut resources_cached = 0;

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

            if let Ok(sections) = self.db.get_sections_by_course(course.id) {
                for section in sections {
                    for act in section.activities {
                        resources_cached += self.enrich_activity(&http, session, &act).await;
                    }
                }
            }
        }

        Ok(SyncSummary {
            courses_synced: courses_count,
            sections_synced: total_sections,
            activities_synced: total_acts,
            resources_cached,
        })
    }

    async fn enrich_activity(
        &self,
        http: &reqwest::Client,
        session: &MoodleSession,
        act: &Activity,
    ) -> usize {
        let Some(ref act_url) = act.url else {
            return 0;
        };

        if act.activity_type == ActivityType::Assign {
            if let Ok(resp) = http
                .get(act_url)
                .header(COOKIE, session.cookie_header())
                .send()
                .await
            {
                if let Ok(assign_html) = resp.text().await {
                    let details = assignment_parser::parse_assignment_details(&assign_html, act.id);
                    let _ = self.db.update_activity_submission_details(
                        act.id,
                        details.is_submitted,
                        details.due_date,
                    );
                    if let Some(ref intro) = details.description_html {
                        let _ = self.db.update_activity_description_if_empty(act.id, intro);
                    }
                }
            }
            return 0;
        }

        if act.activity_type == ActivityType::Url {
            if act_url.contains("/mod/url/view.php") {
                if let Ok(Some(direct_url)) =
                    resource::resolve_external_url(http, session, act_url).await
                {
                    let _ = self.db.update_activity_url(act.id, &direct_url);
                }
            }
            return 0;
        }

        let should_auto_cache = matches!(
            act.activity_type,
            ActivityType::Resource | ActivityType::Folder
        ) || matches!(&act.activity_type, ActivityType::Other(m) if m == "page" || m == "book");
        if !should_auto_cache {
            return 0;
        }

        if let Some(ref existing_path) = act.local_file_path {
            if !existing_path.is_empty() && Path::new(existing_path).exists() {
                return 0;
            }
        }

        if let Ok(Some((path, mime))) = resource::download_activity_resource(
            http,
            session,
            act.id,
            &act.title,
            act_url,
            &self.cache_dir,
            false,
        )
        .await
        {
            let path_str = path.to_string_lossy().to_string();
            let _ = self.db.update_activity_local_file(act.id, &path_str, &mime);
            if !path_str.is_empty() {
                return 1;
            }
        }
        0
    }
}

#[path = "sync_tests.rs"]
#[cfg(test)]
mod tests;
