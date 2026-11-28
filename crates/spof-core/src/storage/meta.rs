use super::Database;
use crate::error::Result;
use rusqlite::{params, OptionalExtension};

impl Database {
    /// Lê uma configuração local (chave/valor) que não é sincronizada.
    pub fn meta_get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Bytes alocados pelo banco SQLite (páginas em uso x tamanho da página).
    pub fn database_size_bytes(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok((pages.max(0) * page_size.max(0)) as u64)
    }

    pub fn meta_delete(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM app_meta WHERE key = ?1", params![key])?;
        Ok(())
    }
}
