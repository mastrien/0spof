use super::Database;
use crate::error::Result;
use crate::models::{OutboxItem, OutboxStatus};
use chrono::{DateTime, Utc};
use rusqlite::{params, Row};

const OUTBOX_COLUMNS: &str = "id, assign_id, filename, file_hash, local_file_path, status, \
     retry_count, last_attempt_at, last_error, rfc3161_token, created_at, updated_at";

fn parse_ts(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Converte uma linha de `outbox_items` (colunas em `OUTBOX_COLUMNS`) em `OutboxItem`.
fn map_outbox_row(row: &Row) -> rusqlite::Result<OutboxItem> {
    let status_str: String = row.get(5)?;
    let last_attempt_str: Option<String> = row.get(7)?;
    let created_str: String = row.get(10)?;
    let updated_str: String = row.get(11)?;
    Ok(OutboxItem {
        id: Some(row.get(0)?),
        assign_id: row.get(1)?,
        filename: row.get(2)?,
        file_hash: row.get(3)?,
        local_file_path: row.get(4)?,
        status: status_str.parse().unwrap_or(OutboxStatus::Pending),
        retry_count: row.get(6)?,
        last_attempt_at: last_attempt_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        }),
        last_error: row.get(8)?,
        rfc3161_token: row.get(9)?,
        created_at: parse_ts(&created_str),
        updated_at: parse_ts(&updated_str),
    })
}

impl Database {
    // --- Outbox Items ---

    pub fn insert_outbox_item(&self, item: &OutboxItem) -> Result<i64> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            INSERT INTO outbox_items (
                assign_id, filename, file_hash, local_file_path, status,
                retry_count, last_attempt_at, last_error, rfc3161_token, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                item.assign_id,
                item.filename,
                item.file_hash,
                item.local_file_path,
                item.status.as_str(),
                item.retry_count,
                item.last_attempt_at.map(|t| t.to_rfc3339()),
                item.last_error,
                item.rfc3161_token,
                item.created_at.to_rfc3339(),
                item.updated_at.to_rfc3339(),
            ],
        )?;
        let inserted_id = connection.last_insert_rowid();
        Ok(inserted_id)
    }

    pub fn get_outbox_item(&self, outbox_id: i64) -> Result<Option<OutboxItem>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(&format!(
            "SELECT {OUTBOX_COLUMNS} FROM outbox_items WHERE id = ?1"
        ))?;
        let mut rows = statement.query_map(params![outbox_id], map_outbox_row)?;

        if let Some(outbox_result) = rows.next() {
            Ok(Some(outbox_result?))
        } else {
            Ok(None)
        }
    }

    fn query_outbox(&self, where_clause: &str, order: &str) -> Result<Vec<OutboxItem>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(&format!(
            "SELECT {OUTBOX_COLUMNS} FROM outbox_items {where_clause} ORDER BY {order}"
        ))?;
        let rows = statement.query_map([], map_outbox_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// Todos os itens da fila (inclusive entregues), do mais antigo ao mais recente.
    pub fn get_all_outbox_items(&self) -> Result<Vec<OutboxItem>> {
        self.query_outbox("", "created_at ASC, id ASC")
    }

    pub fn get_pending_outbox_items(&self) -> Result<Vec<OutboxItem>> {
        self.query_outbox("WHERE status IN ('pending', 'failed')", "created_at ASC")
    }

    pub fn update_outbox_item(&self, item: &OutboxItem) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            UPDATE outbox_items SET
                status = ?1,
                retry_count = ?2,
                last_attempt_at = ?3,
                last_error = ?4,
                rfc3161_token = ?5,
                updated_at = ?6
            WHERE id = ?7
            "#,
            params![
                item.status.as_str(),
                item.retry_count,
                item.last_attempt_at.map(|t| t.to_rfc3339()),
                item.last_error,
                item.rfc3161_token,
                item.updated_at.to_rfc3339(),
                item.id,
            ],
        )?;
        Ok(())
    }
}
