pub mod client;
pub mod session;

pub use client::MoodleClient;
pub use session::MoodleSession;

/// Indica que o login terminou: a página carregada já é uma área autenticada do Moodle.
pub fn is_authenticated_moodle_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.contains("/my/")
        || lower.contains("/my/courses.php")
        || lower.contains("/user/preferences.php")
        || lower.contains("/course/view.php"))
        && !lower.contains("/login/")
        && !lower.contains("/auth/oauth2/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_authenticated_moodle_url() {
        assert!(is_authenticated_moodle_url(
            "https://moodle2.jcr.ifsp.edu.br/my/"
        ));
        assert!(is_authenticated_moodle_url(
            "https://moodle2.jcr.ifsp.edu.br/my/courses.php"
        ));
        assert!(is_authenticated_moodle_url(
            "https://moodle2.jcr.ifsp.edu.br/course/view.php?id=86"
        ));

        // Páginas ainda não autenticadas
        assert!(!is_authenticated_moodle_url(
            "https://moodle2.jcr.ifsp.edu.br/login/index.php"
        ));
        assert!(!is_authenticated_moodle_url(
            "https://moodle2.jcr.ifsp.edu.br/auth/oauth2/login.php?id=2"
        ));
        assert!(!is_authenticated_moodle_url(
            "https://suap.ifsp.edu.br/accounts/login/"
        ));
    }
}
