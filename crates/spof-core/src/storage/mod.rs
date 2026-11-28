mod backup;
mod course_export;
mod courses;
mod meta;
mod outbox;
mod schema;

#[cfg(test)]
mod backup_tests;
#[cfg(test)]
mod course_export_tests;
#[cfg(test)]
mod tests;

pub use backup::{BackupData, BundledFile, ImportSummary};
pub use course_export::{
    is_material_activity, is_pdf_activity, suggested_course_export_filename, CourseExportSelection,
    ARCHIVE_MANIFEST_NAME,
};
pub use courses::ActivityIndexRow;

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Thread-safe SQLite storage handle for 0spof offline cache.
#[derive(Clone)]
pub struct Database {
    pub(crate) conn: Arc<Mutex<Connection>>,
    /// Relatório da última sync (JSON) que não pôde ser gravado em `app_meta`; fica só em
    /// memória para o aviso chegar ao usuário até a próxima gravação bem-sucedida.
    unsaved_sync_report: Arc<Mutex<Option<String>>>,
    /// Consultas de prazos em lote já feitas (observabilidade/testes do fatiamento).
    due_date_queries: Arc<AtomicUsize>,
}

impl Database {
    /// Opens or creates an SQLite database at the specified path and initializes tables.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let connection = Connection::open(path)?;
        let database = Self::from_connection(connection);
        database.init_schema()?;
        Ok(database)
    }

    fn from_connection(connection: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(connection)),
            unsaved_sync_report: Arc::new(Mutex::new(None)),
            due_date_queries: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Guarda (ou limpa) o relatório de sync que falhou ao ser gravado.
    pub fn set_unsaved_sync_report(&self, json: Option<String>) {
        *self.unsaved_sync_report.lock().unwrap() = json;
    }

    pub fn unsaved_sync_report(&self) -> Option<String> {
        self.unsaved_sync_report.lock().unwrap().clone()
    }

    pub(crate) fn count_due_date_query(&self) {
        self.due_date_queries.fetch_add(1, Ordering::Relaxed);
    }

    /// Total de consultas de prazos em lote feitas por este banco.
    pub fn due_date_query_count(&self) -> usize {
        self.due_date_queries.load(Ordering::Relaxed)
    }

    /// Opens an in-memory SQLite database, primarily for tests and ephemeral usage.
    pub fn open_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        let database = Self::from_connection(connection);
        database.init_schema()?;
        Ok(database)
    }

    fn init_schema(&self) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        schema::initialize_schema(&connection)
    }
}
