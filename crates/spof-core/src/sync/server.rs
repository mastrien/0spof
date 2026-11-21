use super::{pairing::Pairing, snapshot::Snapshot};
use crate::error::{Result, SpofError};
use crate::storage::Database;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 7378;
const TOKEN_KEY: &str = "sync.server_token";
const MAX_BODY: u64 = 64 * 1024 * 1024;

/// Nome deste aparelho, como aparece no outro lado.
pub fn device_name() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "0spof".to_string())
}

/// Token do servidor: criado no primeiro uso e guardado no banco.
pub fn server_token(db: &Database) -> Result<String> {
    if let Some(t) = db.meta_get(TOKEN_KEY)? {
        return Ok(t);
    }
    rotate_token(db)
}

/// Gera outro token, o que invalida todos os pareamentos anteriores.
pub fn rotate_token(db: &Database) -> Result<String> {
    let mut bytes = [0u8; 24];
    getrandom::getrandom(&mut bytes).map_err(|e| SpofError::Validation(e.to_string()))?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    db.meta_set(TOKEN_KEY, &token)?;
    Ok(token)
}

fn token_matches(expected: &str, header: Option<&str>) -> bool {
    let given = header.and_then(|h| h.strip_prefix("Bearer ")).unwrap_or("");
    // Comparação sem curto-circuito.
    let (a, b) = (expected.as_bytes(), given.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= (*a.get(i).unwrap_or(&0) ^ *b.get(i).unwrap_or(&0)) as usize;
    }
    diff == 0
}

/// Resposta HTTP pura (sem socket), para facilitar os testes.
pub struct Reply {
    pub status: u16,
    pub body: String,
}

fn reply(status: u16, body: impl Into<String>) -> Reply {
    Reply { status, body: body.into() }
}

pub fn handle(db: &Database, token: &str, method: &str, path: &str, auth: Option<&str>, body: &str) -> Reply {
    if !token_matches(token, auth) {
        return reply(401, r#"{"error":"unauthorized"}"#);
    }
    match (method, path) {
        ("GET", "/sync/snapshot") => match db
            .sync_snapshot(&device_name())
            .and_then(|s| serde_json::to_string(&s).map_err(Into::into))
        {
            Ok(json) => reply(200, json),
            Err(e) => reply(500, e.to_string()),
        },
        ("POST", "/sync/snapshot") => match serde_json::from_str::<Snapshot>(body) {
            Ok(snap) => match db.apply_snapshot(&snap) {
                Ok(sum) => reply(200, serde_json::to_string(&sum).unwrap_or_default()),
                Err(e) => reply(500, e.to_string()),
            },
            Err(e) => reply(400, e.to_string()),
        },
        _ => reply(404, r#"{"error":"not found"}"#),
    }
}

pub struct ServerHandle {
    pub port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ServerHandle {
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Sobe o servidor em `0.0.0.0:port` (`0` escolhe uma porta livre). O token é lido do banco a cada requisição.
pub fn start(db: Database, port: u16) -> Result<ServerHandle> {
    let server = tiny_http::Server::http(("0.0.0.0", port))
        .map_err(|e| SpofError::Network(format!("Não foi possível abrir a porta {port}: {e}")))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            let Ok(Some(mut req)) = server.recv_timeout(Duration::from_millis(250)) else {
                continue;
            };
            let method = req.method().as_str().to_string();
            let path = req.url().split('?').next().unwrap_or("").to_string();
            let auth = req
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.as_str().to_string());
            let mut body = String::new();
            let _ = req.as_reader().take(MAX_BODY).read_to_string(&mut body);
            let out = match server_token(&db) {
                Ok(token) => handle(&db, &token, &method, &path, auth.as_deref(), &body),
                Err(e) => reply(500, e.to_string()),
            };
            let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let _ = req.respond(tiny_http::Response::from_string(out.body).with_status_code(out.status).with_header(header));
        }
    });
    Ok(ServerHandle { port, stop, thread: Some(thread) })
}

/// IP local preferido para o celular alcançar este PC.
pub fn local_host() -> String {
    local_ip_address::local_ip().map(|ip| ip.to_string()).unwrap_or_else(|_| "127.0.0.1".into())
}

pub fn pairing_for(db: &Database, port: u16) -> Result<Pairing> {
    Ok(Pairing { host: local_host(), port, token: server_token(db)?, name: device_name() })
}

/// QR code (SVG) com o conteúdo do pareamento.
pub fn pairing_qr_svg(pairing: &Pairing) -> Result<String> {
    let code = qrcode::QrCode::new(pairing.to_json().as_bytes())
        .map_err(|e| SpofError::Validation(e.to_string()))?;
    Ok(code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Course;
    use crate::sync::client;

    fn course(db: &Database, id: i64, name: &str) {
        db.upsert_course(&Course { id, fullname: name.into(), shortname: name.into(), summary: None, last_synced_at: None, is_muted: false })
            .unwrap();
    }

    #[test]
    fn rejects_bad_token_and_unknown_route() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(handle(&db, "t", "GET", "/sync/snapshot", None, "").status, 401);
        assert_eq!(handle(&db, "t", "GET", "/sync/snapshot", Some("Bearer x"), "").status, 401);
        assert_eq!(handle(&db, "t", "GET", "/nada", Some("Bearer t"), "").status, 404);
        assert_eq!(handle(&db, "t", "POST", "/sync/snapshot", Some("Bearer t"), "{").status, 400);
        assert_eq!(handle(&db, "t", "GET", "/sync/snapshot", Some("Bearer t"), "").status, 200);
    }

    #[test]
    fn token_rotation_changes_value() {
        let db = Database::open_in_memory().unwrap();
        let a = server_token(&db).unwrap();
        assert_eq!(a, server_token(&db).unwrap());
        assert_ne!(a, rotate_token(&db).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pc_and_phone_sync_both_ways_over_http() {
        let pc = Database::open_in_memory().unwrap();
        course(&pc, 1, "Do PC");
        let phone = Database::open_in_memory().unwrap();
        course(&phone, 2, "Do celular");

        let handle = start(pc.clone(), 0).unwrap();
        let mut pairing = pairing_for(&pc, handle.port).unwrap();
        pairing.host = "127.0.0.1".into();

        let r = client::pair(&phone, &pairing, "Pixel").await.unwrap();
        assert_eq!(r.received.courses, 1);
        assert_eq!(phone.get_courses().unwrap().len(), 2);
        assert_eq!(pc.get_courses().unwrap().len(), 2);
        assert!(phone.paired_peer().unwrap().is_some());

        // Token errado não pareia nem grava nada.
        let other = Database::open_in_memory().unwrap();
        let bad = Pairing { token: "errado".into(), ..pairing.clone() };
        assert!(client::pair(&other, &bad, "X").await.is_err());
        assert!(other.paired_peer().unwrap().is_none());

        // Após rotacionar o token, o celular pareado é recusado.
        rotate_token(&pc).unwrap();
        assert!(client::sync_now(&phone, "Pixel").await.is_err());
        handle.stop();
    }

    #[test]
    fn qr_is_svg() {
        let p = Pairing { host: "1.2.3.4".into(), port: 7378, token: "t".into(), name: "PC".into() };
        assert!(pairing_qr_svg(&p).unwrap().contains("<svg"));
    }
}
