use crate::error::Result;
use rusqlite::Connection;

/// Initializes the SQLite database schema including foreign keys, tables, migrations, and indices.
pub(crate) fn initialize_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS courses (
            id INTEGER PRIMARY KEY,
            fullname TEXT NOT NULL,
            shortname TEXT NOT NULL,
            summary TEXT,
            last_synced_at TEXT,
            is_muted INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS sections (
            id INTEGER PRIMARY KEY,
            course_id INTEGER NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            summary TEXT
        );

        CREATE TABLE IF NOT EXISTS activities (
            id INTEGER PRIMARY KEY,
            section_id INTEGER NOT NULL REFERENCES sections(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            activity_type TEXT NOT NULL,
            url TEXT,
            description TEXT,
            local_file_path TEXT,
            content_mime TEXT,
            due_date TEXT,
            is_submitted INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS outbox_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            assign_id INTEGER NOT NULL,
            filename TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            local_file_path TEXT NOT NULL,
            status TEXT NOT NULL,
            retry_count INTEGER NOT NULL DEFAULT 0,
            last_attempt_at TEXT,
            last_error TEXT,
            rfc3161_token BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;

    // Idempotent column migrations for existing databases (must run before indices on these columns)
    let _ = connection.execute(
        "ALTER TABLE courses ADD COLUMN is_muted INTEGER NOT NULL DEFAULT 0;",
        [],
    );
    let _ = connection.execute("ALTER TABLE activities ADD COLUMN description TEXT;", []);
    let _ = connection.execute(
        "ALTER TABLE activities ADD COLUMN local_file_path TEXT;",
        [],
    );
    let _ = connection.execute("ALTER TABLE activities ADD COLUMN content_mime TEXT;", []);

    // Create indices after migrations ensure all columns exist
    connection.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_activities_due_date ON activities(due_date);
        CREATE INDEX IF NOT EXISTS idx_outbox_status ON outbox_items(status);
        CREATE INDEX IF NOT EXISTS idx_courses_muted ON courses(is_muted);
        "#,
    )?;

    Ok(())
}
