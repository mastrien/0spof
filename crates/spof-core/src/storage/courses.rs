use super::Database;
use crate::error::Result;
use crate::models::{Activity, ActivityType, Course, Section};
use chrono::{DateTime, Utc};
use rusqlite::{params, Row};

fn map_activity_row(row: &Row) -> rusqlite::Result<Activity> {
    let type_str: String = row.get(3)?;
    let due_date_str: Option<String> = row.get(8)?;
    let is_submitted_flag: i64 = row.get(9)?;
    Ok(Activity {
        id: row.get(0)?,
        section_id: row.get(1)?,
        title: row.get(2)?,
        activity_type: ActivityType::from_moodle_modname(&type_str),
        url: row.get(4)?,
        description: row.get(5)?,
        local_file_path: row.get(6)?,
        content_mime: row.get(7)?,
        due_date: due_date_str.and_then(|timestamp_str| {
            DateTime::parse_from_rfc3339(&timestamp_str)
                .ok()
                .map(|datetime| datetime.with_timezone(&Utc))
        }),
        is_submitted: is_submitted_flag == 1,
    })
}

impl Database {
    // --- Courses ---

    pub fn upsert_course(&self, course: &Course) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
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
                course
                    .last_synced_at
                    .map(|timestamp| timestamp.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_courses(&self) -> Result<Vec<Course>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT id, fullname, shortname, summary, last_synced_at FROM courses ORDER BY fullname ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let last_synced_str: Option<String> = row.get(4)?;
            Ok(Course {
                id: row.get(0)?,
                fullname: row.get(1)?,
                shortname: row.get(2)?,
                summary: row.get(3)?,
                last_synced_at: last_synced_str.and_then(|timestamp_str| {
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .ok()
                        .map(|datetime| datetime.with_timezone(&Utc))
                }),
            })
        })?;

        let mut course_list = Vec::new();
        for course_row in rows {
            course_list.push(course_row?);
        }
        Ok(course_list)
    }

    pub fn get_course(&self, course_id: i64) -> Result<Option<Course>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            "SELECT id, fullname, shortname, summary, last_synced_at FROM courses WHERE id = ?1",
        )?;
        let mut rows = statement.query_map(params![course_id], |row| {
            let last_synced_str: Option<String> = row.get(4)?;
            Ok(Course {
                id: row.get(0)?,
                fullname: row.get(1)?,
                shortname: row.get(2)?,
                summary: row.get(3)?,
                last_synced_at: last_synced_str.and_then(|timestamp_str| {
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .ok()
                        .map(|datetime| datetime.with_timezone(&Utc))
                }),
            })
        })?;

        if let Some(course_result) = rows.next() {
            Ok(Some(course_result?))
        } else {
            Ok(None)
        }
    }

    // --- Sections ---

    pub fn upsert_section(&self, section: &Section) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
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
        let mut section_list = {
            let connection = self.conn.lock().unwrap();
            let mut statement = connection.prepare(
                "SELECT id, course_id, name, summary FROM sections WHERE course_id = ?1 ORDER BY id ASC",
            )?;
            let rows = statement.query_map(params![course_id], |row| {
                Ok(Section {
                    id: row.get(0)?,
                    course_id: row.get(1)?,
                    name: row.get(2)?,
                    summary: row.get(3)?,
                    activities: Vec::new(),
                })
            })?;

            let mut list = Vec::new();
            for section_row in rows {
                list.push(section_row?);
            }
            list
        };

        for section in &mut section_list {
            section.activities = self.get_activities_by_section(section.id)?;
        }

        Ok(section_list)
    }

    // --- Activities ---

    pub fn upsert_activity(&self, activity: &Activity) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            INSERT INTO activities (
                id, section_id, title, activity_type, url, description, local_file_path, content_mime, due_date, is_submitted
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                section_id = excluded.section_id,
                title = excluded.title,
                activity_type = excluded.activity_type,
                url = excluded.url,
                description = COALESCE(excluded.description, activities.description),
                local_file_path = COALESCE(excluded.local_file_path, activities.local_file_path),
                content_mime = COALESCE(excluded.content_mime, activities.content_mime),
                due_date = COALESCE(excluded.due_date, activities.due_date),
                is_submitted = CASE WHEN excluded.is_submitted = 1 THEN 1 ELSE activities.is_submitted END;
            "#,
            params![
                activity.id,
                activity.section_id,
                activity.title,
                activity.activity_type.as_str(),
                activity.url,
                activity.description,
                activity.local_file_path,
                activity.content_mime,
                activity.due_date.map(|timestamp| timestamp.to_rfc3339()),
                if activity.is_submitted { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn update_activity_submission_details(
        &self,
        activity_id: i64,
        is_submitted: bool,
        due_date: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            UPDATE activities 
            SET is_submitted = ?1,
                due_date = COALESCE(?2, due_date)
            WHERE id = ?3
            "#,
            params![
                if is_submitted { 1 } else { 0 },
                due_date.map(|d| d.to_rfc3339()),
                activity_id,
            ],
        )?;
        Ok(())
    }

    pub fn update_activity_description_if_empty(
        &self,
        activity_id: i64,
        description: &str,
    ) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            r#"
            UPDATE activities 
            SET description = ?1
            WHERE id = ?2 AND (description IS NULL OR description = '')
            "#,
            params![description, activity_id],
        )?;
        Ok(())
    }

    pub fn update_activity_local_file(
        &self,
        activity_id: i64,
        local_file_path: &str,
        content_mime: &str,
    ) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            "UPDATE activities SET local_file_path = ?2, content_mime = ?3 WHERE id = ?1",
            params![activity_id, local_file_path, content_mime],
        )?;
        Ok(())
    }

    pub fn update_activity_url(&self, activity_id: i64, url: &str) -> Result<()> {
        let connection = self.conn.lock().unwrap();
        connection.execute(
            "UPDATE activities SET url = ?2 WHERE id = ?1",
            params![activity_id, url],
        )?;
        Ok(())
    }

    pub fn get_activity(&self, activity_id: i64) -> Result<Option<Activity>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            r#"
            SELECT id, section_id, title, activity_type, url, description, local_file_path, content_mime, due_date, is_submitted
            FROM activities WHERE id = ?1
            "#,
        )?;
        let mut rows = statement.query_map(params![activity_id], map_activity_row)?;
        if let Some(res) = rows.next() {
            Ok(Some(res?))
        } else {
            Ok(None)
        }
    }

    pub fn get_activities_by_section(&self, section_id: i64) -> Result<Vec<Activity>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            r#"
            SELECT id, section_id, title, activity_type, url, description, local_file_path, content_mime, due_date, is_submitted
            FROM activities WHERE section_id = ?1 ORDER BY id ASC
            "#,
        )?;
        let rows = statement.query_map(params![section_id], map_activity_row)?;

        let mut activity_list = Vec::new();
        for activity_row in rows {
            activity_list.push(activity_row?);
        }
        Ok(activity_list)
    }

    pub fn get_upcoming_deadlines(&self) -> Result<Vec<Activity>> {
        let connection = self.conn.lock().unwrap();
        let mut statement = connection.prepare(
            r#"
            SELECT id, section_id, title, activity_type, url, description, local_file_path, content_mime, due_date, is_submitted 
            FROM activities 
            WHERE (activity_type = 'assign' OR due_date IS NOT NULL) AND is_submitted = 0
            ORDER BY 
                CASE WHEN due_date IS NOT NULL THEN 0 ELSE 1 END,
                due_date ASC,
                id ASC
            "#,
        )?;
        let rows = statement.query_map([], map_activity_row)?;

        let mut deadline_list = Vec::new();
        for deadline_row in rows {
            deadline_list.push(deadline_row?);
        }
        Ok(deadline_list)
    }
}
