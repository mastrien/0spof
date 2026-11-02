mod courses;
mod outbox;
mod schema;

#[cfg(test)]
mod tests;

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Thread-safe SQLite storage handle for 0spof offline cache.
#[derive(Clone)]
pub struct Database {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Opens or creates an SQLite database at the specified path and initializes tables.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let connection = Connection::open(path)?;
        let database = Self {
            conn: Arc::new(Mutex::new(connection)),
        };
        database.init_schema()?;
        Ok(database)
    }

    /// Opens an in-memory SQLite database, primarily for tests and ephemeral usage.
    pub fn open_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        let database = Self {
            conn: Arc::new(Mutex::new(connection)),
        };
        database.init_schema()?;
        Ok(database)
    }

    fn init_schema(&self) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        schema::initialize_schema(&connection)
    }
}
