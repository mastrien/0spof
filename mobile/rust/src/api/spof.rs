//! API exposta ao Dart via flutter_rust_bridge: camada fina sobre `spof_core::api`.
//! Estruturas compostas viajam como JSON (o Dart tem os modelos em `lib/models.dart`),
//! para não espelhar cada DTO no bridge.

use std::sync::{Mutex, OnceLock};

use spof_core::api::{self, dto::SessionInfoDto};
use spof_core::auth::MoodleSession;
use spof_core::storage::Database;
use tokio::runtime::Runtime;

static DB: OnceLock<Database> = OnceLock::new();
static SESSION: Mutex<Option<MoodleSession>> = Mutex::new(None);
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

type Res<T> = Result<T, String>;

fn db() -> Res<&'static Database> {
    DB.get()
        .ok_or_else(|| "banco não aberto: chame open_database primeiro".to_string())
}

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("falha ao criar o runtime tokio"))
}

fn json<T: serde::Serialize>(value: &T) -> Res<String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

fn session() -> Option<MoodleSession> {
    SESSION.lock().ok().and_then(|g| g.clone())
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Abre (ou cria) o banco no diretório de documentos do app. Chamar uma vez, no início.
pub fn open_database(path: String) -> Res<()> {
    if DB.get().is_none() {
        let database = Database::open(&path).map_err(|e| e.to_string())?;
        let _ = DB.set(database);
    }
    Ok(())
}

pub fn app_version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

// ---------- Leitura ----------

pub fn system_status() -> Res<String> {
    json(&api::system_status(db()?)?)
}

pub fn upcoming_deadlines() -> Res<String> {
    json(&api::upcoming_deadlines(db()?)?)
}

pub fn courses() -> Res<String> {
    json(&api::courses(db()?)?)
}

pub fn course_details(course_id: i64) -> Res<String> {
    json(&api::course_details(db()?, course_id)?)
}

pub fn set_course_muted(course_id: i64, is_muted: bool) -> Res<()> {
    api::set_course_muted(db()?, course_id, is_muted)
}

pub fn outbox_items() -> Res<String> {
    json(&api::outbox_items(db()?)?)
}

pub fn submission_proof(outbox_id: i64) -> Res<String> {
    json(&api::submission_proof(db()?, outbox_id)?)
}

// ---------- Sessão e crawl ----------

/// Guarda a sessão do Moodle (obtida pelo WebView de login) em memória.
pub fn save_session(base_url: String, session_cookie: String) -> Res<String> {
    let (url, cookie) = (base_url.trim(), session_cookie.trim());
    if url.is_empty() || cookie.is_empty() {
        return Err("URL base e Cookie de Sessão não podem ser vazios".into());
    }
    let s = MoodleSession::new(url.to_string(), cookie.to_string());
    let info = SessionInfoDto {
        is_authenticated: true,
        base_url: Some(s.base_url.clone()),
    };
    *SESSION.lock().map_err(|_| "sessão indisponível")? = Some(s);
    json(&info)
}

pub fn session_info() -> Res<String> {
    json(&match session() {
        Some(s) => SessionInfoDto { is_authenticated: true, base_url: Some(s.base_url) },
        None => SessionInfoDto { is_authenticated: false, base_url: None },
    })
}

pub fn logout() -> Res<()> {
    *SESSION.lock().map_err(|_| "sessão indisponível")? = None;
    Ok(())
}

pub fn sync_moodle() -> Res<String> {
    let s = session().ok_or("Nenhuma sessão configurada. Faça login no Moodle.")?;
    json(&runtime().block_on(api::sync_moodle(db()?, s))?)
}

pub fn download_activity_media(activity_id: i64, allow_video: bool) -> Res<String> {
    let s = session().ok_or("Sessão Moodle não configurada para download.")?;
    runtime().block_on(api::download_activity_media(db()?, s, activity_id, allow_video))
}

// ---------- Entregas (outbox) ----------

pub fn enqueue_submission(assign_id: i64, file_path: String, fetch_tsa: bool) -> Res<String> {
    json(&runtime().block_on(api::enqueue_submission(db()?, assign_id, &file_path, fetch_tsa))?)
}

// ---------- Dados ----------

pub fn export_data() -> Res<String> {
    api::export_data(db()?)
}

pub fn import_data(json_content: String) -> Res<String> {
    json(&api::import_data(db()?, &json_content)?)
}

pub fn wipe_all_data() -> Res<()> {
    api::wipe_all_data(db()?)
}
