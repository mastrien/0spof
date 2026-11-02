use crate::auth::session::MoodleSession;
use crate::error::{Result, SpofError};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};
use std::time::Duration;

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36 0spof/0.1.0";

/// HTTP client configured specifically for resilient Moodle and SUAP interactions.
#[derive(Clone)]
pub struct MoodleClient {
    http: reqwest::Client,
    base_url: String,
    session: Option<MoodleSession>,
}

impl MoodleClient {
    /// Creates a new MoodleClient pointing to the specified base URL.
    pub fn new(base_url: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| SpofError::Network(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            session: None,
        })
    }

    /// Attaches an authenticated session to the client.
    pub fn with_session(mut self, session: MoodleSession) -> Self {
        self.session = Some(session);
        self
    }

    pub fn session(&self) -> Option<&MoodleSession> {
        self.session.as_ref()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Checks if the remote Moodle server is reachable and responsive (health check).
    pub async fn check_health(&self) -> bool {
        let endpoint = format!("{}/login/index.php", self.base_url);
        match self.http.get(&endpoint).send().await {
            Ok(resp) => resp.status().is_success() || resp.status().is_redirection(),
            Err(_) => false,
        }
    }

    /// Inspects an HTML body to determine if the user is authenticated in Moodle.
    pub fn is_authenticated_html(html: &str) -> bool {
        // Unauthenticated indicator: presence of the login form or login button
        let has_login_form = html.contains("name=\"logintoken\"")
            || html.contains("name='logintoken'")
            || html.contains("/login/index.php");

        // Authenticated indicator: presence of user profile, logout link, or my courses
        let has_auth_indicators = html.contains("/login/logout.php")
            || html.contains("sesskey")
            || html.contains("myoverview");

        has_auth_indicators && !has_login_form
    }

    /// Verifies if the attached session is still authenticated against the remote Moodle.
    pub async fn verify_session_online(&self) -> Result<bool> {
        let session = self.session.as_ref().ok_or_else(|| {
            SpofError::Authentication("No session attached to client".to_string())
        })?;

        if session.is_expired() {
            return Ok(false);
        }

        let endpoint = format!("{}/my/", self.base_url);
        let resp = self
            .http
            .get(&endpoint)
            .header(COOKIE, session.cookie_header())
            .send()
            .await
            .map_err(|e| SpofError::Network(format!("Failed to verify session: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(false);
        }

        let body = resp
            .text()
            .await
            .map_err(|e| SpofError::Network(format!("Failed to read response body: {}", e)))?;

        Ok(Self::is_authenticated_html(&body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation_and_base_url_trim() {
        let client = MoodleClient::new("https://moodle.if.edu.br///".to_string()).unwrap();
        assert_eq!(client.base_url(), "https://moodle.if.edu.br");
        assert!(client.session().is_none());

        let session = MoodleSession::new(client.base_url().to_string(), "sess123".to_string());
        let client_with_session = client.with_session(session);
        assert!(client_with_session.session().is_some());
    }

    #[test]
    fn test_is_authenticated_html_evaluation() {
        let logged_in_html = r#"
            <html>
                <nav><a href="https://moodle.edu.br/login/logout.php?sesskey=abc123">Sair</a></nav>
                <div id="myoverview">Cursos do Semestre</div>
            </html>
        "#;
        assert!(MoodleClient::is_authenticated_html(logged_in_html));

        let logged_out_html = r#"
            <html>
                <form action="/login/index.php" method="post">
                    <input type="hidden" name="logintoken" value="token123">
                </form>
            </html>
        "#;
        assert!(!MoodleClient::is_authenticated_html(logged_out_html));
    }
}
