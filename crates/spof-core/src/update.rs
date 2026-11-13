use crate::error::{Result, SpofError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const DEFAULT_RELEASES_API_URL: &str =
    "https://api.github.com/repos/mastrien/0spof/releases/latest";
pub const DEFAULT_RELEASES_PAGE_URL: &str = "https://github.com/mastrien/0spof/releases";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    pub release_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubReleasePayload {
    tag_name: String,
    html_url: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Parses a semantic version string (e.g., `v0.1.0`, `0.2.1-beta`) into `(major, minor, patch)`.
pub fn parse_semver_tuple(version: &str) -> Option<(u64, u64, u64)> {
    let clean = version.trim().trim_start_matches(['v', 'V']);
    let core = clean.split(['-', '+']).next()?.trim();
    if core.is_empty() {
        return None;
    }

    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Some((major, minor, patch))
}

/// Returns `true` when `latest` represents a strictly newer version than `current`.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver_tuple(current), parse_semver_tuple(latest)) {
        (Some(cur), Some(lat)) => {
            if lat != cur {
                lat > cur
            } else {
                // Stable release (no `-` suffix) supersedes pre-release of same tuple.
                let cur_pre = current.trim().contains('-');
                let lat_pre = latest.trim().contains('-');
                cur_pre && !lat_pre
            }
        }
        _ => false,
    }
}

/// Parses a GitHub Releases API JSON response and compares it against `current_version`.
pub fn parse_github_release_json(
    current_version: &str,
    json_body: &str,
) -> Result<UpdateCheckResult> {
    let payload: GithubReleasePayload = serde_json::from_str(json_body)?;
    let normalized_current = normalize_tag(current_version);
    let normalized_latest = normalize_tag(&payload.tag_name);

    let update_available = !payload.draft
        && !payload.prerelease
        && is_newer_version(current_version, &payload.tag_name);

    let release_url = payload
        .html_url
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASES_PAGE_URL.to_string());

    let release_notes = payload.body.and_then(|b| {
        let trimmed = b.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    Ok(UpdateCheckResult {
        current_version: normalized_current,
        latest_version: normalized_latest,
        update_available,
        release_url,
        release_notes,
    })
}

fn normalize_tag(version: &str) -> String {
    let trimmed = version.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        "0.0.0".to_string()
    } else {
        format!("v{trimmed}")
    }
}

/// Queries GitHub Releases (`releases/latest`) to check whether a newer version is available.
pub async fn check_for_updates(current_version: &str) -> Result<UpdateCheckResult> {
    check_for_updates_from(current_version, DEFAULT_RELEASES_API_URL).await
}

/// Queries a specific GitHub Releases API URL to check for updates.
pub async fn check_for_updates_from(
    current_version: &str,
    api_url: &str,
) -> Result<UpdateCheckResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(format!("0spof/{current_version}"))
        .build()
        .map_err(|e| SpofError::Network(format!("Falha ao iniciar cliente HTTP: {e}")))?;

    let response = client
        .get(api_url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| SpofError::Network(format!("Falha ao consultar atualizações: {e}")))?;

    if !response.status().is_success() {
        return Err(SpofError::Network(format!(
            "GitHub Releases retornou status {}",
            response.status()
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| SpofError::Network(format!("Falha ao ler resposta de atualização: {e}")))?;

    parse_github_release_json(current_version, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver_tuple_variants() {
        assert_eq!(parse_semver_tuple("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_semver_tuple("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver_tuple("V2.10.4-rc.1+build5"), Some((2, 10, 4)));
        assert_eq!(parse_semver_tuple("1.5"), Some((1, 5, 0)));
        assert_eq!(parse_semver_tuple(""), None);
        assert_eq!(parse_semver_tuple("v"), None);
        assert_eq!(parse_semver_tuple("1.2.3.4"), None);
        assert_eq!(parse_semver_tuple("abc"), None);
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "v0.2.0"));
        assert!(is_newer_version("0.1.9", "v1.0.0"));
        assert!(is_newer_version("0.2.0-rc.1", "v0.2.0"));

        assert!(!is_newer_version("0.1.0", "v0.1.0"));
        assert!(!is_newer_version("v0.2.0", "0.1.9"));
        assert!(!is_newer_version("0.1.0", "invalid"));
    }

    #[test]
    fn test_parse_github_release_json_update_available() {
        let json = r#"{
            "tag_name": "v0.2.0",
            "html_url": "https://github.com/mastrien/0spof/releases/tag/v0.2.0",
            "body": "Melhorias de estabilidade",
            "draft": false,
            "prerelease": false
        }"#;

        let res = parse_github_release_json("0.1.0", json).expect("should parse");
        assert_eq!(res.current_version, "v0.1.0");
        assert_eq!(res.latest_version, "v0.2.0");
        assert!(res.update_available);
        assert_eq!(
            res.release_url,
            "https://github.com/mastrien/0spof/releases/tag/v0.2.0"
        );
        assert_eq!(
            res.release_notes.as_deref(),
            Some("Melhorias de estabilidade")
        );
    }

    #[test]
    fn test_parse_github_release_json_up_to_date() {
        let json = r#"{
            "tag_name": "v0.1.0",
            "html_url": "https://github.com/mastrien/0spof/releases/tag/v0.1.0",
            "body": "",
            "draft": false,
            "prerelease": false
        }"#;

        let res = parse_github_release_json("0.1.0", json).expect("should parse");
        assert!(!res.update_available);
        assert_eq!(res.release_notes, None);
    }

    #[test]
    fn test_parse_github_release_json_ignores_draft_or_prerelease() {
        let json = r#"{
            "tag_name": "v0.3.0",
            "html_url": "",
            "draft": false,
            "prerelease": true
        }"#;

        let res = parse_github_release_json("0.1.0", json).expect("should parse");
        assert!(!res.update_available);
        assert_eq!(res.release_url, DEFAULT_RELEASES_PAGE_URL);
    }

    #[test]
    fn test_parse_github_release_json_invalid_json() {
        let err = parse_github_release_json("0.1.0", "not json").unwrap_err();
        assert!(matches!(err, SpofError::Serialization(_)));
    }
}
