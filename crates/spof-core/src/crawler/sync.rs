use crate::auth::{session::parser::extract_sesskey, MoodleClient};
use crate::crawler::{parser, resource};
use crate::error::{Result, SpofError};
use crate::models::ActivityType;
use crate::storage::Database;
use reqwest::header::{CONTENT_TYPE, COOKIE};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncSummary {
    pub courses_synced: usize,
    pub sections_synced: usize,
    pub activities_synced: usize,
    #[serde(default)]
    pub resources_cached: usize,
}

/// A sessão é considerada expirada quando alguma página autenticada (`/my/`) redirecionou
/// para o login e nenhum curso foi encontrado — a página raiz pública não conta como sucesso.
pub(crate) fn is_session_rejected(
    redirected_to_login: bool,
    successful_responses: usize,
    courses_found: usize,
) -> bool {
    redirected_to_login && (successful_responses == 0 || courses_found == 0)
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

    /// Faz o download sob demanda (incluindo vídeos se `allow_video = true`) de uma atividade específica.
    pub async fn download_activity(&self, activity_id: i64, allow_video: bool) -> Result<String> {
        let session = self.client.session().ok_or_else(|| {
            SpofError::Authentication("Sessão ausente para download de recurso".to_string())
        })?;
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

    /// Executa a sincronização completa varrendo o dashboard, disciplinas e cacheando PDFs/imagens/HTML.
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

        // Tenta buscar via AJAX do Moodle 4.x se tiver sesskey
        if let Some(sesskey) = discovered_sesskey {
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

            // Auto-cache resources & resolve direct external links
            if let Ok(sections) = self.db.get_sections_by_course(course.id) {
                for section in sections {
                    for act in section.activities {
                        if act.activity_type == ActivityType::Url {
                            if let Some(ref act_url) = act.url {
                                if act_url.contains("/mod/url/view.php") {
                                    if let Ok(Some(direct_url)) =
                                        resource::resolve_external_url(&http, session, act_url)
                                            .await
                                    {
                                        let _ = self.db.update_activity_url(act.id, &direct_url);
                                    }
                                }
                            }
                            continue;
                        }

                        let should_auto_cache = matches!(
                            act.activity_type,
                            ActivityType::Resource | ActivityType::Folder
                        ) || matches!(
                            &act.activity_type,
                            ActivityType::Other(m) if m == "page" || m == "book"
                        );
                        if !should_auto_cache {
                            continue;
                        }
                        if let Some(ref existing_path) = act.local_file_path {
                            if !existing_path.is_empty() && Path::new(existing_path).exists() {
                                continue;
                            }
                        }
                        if let Some(ref act_url) = act.url {
                            if let Ok(Some((path, mime))) = resource::download_activity_resource(
                                &http,
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
                                let _ =
                                    self.db.update_activity_local_file(act.id, &path_str, &mime);
                                if !path_str.is_empty() {
                                    resources_cached += 1;
                                }
                            }
                        }
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
}

#[path = "sync_tests.rs"]
#[cfg(test)]
mod tests;
