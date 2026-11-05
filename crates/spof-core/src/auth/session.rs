use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an active or cached Moodle user session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoodleSession {
    /// The actual value of the MoodleSession cookie.
    pub session_cookie: String,
    /// Base URL of the Moodle instance (e.g., "https://moodle.instituicao.edu.br")
    pub base_url: String,
    /// User session key (sesskey) used for anti-CSRF in internal requests and submissions.
    pub sesskey: Option<String>,
    /// When the session was captured.
    pub created_at: DateTime<Utc>,
    /// Optional expiration time if known.
    pub expires_at: Option<DateTime<Utc>>,
}

impl MoodleSession {
    pub fn new(base_url: String, session_cookie: String) -> Self {
        let trimmed_cookie = session_cookie.trim();
        let stripped_header = trimmed_cookie
            .strip_prefix("Cookie:")
            .unwrap_or(trimmed_cookie)
            .trim();
        let clean_cookie = stripped_header
            .strip_prefix("MoodleSession=")
            .unwrap_or(stripped_header)
            .trim_matches(|c| c == ';' || c == '"' || c == ' ')
            .to_string();

        Self {
            session_cookie: clean_cookie,
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            sesskey: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    /// Returns the formatted Cookie header value.
    pub fn cookie_header(&self) -> String {
        format!("MoodleSession={}", self.session_cookie)
    }

    /// Checks if the session has exceeded its explicit expiration date.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            Utc::now() > exp
        } else {
            false
        }
    }

    /// Attaches the extracted anti-CSRF sesskey to this session.
    pub fn with_sesskey(mut self, sesskey: String) -> Self {
        self.sesskey = Some(sesskey);
        self
    }
}

/// Helper functions to parse anti-CSRF tokens from Moodle HTML responses.
pub mod parser {
    /// Extracts the login anti-CSRF token (`logintoken`) from Moodle's login page HTML.
    /// Handles attributes in varied order and single or double quotes.
    pub fn extract_login_token(html: &str) -> Option<String> {
        let input_tag = html.lines().find(|line| {
            line.contains("name=\"logintoken\"") || line.contains("name='logintoken'")
        })?;

        extract_attribute_value(input_tag, "value")
    }

    /// Extracts the active session key (`sesskey`) from either HTML input tags or inline JSON/JS snippets:
    /// e.g. `name="sesskey" value="abc123"` or `"sesskey":"abc123"` or `M.cfg.sesskey = "abc123"`.
    pub fn extract_sesskey(html: &str) -> Option<String> {
        // Pattern 1: HTML input tag
        for line in html.lines() {
            if line.contains("name=\"sesskey\"") || line.contains("name='sesskey'") {
                if let Some(val) = extract_attribute_value(line, "value") {
                    return Some(val);
                }
            }
        }

        // Pattern 2: JSON key "sesskey":"..."
        if let Some(pos) = html.find("\"sesskey\":\"") {
            let start = pos + "\"sesskey\":\"".len();
            if let Some(end) = html[start..].find('"') {
                return Some(html[start..start + end].to_string());
            }
        }

        // Pattern 3: JavaScript assignment M.cfg.sesskey = "..."
        if let Some(pos) = html.find("M.cfg.sesskey") {
            let snippet = &html[pos..];
            if let Some(quote_start) = snippet.find('"') {
                let after = &snippet[quote_start + 1..];
                if let Some(quote_end) = after.find('"') {
                    return Some(after[..quote_end].to_string());
                }
            }
        }

        None
    }

    fn extract_attribute_value(tag: &str, attr: &str) -> Option<String> {
        let pattern_double = format!("{}=\"", attr);
        let pattern_single = format!("{}='", attr);

        if let Some(start_idx) = tag.find(&pattern_double) {
            let val_start = start_idx + pattern_double.len();
            let val_end = tag[val_start..].find('"')?;
            return Some(tag[val_start..val_start + val_end].to_string());
        }

        if let Some(start_idx) = tag.find(&pattern_single) {
            let val_start = start_idx + pattern_single.len();
            let val_end = tag[val_start..].find('\'')?;
            return Some(tag[val_start..val_start + val_end].to_string());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::parser::*;
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_moodle_session_lifecycle() {
        let mut session = MoodleSession::new(
            "https://moodle.if.edu.br".to_string(),
            "abc123session".to_string(),
        );

        assert_eq!(session.cookie_header(), "MoodleSession=abc123session");
        assert!(!session.is_expired());

        // Test with MoodleSession= prefix and semicolon
        let session2 = MoodleSession::new(
            "https://moodle.if.edu.br/".to_string(),
            "MoodleSession=xyz789;".to_string(),
        );
        assert_eq!(session2.session_cookie, "xyz789");
        assert_eq!(session2.base_url, "https://moodle.if.edu.br");

        // Set past expiration
        session.expires_at = Some(Utc::now() - Duration::hours(1));
        assert!(session.is_expired());

        // Attach sesskey
        let with_key = session.with_sesskey("key999".to_string());
        assert_eq!(with_key.sesskey.as_deref(), Some("key999"));
    }

    #[test]
    fn test_extract_login_token_double_quotes() {
        let html = r#"
            <form action="https://moodle.edu.br/login/index.php" method="post">
                <input type="hidden" name="logintoken" value="token_secret_12345">
                <input type="text" name="username">
            </form>
        "#;
        let token = extract_login_token(html);
        assert_eq!(token.as_deref(), Some("token_secret_12345"));
    }

    #[test]
    fn test_extract_login_token_single_quotes() {
        let html = r#"<input type='hidden' name='logintoken' value='token_single_quote'>"#;
        let token = extract_login_token(html);
        assert_eq!(token.as_deref(), Some("token_single_quote"));
    }

    #[test]
    fn test_extract_login_token_missing() {
        let html = r#"<div>Sem formulário de login aqui</div>"#;
        assert_eq!(extract_login_token(html), None);
    }

    #[test]
    fn test_extract_sesskey_from_input() {
        let html = r#"<input type="hidden" name="sesskey" value="session_key_input_123">"#;
        let key = extract_sesskey(html);
        assert_eq!(key.as_deref(), Some("session_key_input_123"));
    }

    #[test]
    fn test_extract_sesskey_from_json() {
        let html = r#"var config = {"wwwroot":"https://moodle","sesskey":"json_sesskey_789"};"#;
        let key = extract_sesskey(html);
        assert_eq!(key.as_deref(), Some("json_sesskey_789"));
    }

    #[test]
    fn test_extract_sesskey_from_mcfg() {
        let html = r#"<script>M.cfg.sesskey = "cfg_sesskey_456";</script>"#;
        let key = extract_sesskey(html);
        assert_eq!(key.as_deref(), Some("cfg_sesskey_456"));
    }
}
