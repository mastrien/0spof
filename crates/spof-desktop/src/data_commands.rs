use crate::commands::AppState;
use crate::dto::{ImportSummaryDto, SessionInfoDto, UpdateInfoDto};

pub fn handle_logout(state: &AppState) -> SessionInfoDto {
    let mut session_guard = state.session.lock().unwrap();
    *session_guard = None;
    SessionInfoDto {
        is_authenticated: false,
        base_url: None,
    }
}

pub fn handle_wipe_all_data(state: &AppState) -> Result<(), String> {
    spof_core::api::wipe_all_data(&state.db)
}

pub fn handle_export_data(state: &AppState) -> Result<String, String> {
    spof_core::api::export_data(&state.db)
}

pub fn handle_import_data(
    state: &AppState,
    json_content: String,
) -> Result<ImportSummaryDto, String> {
    spof_core::api::import_data(&state.db, &json_content)
}

pub fn handle_get_app_version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

pub async fn handle_check_for_updates() -> Result<UpdateInfoDto, String> {
    let result = spof_core::check_for_updates(env!("CARGO_PKG_VERSION"))
        .await
        .map_err(|e| e.to_string())?;

    Ok(UpdateInfoDto {
        current_version: result.current_version,
        latest_version: result.latest_version,
        update_available: result.update_available,
        release_url: result.release_url,
        release_notes: result.release_notes,
    })
}

#[cfg(test)]
#[path = "data_commands_tests.rs"]
mod tests;
