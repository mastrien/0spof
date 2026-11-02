use crate::error::Result;
use crate::models::{Activity, ActivityType, Course, OutboxItem, OutboxStatus, Section};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Thread-safe SQLite storage handle for 0spof offline cache.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Opens or creates an SQLite database at the specified path and initializes tables.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Opens an in-memory SQLite database, primarily for tests and ephemeral usage.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS courses (
                id INTEGER PRIMARY KEY,
                fullname TEXT NOT NULL,
                shortname TEXT NOT NULL,
                summary TEXT,
                last_synced_at TEXT
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

            CREATE INDEX IF NOT EXISTS idx_activities_due_date ON activities(due_date);
            CREATE INDEX IF NOT EXISTS idx_outbox_status ON outbox_items(status);
            "#,
        )?;
        Ok(())
    }

    // --- Courses ---

    pub fn upsert_course(&self, course: &Course) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO courses (id, fullname, shortname, summary, last_synced_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                fullname = excluded.fullname,
                shortname = excluded.shortname,
                summary = excluded.summary,
                last_synced_at = excluded.last_synced_at;
            "#,
            params![
                course.id,
                course.fullname,
                course.shortname,
                course.summary,
                course.last_synced_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_courses(&self) -> Result<Vec<Course>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fullname, shortname, summary, last_synced_at FROM courses ORDER BY fullname ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let last_synced: Option<String> = row.get(4)?;
            Ok(Course {
                id: row.get(0)?,
                fullname: row.get(1)?,
                shortname: row.get(2)?,
                summary: row.get(3)?,
                last_synced_at: last_synced.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
            })
        })?;

        let mut courses = Vec::new();
        for r in rows {
            courses.push(r?);
        }
        Ok(courses)
    }

    pub fn get_course(&self, id: i64) -> Result<Option<Course>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fullname, shortname, summary, last_synced_at FROM courses WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            let last_synced: Option<String> = row.get(4)?;
            Ok(Course {
                id: row.get(0)?,
                fullname: row.get(1)?,
                shortname: row.get(2)?,
                summary: row.get(3)?,
                last_synced_at: last_synced.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    // --- Sections ---

    pub fn upsert_section(&self, section: &Section) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO sections (id, course_id, name, summary)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                course_id = excluded.course_id,
                name = excluded.name,
                summary = excluded.summary;
            "#,
            params![section.id, section.course_id, section.name, section.summary],
        )?;
        Ok(())
    }

    pub fn get_sections_by_course(&self, course_id: i64) -> Result<Vec<Section>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, course_id, name, summary FROM sections WHERE course_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![course_id], |row| {
            Ok(Section {
                id: row.get(0)?,
                course_id: row.get(1)?,
                name: row.get(2)?,
                summary: row.get(3)?,
                activities: Vec::new(),
            })
        })?;

        let mut sections = Vec::new();
        for r in rows {
            sections.push(r?);
        }
        Ok(sections)
    }

    // --- Activities ---

    pub fn upsert_activity(&self, activity: &Activity) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO activities (id, section_id, title, activity_type, url, due_date, is_submitted)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                section_id = excluded.section_id,
                title = excluded.title,
                activity_type = excluded.activity_type,
                url = excluded.url,
                due_date = excluded.due_date,
                is_submitted = excluded.is_submitted;
            "#,
            params![
                activity.id,
                activity.section_id,
                activity.title,
                activity.activity_type.as_str(),
                activity.url,
                activity.due_date.map(|t| t.to_rfc3339()),
                if activity.is_submitted { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn get_activities_by_section(&self, section_id: i64) -> Result<Vec<Activity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, section_id, title, activity_type, url, due_date, is_submitted FROM activities WHERE section_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![section_id], |row| {
            let type_str: String = row.get(3)?;
            let due_date: Option<String> = row.get(5)?;
            let is_sub: i64 = row.get(6)?;
            Ok(Activity {
                id: row.get(0)?,
                section_id: row.get(1)?,
                title: row.get(2)?,
                activity_type: ActivityType::from_moodle_modname(&type_str),
                url: row.get(4)?,
                due_date: due_date.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                is_submitted: is_sub == 1,
            })
        })?;

        let mut acts = Vec::new();
        for r in rows {
            acts.push(r?);
        }
        Ok(acts)
    }

    pub fn get_upcoming_deadlines(&self) -> Result<Vec<Activity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, section_id, title, activity_type, url, due_date, is_submitted 
            FROM activities 
            WHERE due_date IS NOT NULL AND is_submitted = 0
            ORDER BY due_date ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let type_str: String = row.get(3)?;
            let due_date: Option<String> = row.get(5)?;
            let is_sub: i64 = row.get(6)?;
            Ok(Activity {
                id: row.get(0)?,
                section_id: row.get(1)?,
                title: row.get(2)?,
                activity_type: ActivityType::from_moodle_modname(&type_str),
                url: row.get(4)?,
                due_date: due_date.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                is_submitted: is_sub == 1,
            })
        })?;

        let mut deadlines = Vec::new();
        for r in rows {
            deadlines.push(r?);
        }
        Ok(deadlines)
    }

    // --- Outbox Items ---

    pub fn insert_outbox_item(&self, item: &OutboxItem) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
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
        let id = conn.last_insert_rowid();
        Ok(id)
    }

    pub fn get_outbox_item(&self, id: i64) -> Result<Option<OutboxItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, assign_id, filename, file_hash, local_file_path, status,
                   retry_count, last_attempt_at, last_error, rfc3161_token, created_at, updated_at
            FROM outbox_items
            WHERE id = ?1
            "#,
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            let status_str: String = row.get(5)?;
            let last_attempt: Option<String> = row.get(7)?;
            let created_str: String = row.get(10)?;
            let updated_str: String = row.get(11)?;

            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(OutboxItem {
                id: Some(row.get(0)?),
                assign_id: row.get(1)?,
                filename: row.get(2)?,
                file_hash: row.get(3)?,
                local_file_path: row.get(4)?,
                status: status_str.parse().unwrap_or(OutboxStatus::Pending),
                retry_count: row.get(6)?,
                last_attempt_at: last_attempt.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                last_error: row.get(8)?,
                rfc3161_token: row.get(9)?,
                created_at,
                updated_at,
            })
        })?;

        if let Some(r) = rows.next() {
            Ok(Some(r?))
        } else {
            Ok(None)
        }
    }

    pub fn get_pending_outbox_items(&self) -> Result<Vec<OutboxItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, assign_id, filename, file_hash, local_file_path, status,
                   retry_count, last_attempt_at, last_error, rfc3161_token, created_at, updated_at
            FROM outbox_items
            WHERE status IN ('pending', 'failed')
            ORDER BY created_at ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(5)?;
            let last_attempt: Option<String> = row.get(7)?;
            let created_str: String = row.get(10)?;
            let updated_str: String = row.get(11)?;

            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(OutboxItem {
                id: Some(row.get(0)?),
                assign_id: row.get(1)?,
                filename: row.get(2)?,
                file_hash: row.get(3)?,
                local_file_path: row.get(4)?,
                status: status_str.parse().unwrap_or(OutboxStatus::Pending),
                retry_count: row.get(6)?,
                last_attempt_at: last_attempt.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                last_error: row.get(8)?,
                rfc3161_token: row.get(9)?,
                created_at,
                updated_at,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    pub fn update_outbox_item(&self, item: &OutboxItem) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_in_memory_db_schema() {
        let db = Database::open_in_memory().expect("in-memory db failed");
        let courses = db.get_courses().expect("failed to get courses");
        assert!(courses.is_empty());
    }

    #[test]
    fn test_file_db_schema() {
        let temp = NamedTempFile::new().expect("temp file failed");
        let db = Database::open(temp.path()).expect("file db failed");
        let courses = db.get_courses().expect("failed to get courses");
        assert!(courses.is_empty());
    }

    #[test]
    fn test_courses_crud() {
        let db = Database::open_in_memory().unwrap();
        let course = Course {
            id: 10,
            fullname: "Sistemas Operacionais".to_string(),
            shortname: "SO".to_string(),
            summary: Some("Ementa de SO".to_string()),
            last_synced_at: Some(Utc::now()),
        };

        db.upsert_course(&course).unwrap();
        let fetched = db.get_course(10).unwrap().expect("course not found");
        assert_eq!(fetched.fullname, "Sistemas Operacionais");

        // Update
        let updated_course = Course {
            id: 10,
            fullname: "Sistemas Operacionais Avançados".to_string(),
            shortname: "SO-Adv".to_string(),
            summary: None,
            last_synced_at: None,
        };
        db.upsert_course(&updated_course).unwrap();
        let fetched_updated = db.get_course(10).unwrap().unwrap();
        assert_eq!(fetched_updated.fullname, "Sistemas Operacionais Avançados");
        assert_eq!(fetched_updated.summary, None);
    }

    #[test]
    fn test_sections_and_activities() {
        let db = Database::open_in_memory().unwrap();
        let course = Course {
            id: 1,
            fullname: "Cálculo 1".to_string(),
            shortname: "CALC1".to_string(),
            summary: None,
            last_synced_at: None,
        };
        db.upsert_course(&course).unwrap();

        let section = Section {
            id: 100,
            course_id: 1,
            name: "Limites e Derivadas".to_string(),
            summary: None,
            activities: Vec::new(),
        };
        db.upsert_section(&section).unwrap();

        let activity1 = Activity {
            id: 1001,
            section_id: 100,
            title: "Lista 1 - Limites".to_string(),
            activity_type: ActivityType::Assign,
            url: Some("https://moodle/assign/1".to_string()),
            due_date: Some(Utc::now()),
            is_submitted: false,
        };
        let activity2 = Activity {
            id: 1002,
            section_id: 100,
            title: "Apostila de Limites".to_string(),
            activity_type: ActivityType::Resource,
            url: None,
            due_date: None,
            is_submitted: true,
        };

        db.upsert_activity(&activity1).unwrap();
        db.upsert_activity(&activity2).unwrap();

        let acts = db.get_activities_by_section(100).unwrap();
        assert_eq!(acts.len(), 2);

        let upcoming = db.get_upcoming_deadlines().unwrap();
        assert_eq!(upcoming.len(), 1);
        assert_eq!(upcoming[0].id, 1001);
    }

    #[test]
    fn test_outbox_items_crud() {
        let db = Database::open_in_memory().unwrap();
        let mut item = OutboxItem::new(
            55,
            "relatorio.pdf".to_string(),
            "hash_123456789".to_string(),
            "/caminho/relatorio.pdf".to_string(),
        );

        let id = db.insert_outbox_item(&item).unwrap();
        assert!(id > 0);
        item.id = Some(id);

        let fetched = db.get_outbox_item(id).unwrap().expect("item not found");
        assert_eq!(fetched.filename, "relatorio.pdf");
        assert_eq!(fetched.status, OutboxStatus::Pending);

        let pending = db.get_pending_outbox_items().unwrap();
        assert_eq!(pending.len(), 1);

        // Update item with failure and retry
        item.record_failure("Erro 500 no Moodle".to_string());
        db.update_outbox_item(&item).unwrap();

        let fetched_failed = db.get_outbox_item(id).unwrap().unwrap();
        assert_eq!(fetched_failed.status, OutboxStatus::Failed);
        assert_eq!(fetched_failed.retry_count, 1);
        assert_eq!(
            fetched_failed.last_error.as_deref(),
            Some("Erro 500 no Moodle")
        );

        // Mark delivered
        item.record_success();
        db.update_outbox_item(&item).unwrap();

        let pending_after = db.get_pending_outbox_items().unwrap();
        assert!(pending_after.is_empty());
    }
}
