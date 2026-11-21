//! Servidor de sincronização com o celular: liga/desliga, QR code de pareamento, troca de token.

use crate::commands::AppState;
use serde::{Deserialize, Serialize};
use spof_core::sync::server;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncInfoDto {
    pub running: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub device_name: String,
    /// Conteúdo do QR code (também serve para colar no celular).
    pub pairing_code: Option<String>,
    pub qr_svg: Option<String>,
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

pub fn handle_get_sync_info(state: &AppState) -> Result<SyncInfoDto, String> {
    let port = state.sync_server.lock().unwrap().as_ref().map(|h| h.port);
    let device_name = server::device_name();
    let Some(port) = port else {
        return Ok(SyncInfoDto {
            running: false,
            host: None,
            port: None,
            device_name,
            pairing_code: None,
            qr_svg: None,
        });
    };
    let pairing = server::pairing_for(&state.db, port).map_err(err)?;
    Ok(SyncInfoDto {
        running: true,
        host: Some(pairing.host.clone()),
        port: Some(port),
        device_name,
        pairing_code: Some(pairing.to_json()),
        qr_svg: Some(server::pairing_qr_svg(&pairing).map_err(err)?),
    })
}

pub fn handle_start_sync_server(state: &AppState) -> Result<SyncInfoDto, String> {
    {
        let mut guard = state.sync_server.lock().unwrap();
        if guard.is_none() {
            // Porta fixa para o QR continuar válido entre execuções; se estiver ocupada, uma livre.
            let handle = server::start(state.db.clone(), server::DEFAULT_PORT)
                .or_else(|_| server::start(state.db.clone(), 0))
                .map_err(err)?;
            *guard = Some(handle);
        }
    }
    handle_get_sync_info(state)
}

pub fn handle_stop_sync_server(state: &AppState) -> Result<SyncInfoDto, String> {
    let handle = state.sync_server.lock().unwrap().take();
    if let Some(h) = handle {
        h.stop();
    }
    handle_get_sync_info(state)
}

/// Gera outro token: todos os celulares pareados precisam ler o QR de novo.
pub fn handle_rotate_sync_token(state: &AppState) -> Result<SyncInfoDto, String> {
    server::rotate_token(&state.db).map_err(err)?;
    handle_get_sync_info(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spof_core::storage::Database;

    #[test]
    fn start_stop_and_rotate() {
        let state = AppState::new(Database::open_in_memory().unwrap());
        assert!(!handle_get_sync_info(&state).unwrap().running);

        let on = handle_start_sync_server(&state).unwrap();
        assert!(on.running && on.qr_svg.unwrap().contains("<svg"));
        let before = on.pairing_code.unwrap();

        let after = handle_rotate_sync_token(&state)
            .unwrap()
            .pairing_code
            .unwrap();
        assert_ne!(before, after);

        assert!(!handle_stop_sync_server(&state).unwrap().running);
    }
}
