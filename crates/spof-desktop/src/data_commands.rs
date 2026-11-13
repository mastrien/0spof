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
    state.db.wipe_all_data().map_err(|e| e.to_string())
}

pub fn handle_export_data(state: &AppState) -> Result<String, String> {
    state.db.export_data_json().map_err(|e| e.to_string())
}

pub fn handle_import_data(
    state: &AppState,
    json_content: String,
) -> Result<ImportSummaryDto, String> {
    let summary = state
        .db
        .import_data_json(&json_content)
        .map_err(|e| e.to_string())?;

    Ok(ImportSummaryDto {
        courses_imported: summary.courses_imported,
        sections_imported: summary.sections_imported,
        activities_imported: summary.activities_imported,
        outbox_items_imported: summary.outbox_items_imported,
    })
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
