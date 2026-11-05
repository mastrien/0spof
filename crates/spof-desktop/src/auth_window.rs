use crate::commands::{handle_save_session, AppState};
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub fn is_authenticated_moodle_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.contains("/my/")
        || lower.contains("/my/courses.php")
        || lower.contains("/user/preferences.php")
        || lower.contains("/course/view.php"))
        && !lower.contains("/login/")
        && !lower.contains("/auth/oauth2/")
}

pub fn handle_open_moodle_login_window(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    base_url_opt: Option<String>,
) -> Result<(), String> {
    let base_url = base_url_opt
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            let guard = state.session.lock().unwrap();
            guard.as_ref().map(|s| s.base_url.clone())
        })
        .unwrap_or_else(|| "https://moodle2.jcr.ifsp.edu.br".to_string());

    let clean_base = base_url.trim().trim_end_matches('/').to_string();
    let login_url = format!("{}/login/index.php", clean_base);

    if let Some(existing) = app.get_webview_window("moodle_login_popup") {
        let _ = existing.set_focus();
        return Ok(());
    }

    let parsed_url = login_url
        .parse()
        .map_err(|e| format!("URL de login inválida: {}", e))?;

    let app_clone = app.clone();
    let state_clone = state.clone();
    let clean_base_clone = clean_base.clone();

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        "moodle_login_popup",
        tauri::WebviewUrl::External(parsed_url),
    )
    .title("Conexão Institucional - Moodle / SUAP")
    .inner_size(580.0, 740.0)
    .center()
    .resizable(true)
    .on_page_load(move |_win, _payload| {
        #[cfg(windows)]
        {
            let url_str = _payload.url().as_str();
            if is_authenticated_moodle_url(url_str) {
                extract_and_save_cookie_windows(&_win, &app_clone, &state_clone, &clean_base_clone);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (
                &_win,
                &_payload,
                &app_clone,
                &state_clone,
                &clean_base_clone,
            );
        }
    })
    .build()
    .map_err(|e| format!("Falha ao abrir janela de autenticação: {}", e))?;

    Ok(())
}

#[cfg(windows)]
fn extract_and_save_cookie_windows(
    win: &tauri::WebviewWindow,
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    base_url: &str,
) {
    use webview2_com::GetCookiesCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_2;
    use windows_core::{Interface, HSTRING, PCWSTR, PWSTR};

    let app_handle = app.clone();
    let state_handle = state.clone();
    let base_url_owned = base_url.to_string();
    let win_handle = win.clone();

    let _ = win.with_webview(move |webview| unsafe {
        let controller = webview.controller();
        if let Ok(core) = controller.CoreWebView2() {
            if let Ok(core2) = core.cast::<ICoreWebView2_2>() {
                if let Ok(manager) = core2.CookieManager() {
                    let hurl = HSTRING::from(&base_url_owned);
                    let pcwstr = PCWSTR::from_raw(hurl.as_ptr());

                    let handler =
                        GetCookiesCompletedHandler::create(Box::new(move |_res, list| {
                            if let Some(list) = list {
                                let mut count = 0u32;
                                let _ = list.Count(&mut count);
                                for i in 0..count {
                                    if let Ok(cookie) = list.GetValueAtIndex(i) {
                                        let mut name = PWSTR(std::ptr::null_mut());
                                        let _ = cookie.Name(std::ptr::addr_of_mut!(name));
                                        let mut value = PWSTR(std::ptr::null_mut());
                                        let _ = cookie.Value(std::ptr::addr_of_mut!(value));

                                        let name_str = if !name.0.is_null() {
                                            name.to_string().unwrap_or_default()
                                        } else {
                                            String::new()
                                        };

                                        let value_str = if !value.0.is_null() {
                                            value.to_string().unwrap_or_default()
                                        } else {
                                            String::new()
                                        };

                                        if name_str == "MoodleSession" && !value_str.is_empty() {
                                            let _ = handle_save_session(
                                                &state_handle,
                                                base_url_owned.clone(),
                                                value_str,
                                            );
                                            let _ = app_handle.emit("session_connected", ());
                                            let _ = win_handle.close();
                                            break;
                                        }
                                    }
                                }
                            }
                            Ok(())
                        }));

                    let _ = manager.GetCookies(pcwstr, &handler);
                }
            }
        }
    });
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
