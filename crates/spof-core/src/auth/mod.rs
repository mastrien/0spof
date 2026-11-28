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

/// Origem (esquema, host, porta) igual entre a base configurada e a página.
fn same_origin(base: &str, page: &str) -> bool {
    let (Ok(b), Ok(p)) = (url::Url::parse(base), url::Url::parse(page)) else {
        return false;
    };
    matches!(b.scheme(), "http" | "https")
        && b.scheme() == p.scheme()
        && b.host_str().is_some()
        && b.host_str().map(str::to_ascii_lowercase) == p.host_str().map(str::to_ascii_lowercase)
        && b.port_or_known_default() == p.port_or_known_default()
}

/// Decide se a página carregada no popup de login deve disparar a captura da
/// sessão: o carregamento terminou, a URL parece área autenticada E pertence à
/// mesma origem (esquema, host e porta) da base configurada. Sem a checagem de
/// origem, qualquer site com `/my/` no caminho (inclusive um redirecionamento
/// de terceiros) acionaria a leitura do cookie.
pub fn should_capture(base: &str, page_url: &str, finished: bool) -> bool {
    finished && is_authenticated_moodle_url(page_url) && same_origin(base, page_url)
}

/// Compara o domínio de um cookie com o host da base, sem o ponto inicial.
/// Aceita o próprio host ou um domínio-pai legítimo (com ao menos um ponto,
/// para recusar TLDs como `br`).
pub fn cookie_domain_matches(cookie_domain: &str, host: &str) -> bool {
    let d = cookie_domain.trim_start_matches('.').to_ascii_lowercase();
    let h = host.trim_start_matches('.').to_ascii_lowercase();
    if d.is_empty() || h.is_empty() {
        return false;
    }
    d == h || (d.contains('.') && h.ends_with(&format!(".{d}")))
}

/// Escolhe o valor do cookie `MoodleSession` entre `(nome, domínio, valor)`,
/// considerando só cookies cujo domínio case com o host da base. Usado como
/// fallback no macOS, onde o wry compara o domínio por igualdade exata e
/// perde cookies com ponto inicial (`.exemplo.com`).
pub fn pick_moodle_session<'a>(
    cookies: impl IntoIterator<Item = (&'a str, Option<&'a str>, &'a str)>,
    base: &str,
) -> Option<String> {
    let host = url::Url::parse(base).ok()?.host_str()?.to_string();
    cookies
        .into_iter()
        .find(|(name, domain, value)| {
            *name == "MoodleSession"
                && !value.is_empty()
                && domain.is_some_and(|d| cookie_domain_matches(d, &host))
        })
        .map(|(_, _, value)| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://moodle.exemplo.edu.br";

    #[test]
    fn should_capture_requires_finished_and_auth_url() {
        let ok = "https://moodle.exemplo.edu.br/my/";
        assert!(should_capture(BASE, ok, true));
        assert!(!should_capture(BASE, ok, false));
        assert!(!should_capture(
            BASE,
            "https://moodle.exemplo.edu.br/login/index.php",
            true
        ));
    }

    #[test]
    fn should_capture_rejects_other_host_and_subdomain() {
        assert!(!should_capture(BASE, "https://evil.example/my/", true));
        assert!(!should_capture(
            BASE,
            "https://moodle.exemplo.edu.br.evil.example/my/",
            true
        ));
        assert!(!should_capture(
            BASE,
            "https://sub.moodle.exemplo.edu.br/my/",
            true
        ));
        assert!(!should_capture(BASE, "https://exemplo.edu.br/my/", true));
    }

    #[test]
    fn should_capture_checks_scheme_and_port() {
        assert!(!should_capture(
            BASE,
            "http://moodle.exemplo.edu.br/my/",
            true
        ));
        assert!(!should_capture(
            BASE,
            "https://moodle.exemplo.edu.br:8443/my/",
            true
        ));
        // porta padrão explícita equivale à implícita
        assert!(should_capture(
            BASE,
            "https://moodle.exemplo.edu.br:443/my/",
            true
        ));
        assert!(should_capture(
            "http://localhost:8080",
            "http://localhost:8080/my/",
            true
        ));
        assert!(!should_capture(
            "http://localhost:8080",
            "http://localhost:9090/my/",
            true
        ));
        // host case-insensitive
        assert!(should_capture(
            BASE,
            "https://MOODLE.exemplo.edu.br/my/",
            true
        ));
    }

    #[test]
    fn should_capture_rejects_invalid_urls() {
        assert!(!should_capture(BASE, "not a url /my/", true));
        assert!(!should_capture(
            "moodle.exemplo.edu.br",
            "https://moodle.exemplo.edu.br/my/",
            true
        ));
        assert!(!should_capture(
            "",
            "https://moodle.exemplo.edu.br/my/",
            true
        ));
        assert!(!should_capture("file:///tmp", "file:///tmp/my/", true));
    }

    #[test]
    fn cookie_domain_matching() {
        let h = "moodle.exemplo.edu.br";
        assert!(cookie_domain_matches("moodle.exemplo.edu.br", h));
        assert!(cookie_domain_matches(".moodle.exemplo.edu.br", h));
        assert!(cookie_domain_matches(".EXEMPLO.edu.br", h));
        assert!(cookie_domain_matches("edu.br", h));
        assert!(!cookie_domain_matches("br", h));
        assert!(!cookie_domain_matches(".br", h));
        assert!(!cookie_domain_matches("", h));
        assert!(!cookie_domain_matches("evil.com", h));
        assert!(!cookie_domain_matches("oodle.exemplo.edu.br", h));
        assert!(!cookie_domain_matches("sub.moodle.exemplo.edu.br", h));
    }

    #[test]
    fn pick_session_filters_name_domain_and_empty_value() {
        let cookies = [
            ("MoodleSession", Some(".evil.com"), "stolen"),
            ("Other", Some(".exemplo.edu.br"), "x"),
            ("MoodleSession", Some(".exemplo.edu.br"), ""),
            ("MoodleSession", None, "nodomain"),
            ("MoodleSession", Some(".exemplo.edu.br"), "good"),
        ];
        assert_eq!(
            pick_moodle_session(cookies.iter().copied(), BASE),
            Some("good".to_string())
        );
        assert_eq!(
            pick_moodle_session(vec![("MoodleSession", Some(".evil.com"), "v")], BASE),
            None
        );
        assert_eq!(
            pick_moodle_session(vec![("MoodleSession", Some("a.b"), "v")], "bad"),
            None
        );
    }

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
