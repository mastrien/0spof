use crate::commands::{handle_save_session, AppState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub use spof_core::auth::is_authenticated_moodle_url;
use spof_core::auth::should_capture;

const POPUP_LABEL: &str = "moodle_login_popup";

/// Evita capturas simultâneas: `PageLoadEvent::Finished` pode disparar várias
/// vezes (redirecionamentos, iframes) antes de o popup fechar.
static CAPTURING: AtomicBool = AtomicBool::new(false);

struct CaptureGuard;

impl CaptureGuard {
    fn acquire() -> Option<Self> {
        CAPTURING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| CaptureGuard)
    }
}

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURING.store(false, Ordering::Release);
    }
}

/// Salva a sessão; só em caso de sucesso avisa a UI e fecha o popup. Em erro
/// registra em stderr (sem o valor do cookie) e mantém o popup aberto para
/// nova tentativa.
fn save_and_finish(
    win: &tauri::WebviewWindow,
    app: &tauri::AppHandle,
    state: &AppState,
    base_url: &str,
    cookie_value: String,
) -> bool {
    match handle_save_session(state, base_url.to_string(), cookie_value) {
        Ok(_) => {
            let _ = app.emit("session_connected", ());
            let _ = win.close();
            true
        }
        Err(e) => {
            eprintln!("0spof: falha ao salvar a sessão capturada: {e}");
            false
        }
    }
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

    if let Some(existing) = app.get_webview_window(POPUP_LABEL) {
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
        POPUP_LABEL,
        tauri::WebviewUrl::External(parsed_url),
    )
    .title("Conexão Institucional - Moodle / SUAP")
    .inner_size(580.0, 740.0)
    .center()
    .resizable(true)
    .on_page_load(move |win, payload| {
        let finished = payload.event() == tauri::webview::PageLoadEvent::Finished;
        if should_capture(&clean_base_clone, payload.url().as_str(), finished) {
            let win_clone = win.clone();
            let app_clone = app_clone.clone();
            let state_clone = state_clone.clone();
            let clean_base_clone = clean_base_clone.clone();
            // A leitura de cookies bloqueia até a thread principal responder;
            // por isso roda numa thread de bloqueio, nunca no async runtime.
            tauri::async_runtime::spawn_blocking(move || {
                let Some(_guard) = CaptureGuard::acquire() else {
                    return;
                };
                // O popup pode já ter sido fechado por um Finished anterior:
                // ler cookies de uma janela destruída causa panic no wry.
                if app_clone.get_webview_window(POPUP_LABEL).is_none() {
                    return;
                }

                #[cfg(windows)]
                extract_and_save_cookie_windows(
                    &win_clone,
                    &app_clone,
                    &state_clone,
                    &clean_base_clone,
                );

                #[cfg(not(windows))]
                extract_and_save_cookie_native(
                    &win_clone,
                    &app_clone,
                    &state_clone,
                    &clean_base_clone,
                );
            });
        }
    })
    .build()
    .map_err(|e| format!("Falha ao abrir janela de autenticação: {}", e))?;

    Ok(())
}

#[cfg(not(windows))]
fn extract_and_save_cookie_native(
    win: &tauri::WebviewWindow,
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    base_url: &str,
) {
    let Ok(parsed_url) = base_url.parse::<tauri::Url>() else {
        return;
    };
    let mut value = win.cookies_for_url(parsed_url).ok().and_then(|cookies| {
        cookies
            .iter()
            .find(|c| c.name() == "MoodleSession" && !c.value().is_empty())
            .map(|c| c.value().to_string())
    });

    // No macOS o wry (WKWebView) filtra `cookies_for_url` por igualdade exata
    // entre `cookie.domain()` e o host da URL, então um cookie com domínio
    // `.host` (ponto inicial) não é encontrado. Fallback: lista todos os
    // cookies e aceita só `MoodleSession` cujo domínio case com o host da base.
    if value.is_none() {
        if let Ok(all) = win.cookies() {
            value = spof_core::auth::pick_moodle_session(
                all.iter().map(|c| (c.name(), c.domain(), c.value())),
                base_url,
            );
        }
    }

    if let Some(v) = value {
        save_and_finish(win, app, state, base_url, v);
    }
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
                                            if save_and_finish(
                                                &win_handle,
                                                &app_handle,
                                                &state_handle,
                                                &base_url_owned,
                                                value_str,
                                            ) {
                                                break;
                                            }
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
