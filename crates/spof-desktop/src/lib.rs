pub mod commands;

pub use commands::AppState;
use spof_core::storage::Database;
use std::path::PathBuf;
use std::sync::Arc;

pub fn run() {
    let db_path = PathBuf::from("0spof.db");
    let db =
        Database::open(&db_path).expect("Failed to initialize SQLite database for 0spof desktop");
    let app_state = Arc::new(AppState::new(db));

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_system_status,
            commands::get_courses,
            commands::get_outbox_items,
            commands::enqueue_submission,
            commands::get_submission_proof,
            commands::save_session,
            commands::get_session_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 0spof desktop application");
}
