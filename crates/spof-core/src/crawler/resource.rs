use crate::auth::MoodleSession;
use crate::error::{Result, SpofError};
use reqwest::header::{CONTENT_TYPE, COOKIE};
use scraper::{Html, Selector};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaClassification {
    AutoDownload { mime: String, extension: String },
    ExplicitVideo { mime: String, extension: String },
    Other { mime: String, extension: String },
}

pub fn classify_content_type(content_type_header: &str, url: &str) -> MediaClassification {
    let lower_mime = content_type_header
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let lower_url = url.to_ascii_lowercase();

    if lower_mime == "application/pdf" || lower_url.ends_with(".pdf") {
        return MediaClassification::AutoDownload {
            mime: "application/pdf".to_string(),
            extension: "pdf".to_string(),
        };
    }

    if lower_mime.starts_with("image/") {
        let ext = match lower_mime.as_str() {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            _ => "img",
        };
        return MediaClassification::AutoDownload {
            mime: lower_mime,
            extension: ext.to_string(),
        };
    }

    if lower_mime == "text/html" || lower_mime == "application/xhtml+xml" {
        return MediaClassification::AutoDownload {
            mime: "text/html".to_string(),
            extension: "html".to_string(),
        };
    }

    if lower_mime.starts_with("video/")
        || lower_url.ends_with(".mp4")
        || lower_url.ends_with(".webm")
        || lower_url.ends_with(".mkv")
    {
        let ext = if lower_mime.contains("webm") || lower_url.ends_with(".webm") {
            "webm"
        } else if lower_url.ends_with(".mkv") {
            "mkv"
        } else {
            "mp4"
        };
        let mime = if lower_mime.starts_with("video/") {
            lower_mime
        } else {
            format!("video/{}", ext)
        };
        return MediaClassification::ExplicitVideo {
            mime,
            extension: ext.to_string(),
        };
    }

    MediaClassification::Other {
        mime: if lower_mime.is_empty() {
            "application/octet-stream".to_string()
        } else {
            lower_mime
        },
        extension: "bin".to_string(),
    }
}

pub fn extract_pluginfile_url(html: &str) -> Option<(String, bool)> {
    let document = Html::parse_document(html);

    if let Ok(video_sel) = Selector::parse("video source[src], video[src]") {
        if let Some(el) = document.select(&video_sel).next() {
            if let Some(src) = el.value().attr("src") {
                return Some((src.to_string(), true));
            }
        }
    }

    if let Ok(link_sel) = Selector::parse(
        ".resourceworkaround a[href], .resourcecontent a[href], a[href*='pluginfile.php'], object[data*='pluginfile.php']",
    ) {
        if let Some(el) = document.select(&link_sel).next() {
            if let Some(href) = el.value().attr("href").or_else(|| el.value().attr("data")) {
                let lower = href.to_ascii_lowercase();
                let is_video =
                    lower.ends_with(".mp4") || lower.ends_with(".webm") || lower.ends_with(".mkv");
                return Some((href.to_string(), is_video));
            }
        }
    }

    None
}

pub fn sanitize_filename(title: &str, activity_id: i64, extension: &str) -> String {
    let safe_chars: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(50)
        .collect();
    let trimmed = safe_chars.trim_matches('_');
    let base = if trimmed.is_empty() {
        format!("resource_{}", activity_id)
    } else {
        format!("{}_{}", activity_id, trimmed)
    };
    format!("{}.{}", base, extension)
}

pub async fn download_activity_resource(
    http: &reqwest::Client,
    session: &MoodleSession,
    activity_id: i64,
    activity_title: &str,
    activity_url: &str,
    cache_dir: &Path,
    allow_video: bool,
) -> Result<Option<(PathBuf, String)>> {
    let mut target_url = activity_url.to_string();
    if target_url.contains("/mod/resource/view.php") && !target_url.contains("redirect=1") {
        let sep = if target_url.contains('?') { "&" } else { "?" };
        target_url = format!("{}{}redirect=1", target_url, sep);
    }

    let resp = http
        .get(&target_url)
        .header(COOKIE, session.cookie_header())
        .send()
        .await
        .map_err(|e| SpofError::Network(format!("Erro ao baixar recurso: {}", e)))?;

    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let final_url = resp.url().as_str().to_string();

    if final_url.contains("/login/") || final_url.contains("login.php") {
        return Err(SpofError::Authentication(
            "Sessão expirada ao acessar recurso".to_string(),
        ));
    }

    let classification = classify_content_type(&content_type, &final_url);

    match classification {
        MediaClassification::ExplicitVideo { mime, extension } => {
            if !allow_video {
                return Ok(Some((PathBuf::new(), mime)));
            }
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| SpofError::Network(e.to_string()))?;
            std::fs::create_dir_all(cache_dir)?;
            let file_path =
                cache_dir.join(sanitize_filename(activity_title, activity_id, &extension));
            std::fs::write(&file_path, &bytes)?;
            Ok(Some((file_path, mime)))
        }
        MediaClassification::AutoDownload { mime, extension } => {
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| SpofError::Network(e.to_string()))?;

            if mime == "text/html" {
                let html_text = String::from_utf8_lossy(&bytes);
                if let Some((inner_url, is_video)) = extract_pluginfile_url(&html_text) {
                    if is_video && !allow_video {
                        return Ok(Some((PathBuf::new(), "video/mp4".to_string())));
                    }
                    if inner_url != final_url && inner_url != target_url {
                        return Box::pin(download_activity_resource(
                            http,
                            session,
                            activity_id,
                            activity_title,
                            &inner_url,
                            cache_dir,
                            allow_video,
                        ))
                        .await;
                    }
                }
            }

            std::fs::create_dir_all(cache_dir)?;
            let file_path =
                cache_dir.join(sanitize_filename(activity_title, activity_id, &extension));
            std::fs::write(&file_path, &bytes)?;
            Ok(Some((file_path, mime)))
        }
        MediaClassification::Other { mime, extension } => {
            if allow_video {
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| SpofError::Network(e.to_string()))?;
                std::fs::create_dir_all(cache_dir)?;
                let file_path =
                    cache_dir.join(sanitize_filename(activity_title, activity_id, &extension));
                std::fs::write(&file_path, &bytes)?;
                Ok(Some((file_path, mime)))
            } else {
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_content_type() {
        assert_eq!(
            classify_content_type("application/pdf; charset=binary", "https://moodle/file"),
            MediaClassification::AutoDownload {
                mime: "application/pdf".to_string(),
                extension: "pdf".to_string()
            }
        );
        assert_eq!(
            classify_content_type("image/png", "https://moodle/img.png"),
            MediaClassification::AutoDownload {
                mime: "image/png".to_string(),
                extension: "png".to_string()
            }
        );
        assert_eq!(
            classify_content_type("video/mp4", "https://moodle/aula.mp4"),
            MediaClassification::ExplicitVideo {
                mime: "video/mp4".to_string(),
                extension: "mp4".to_string()
            }
        );
    }

    #[test]
    fn test_extract_pluginfile_url() {
        let html = r#"
            <div class="resourceworkaround">
                Clique em <a href="https://moodle/pluginfile.php/123/mod_resource/content/1/slides.pdf">slides.pdf</a>
            </div>
        "#;
        assert_eq!(
            extract_pluginfile_url(html),
            Some((
                "https://moodle/pluginfile.php/123/mod_resource/content/1/slides.pdf".to_string(),
                false
            ))
        );

        let video_html = r#"<video><source src="https://moodle/pluginfile.php/55/video.mp4" type="video/mp4"></video>"#;
        assert_eq!(
            extract_pluginfile_url(video_html),
            Some((
                "https://moodle/pluginfile.php/55/video.mp4".to_string(),
                true
            ))
        );
    }
}
