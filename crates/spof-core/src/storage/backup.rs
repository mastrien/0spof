use super::Database;
use crate::error::Result;
use crate::models::{Activity, ActivityType, Course, OutboxItem, OutboxStatus, Section};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSummary {
    pub courses_imported: usize,
    pub sections_imported: usize,
    pub activities_imported: usize,
    pub outbox_items_imported: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupData {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub courses: Vec<Course>,
    pub sections: Vec<Section>,
    pub activities: Vec<Activity>,
    pub outbox_items: Vec<OutboxItem>,
}

impl Database {
    pub fn export_data(&self) -> Result<BackupData> {
        let courses = self.get_courses()?;
        let connection = self.conn.lock().unwrap();

        let mut section_stmt = connection
            .prepare("SELECT id, course_id, name, summary FROM sections ORDER BY course_id, id")?;
        let sections = section_stmt
            .query_map([], |row| {
                Ok(Section {
                    id: row.get(0)?,
                    course_id: row.get(1)?,
                    name: row.get(2)?,
                    summary: row.get(3)?,
                    activities: Vec::new(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut act_stmt = connection.prepare(
            r#"
            SELECT id, section_id, title, activity_type, url, description,
                   local_file_path, content_mime, due_date, is_submitted
            FROM activities
            ORDER BY section_id, id
            "#,
        )?;
        let activities = act_stmt
            .query_map([], |row| {
                let act_type_str: String = row.get(3)?;
                let due_date_str: Option<String> = row.get(8)?;
                let due_date = due_date_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });
                let is_sub_int: i32 = row.get(9)?;

                Ok(Activity {
                    id: row.get(0)?,
                    section_id: row.get(1)?,
                    title: row.get(2)?,
                    activity_type: ActivityType::from_moodle_modname(&act_type_str),
                    url: row.get(4)?,
                    description: row.get(5)?,
                    local_file_path: row.get(6)?,
                    content_mime: row.get(7)?,
                    due_date,
                    is_submitted: is_sub_int != 0,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut outbox_stmt = connection.prepare(
            r#"
            SELECT id, assign_id, filename, file_hash, local_file_path, status,
                   retry_count, last_attempt_at, last_error, rfc3161_token,
                   created_at, updated_at
            FROM outbox_items
            ORDER BY id
            "#,
        )?;
        let outbox_items = outbox_stmt
            .query_map([], |row| {
                let status_str: String = row.get(5)?;
                let last_attempt_str: Option<String> = row.get(7)?;
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
                    last_attempt_at: last_attempt_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    last_error: row.get(8)?,
                    rfc3161_token: row.get(9)?,
                    created_at,
                    updated_at,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(BackupData {
            version: 1,
            exported_at: Utc::now(),
            courses,
            sections,
            activities,
            outbox_items,
        })
    }

    pub fn export_data_json(&self) -> Result<String> {
        let backup = self.export_data()?;
        let json = serde_json::to_string_pretty(&backup)?;
        Ok(json)
    }

    pub fn import_data(&self, backup: &BackupData) -> Result<ImportSummary> {
        for course in &backup.courses {
            self.upsert_course(course)?;
        }
        for section in &backup.sections {
            self.upsert_section(section)?;
        }
        for activity in &backup.activities {
            self.upsert_activity(activity)?;
        }
        for item in &backup.outbox_items {
            let _ = self.insert_outbox_item(item)?;
        }

        Ok(ImportSummary {
            courses_imported: backup.courses.len(),
            sections_imported: backup.sections.len(),
            activities_imported: backup.activities.len(),
            outbox_items_imported: backup.outbox_items.len(),
        })
    }

    pub fn import_data_json(&self, json_str: &str) -> Result<ImportSummary> {
        let backup: BackupData = serde_json::from_str(json_str)?;
        self.import_data(&backup)
    }

    pub fn wipe_all_data(&self) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute_batch(
            r#"
            PRAGMA foreign_keys = OFF;
            DELETE FROM activities;
            DELETE FROM sections;
            DELETE FROM courses;
            DELETE FROM outbox_items;
            PRAGMA foreign_keys = ON;
            VACUUM;
            "#,
        )?;
        Ok(())
    }
}
